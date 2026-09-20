//! Authenticated, public-egress-only media service. Source metadata issues narrowly scoped
//! capabilities; neither an anonymous URL parameter nor a leaked child URL is an open proxy.

use std::{collections::HashMap, sync::Arc};

use axum::{
    Router,
    body::Body,
    extract::{OriginalUri, Query, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};

mod access;
mod egress;
mod token;
pub use access::{MEDIA_URL_TTL_SECONDS, MediaAuthorization};
use egress::FetchError;
pub use egress::MediaClient;
use token::authorize_manifest;

use crate::ad_filter::{AdFilterConfig, filter_m3u8};
use crate::{rewrite_live_manifest_content, rewrite_vod_manifest_content};

pub const DEFAULT_WEB_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36";
const MAX_MANIFEST_BYTES: usize = 2 * 1024 * 1024;
const MAX_REWRITTEN_MANIFEST_BYTES: usize = 16 * 1024 * 1024;
const MAX_KEY_BYTES: usize = 64 * 1024;
const MAX_RESOURCE_BYTES: usize = 128 * 1024 * 1024;

#[derive(Debug, Clone, Default)]
pub struct SourceHeaders {
    pub ua: Option<String>,
    pub referer: Option<String>,
    pub disable_ad_filter: bool,
}

pub struct ProxyParts {
    pub client: MediaClient,
    pub sources: Arc<HashMap<String, SourceHeaders>>,
    pub public_base_url: String,
    pub authorization: Arc<MediaAuthorization>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaQuery {
    source: Option<String>,
    #[serde(rename = "cineharbor-source")]
    live_source: Option<String>,
    url: Option<String>,
    expires: Option<u64>,
    sig: Option<String>,
    #[serde(rename = "allowCORS")]
    _allow_cors: Option<bool>,
}

impl ProxyParts {
    fn headers_for(&self, source: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        let source_headers = self.sources.get(source);
        let ua = source_headers
            .and_then(|s| s.ua.as_deref())
            .unwrap_or(DEFAULT_WEB_UA);
        if let Ok(value) = HeaderValue::from_str(ua) {
            headers.insert(header::USER_AGENT, value);
        }
        if let Some(referer) = source_headers.and_then(|s| s.referer.as_deref())
            && let Ok(value) = HeaderValue::from_str(referer)
        {
            headers.insert(header::REFERER, value);
        }
        // Preserve byte offsets and prevent decompression amplification, even if another
        // workspace crate enables reqwest compression features through feature unification.
        headers.insert(
            header::ACCEPT_ENCODING,
            HeaderValue::from_static("identity"),
        );
        headers
    }
}

pub fn vod_proxy_router(parts: Arc<ProxyParts>) -> Router {
    Router::new()
        .route("/media/vod/m3u8", get(vod_m3u8).options(preflight))
        .route("/media/vod/segment", get(vod_bytes).options(preflight))
        .route("/media/vod/key", get(vod_bytes).options(preflight))
        .with_state(parts)
}

pub fn live_proxy_router(parts: Arc<ProxyParts>) -> Router {
    Router::new()
        .route("/media/live/m3u8", get(live_m3u8).options(preflight))
        .route("/media/live/segment", get(live_bytes).options(preflight))
        .route("/media/live/key", get(live_bytes).options(preflight))
        .with_state(parts)
}

async fn vod_m3u8(
    State(parts): State<Arc<ProxyParts>>,
    Query(query): Query<MediaQuery>,
    method: Method,
) -> Response {
    proxy_manifest(&parts, query, "vod", method).await
}

async fn live_m3u8(
    State(parts): State<Arc<ProxyParts>>,
    Query(query): Query<MediaQuery>,
    method: Method,
) -> Response {
    proxy_manifest(&parts, query, "live", method).await
}

async fn vod_bytes(
    State(parts): State<Arc<ProxyParts>>,
    Query(query): Query<MediaQuery>,
    OriginalUri(uri): OriginalUri,
    method: Method,
    headers: HeaderMap,
) -> Response {
    proxy_bytes(&parts, query, "vod", uri.path(), method, headers).await
}

async fn live_bytes(
    State(parts): State<Arc<ProxyParts>>,
    Query(query): Query<MediaQuery>,
    OriginalUri(uri): OriginalUri,
    method: Method,
    headers: HeaderMap,
) -> Response {
    proxy_bytes(&parts, query, "live", uri.path(), method, headers).await
}

fn authorize(
    parts: &ProxyParts,
    query: MediaQuery,
    kind: &str,
    asset: &str,
) -> Option<(String, String)> {
    let source = match kind {
        "vod" if query.live_source.is_none() => query.source,
        "live" if query.source.is_none() => query.live_source,
        _ => None,
    };
    let (Some(source), Some(url), Some(expires), Some(sig)) =
        (source, query.url, query.expires, query.sig)
    else {
        return None;
    };
    if source.is_empty()
        || source.len() > 256
        || url.is_empty()
        || url.len() > 8192
        || !parts.sources.contains_key(&source)
        || !parts.authorization.verify(
            &parts.public_base_url,
            &format!("{kind}/{asset}"),
            &source,
            &url,
            expires,
            &sig,
        )
    {
        return None;
    }
    Some((source, url))
}

async fn proxy_manifest(
    parts: &ProxyParts,
    query: MediaQuery,
    kind: &str,
    method: Method,
) -> Response {
    let (source, url) = match authorize(parts, query, kind, "m3u8") {
        Some(value) => value,
        None => return failure(StatusCode::UNAUTHORIZED, "unauthorized"),
    };
    if method == Method::HEAD {
        return match parts
            .client
            .send(Method::HEAD, &url, parts.headers_for(&source))
            .await
        {
            Ok(upstream) if upstream.response.status() == StatusCode::OK => {
                manifest_response(String::new(), false)
            }
            Ok(_) => failure(StatusCode::BAD_GATEWAY, "upstream status rejected"),
            Err(error) => fetch_error(error),
        };
    }
    let (text, final_url) = match fetch_manifest(parts, &source, &url).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // Bound expansion before allocating rewritten URLs/signatures for hostile playlists.
    let resource_bound = text
        .lines()
        .count()
        .saturating_add(text.matches("URI=").count());
    let per_resource = final_url
        .len()
        .saturating_add(source.len())
        .saturating_mul(3)
        .saturating_add(parts.public_base_url.len())
        .saturating_add(256);
    if resource_bound > 20_000
        || text
            .len()
            .saturating_mul(3)
            .saturating_add(resource_bound.saturating_mul(per_resource))
            > MAX_REWRITTEN_MANIFEST_BYTES
    {
        return failure(StatusCode::BAD_GATEWAY, "manifest expansion exceeds limit");
    }
    let rewritten = if kind == "vod" {
        let rewritten =
            rewrite_vod_manifest_content(&text, &final_url, &source, &parts.public_base_url);
        if parts
            .sources
            .get(&source)
            .is_some_and(|headers| headers.disable_ad_filter)
        {
            rewritten
        } else {
            filter_m3u8(&rewritten, &AdFilterConfig::default()).filtered
        }
    } else {
        rewrite_live_manifest_content(&text, &final_url, &source, &parts.public_base_url, false)
    };
    let body = authorize_manifest(&rewritten, &parts.authorization, &parts.public_base_url);
    if body.len() > MAX_REWRITTEN_MANIFEST_BYTES {
        return failure(StatusCode::BAD_GATEWAY, "manifest expansion exceeds limit");
    }
    manifest_response(body, true)
}

async fn proxy_bytes(
    parts: &ProxyParts,
    query: MediaQuery,
    kind: &str,
    path: &str,
    method: Method,
    headers: HeaderMap,
) -> Response {
    let asset = if path.ends_with("/key") {
        "key"
    } else {
        "segment"
    };
    let (source, url) = match authorize(parts, query, kind, asset) {
        Some(value) => value,
        None => return failure(StatusCode::UNAUTHORIZED, "unauthorized"),
    };
    let limit = if asset == "key" {
        MAX_KEY_BYTES
    } else {
        MAX_RESOURCE_BYTES
    };
    forward_bytes(parts, &source, &url, method, &headers, limit).await
}

fn failure(status: StatusCode, message: &'static str) -> Response {
    let mut response = (status, message).into_response();
    response_headers(response.headers_mut());
    response
}

fn fetch_error(error: FetchError) -> Response {
    match error {
        FetchError::Forbidden => failure(StatusCode::FORBIDDEN, "upstream destination denied"),
        FetchError::Unavailable => {
            failure(StatusCode::SERVICE_UNAVAILABLE, "media capacity exceeded")
        }
        FetchError::Upstream => failure(StatusCode::BAD_GATEWAY, "upstream fetch failed"),
    }
}

fn manifest_response(content: String, with_length: bool) -> Response {
    let len = content.len();
    let mut response = if with_length {
        (StatusCode::OK, content).into_response()
    } else {
        Response::new(Body::from_stream(futures_util::stream::empty::<
            Result<axum::body::Bytes, std::io::Error>,
        >()))
    };
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/vnd.apple.mpegurl"),
    );
    if with_length {
        response
            .headers_mut()
            .insert(header::CONTENT_LENGTH, HeaderValue::from(len));
    }
    response_headers(response.headers_mut());
    response
}

fn response_headers(headers: &mut HeaderMap) {
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    headers.insert(
        header::ACCESS_CONTROL_EXPOSE_HEADERS,
        HeaderValue::from_static(
            "Content-Length, Content-Range, Accept-Ranges, ETag, Last-Modified",
        ),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
}

async fn preflight() -> Response {
    let mut response = StatusCode::NO_CONTENT.into_response();
    response_headers(response.headers_mut());
    response.headers_mut().insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, HEAD, OPTIONS"),
    );
    response.headers_mut().insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("Range, If-Range"),
    );
    response
}

async fn fetch_manifest(
    parts: &ProxyParts,
    source: &str,
    url: &str,
) -> Result<(String, String), Box<Response>> {
    let mut upstream = parts
        .client
        .send(Method::GET, url, parts.headers_for(source))
        .await
        .map_err(|error| Box::new(fetch_error(error)))?;
    if upstream.response.status() != StatusCode::OK
        || !identity_encoding(upstream.response.headers())
    {
        return Err(Box::new(failure(
            StatusCode::BAD_GATEWAY,
            "upstream manifest rejected",
        )));
    }
    let final_url = upstream.response.url().to_string();
    let bytes = read_bounded(&mut upstream.response, MAX_MANIFEST_BYTES)
        .await
        .map_err(|message| Box::new(failure(StatusCode::BAD_GATEWAY, message)))?;
    let text = String::from_utf8(bytes)
        .map_err(|_| Box::new(failure(StatusCode::BAD_GATEWAY, "invalid UTF-8 manifest")))?;
    if !text.trim_start_matches('\u{feff}').starts_with("#EXTM3U") {
        return Err(Box::new(failure(
            StatusCode::BAD_GATEWAY,
            "invalid HLS manifest",
        )));
    }
    Ok((text.trim_start_matches('\u{feff}').to_string(), final_url))
}

async fn read_bounded(
    response: &mut reqwest::Response,
    limit: usize,
) -> Result<Vec<u8>, &'static str> {
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err("upstream body exceeds limit");
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "upstream body read failed")?
    {
        if chunk.len() > limit.saturating_sub(body.len()) {
            return Err("upstream body exceeds limit");
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn identity_encoding(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_ENCODING)
        .is_none_or(|value| value == "identity")
}

fn valid_range(value: &str) -> bool {
    if value.len() > 128 {
        return false;
    }
    let Some((start, end)) = value
        .strip_prefix("bytes=")
        .and_then(|value| value.split_once('-'))
    else {
        return false;
    };
    let number = |s: &str| {
        (!s.is_empty() && s.bytes().all(|c| c.is_ascii_digit()))
            .then(|| s.parse::<u64>().ok())
            .flatten()
    };
    match (number(start), number(end)) {
        (None, Some(last)) => start.is_empty() && last > 0,
        (Some(_), None) => end.is_empty(),
        (Some(first), Some(last)) => first <= last,
        _ => false,
    }
}

fn range_extent(value: &str, status: StatusCode) -> Option<u64> {
    let (range, total) = value.strip_prefix("bytes ")?.split_once('/')?;
    let number = |s: &str| {
        (!s.is_empty() && s.bytes().all(|byte| byte.is_ascii_digit()))
            .then(|| s.parse::<u64>().ok())
            .flatten()
    };
    if status == StatusCode::RANGE_NOT_SATISFIABLE {
        return (range == "*").then(|| number(total)).flatten().map(|_| 0);
    }
    let (first, last) = range.split_once('-')?;
    let (first, last) = (number(first)?, number(last)?);
    if last < first || (total != "*" && number(total)? <= last) {
        return None;
    }
    last.checked_sub(first)?.checked_add(1)
}

async fn forward_bytes(
    parts: &ProxyParts,
    source: &str,
    url: &str,
    method: Method,
    incoming: &HeaderMap,
    limit: usize,
) -> Response {
    let mut headers = parts.headers_for(source);
    if incoming.get_all(header::RANGE).iter().count() > 1 {
        return failure(StatusCode::BAD_REQUEST, "invalid range");
    }
    if let Some(range) = incoming.get(header::RANGE) {
        if !range.to_str().is_ok_and(valid_range) {
            return failure(StatusCode::BAD_REQUEST, "invalid range");
        }
        headers.insert(header::RANGE, range.clone());
        if let Some(value) = incoming.get(header::IF_RANGE) {
            if value.len() > 256 {
                return failure(StatusCode::BAD_REQUEST, "invalid if-range");
            }
            headers.insert(header::IF_RANGE, value.clone());
        }
    }
    let upstream = match parts.client.send(method.clone(), url, headers).await {
        Ok(value) => value,
        Err(error) => return fetch_error(error),
    };
    let status = upstream.response.status();
    if !matches!(
        status,
        StatusCode::OK | StatusCode::PARTIAL_CONTENT | StatusCode::RANGE_NOT_SATISFIABLE
    ) || !identity_encoding(upstream.response.headers())
    {
        return failure(StatusCode::BAD_GATEWAY, "upstream status rejected");
    }
    let expected_extent =
        if status == StatusCode::PARTIAL_CONTENT || status == StatusCode::RANGE_NOT_SATISFIABLE {
            let extent = upstream
                .response
                .headers()
                .get(header::CONTENT_RANGE)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| range_extent(value, status));
            let Some(extent) = extent else {
                return failure(StatusCode::BAD_GATEWAY, "invalid upstream range");
            };
            if status == StatusCode::PARTIAL_CONTENT
                && upstream
                    .response
                    .content_length()
                    .is_some_and(|length| length != extent)
            {
                return failure(
                    StatusCode::BAD_GATEWAY,
                    "inconsistent upstream range length",
                );
            }
            Some(extent)
        } else {
            None
        };
    let mut response = Response::new(Body::empty());
    *response.status_mut() = status;
    for name in [
        header::ACCEPT_RANGES,
        header::CONTENT_RANGE,
        header::ETAG,
        header::LAST_MODIFIED,
    ] {
        if let Some(value) = upstream.response.headers().get(&name) {
            response.headers_mut().insert(name, value.clone());
        }
    }
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    response_headers(response.headers_mut());
    if status == StatusCode::RANGE_NOT_SATISFIABLE {
        return response;
    }
    if method == Method::HEAD {
        if let Some(value) = upstream.response.headers().get(header::CONTENT_LENGTH) {
            response
                .headers_mut()
                .insert(header::CONTENT_LENGTH, value.clone());
        }
        return response;
    }
    if upstream
        .response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return failure(StatusCode::BAD_GATEWAY, "upstream body exceeds limit");
    }
    if let Some(value) = upstream.response.headers().get(header::CONTENT_LENGTH) {
        response
            .headers_mut()
            .insert(header::CONTENT_LENGTH, value.clone());
    }
    if expected_extent.is_some_and(|extent| extent > limit as u64) {
        return failure(StatusCode::BAD_GATEWAY, "upstream body exceeds limit");
    }
    let limit = expected_extent.map_or(limit, |extent| limit.min(extent as usize));
    // Backpressure and cancellation follow the response body. The semaphore permit stays
    // alive until the stream is consumed or dropped; no detached unbounded producer exists.
    let stream = futures_util::stream::try_unfold(
        (upstream, 0usize),
        move |(mut upstream, total)| async move {
            match upstream.response.chunk().await {
                Ok(Some(chunk)) if chunk.len() <= limit.saturating_sub(total) => {
                    let size = chunk.len();
                    Ok(Some((chunk, (upstream, total + size))))
                }
                Ok(None) if expected_extent.is_none_or(|extent| extent == total as u64) => Ok(None),
                _ => Err(std::io::Error::other("upstream media stream terminated")),
            }
        },
    );
    *response.body_mut() = Body::from_stream(stream);
    response
}

#[cfg(test)]
mod tests;
