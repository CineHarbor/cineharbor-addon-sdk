//! 媒体代理 HTTP handler（native，reqwest + axum）：standalone addon 自挂转链服务。
//!
//! 独立 addon 用 `/media/vod/*` 或 `/media/live/*` 把上游 CDN 拉回来、重写 m3u8、再转发给
//! 浏览器（带 UA/Referer + CORS）。vod m3u8 含广告过滤；配置 `access_token` 后
//! `/media/vod/*` 必须带匹配的 `token` 查询参数。

use std::collections::HashMap;
use std::sync::Arc;

use axum::Router;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;

use crate::ad_filter::{AdFilterConfig, filter_m3u8};
use crate::{rewrite_live_manifest_content, rewrite_vod_manifest_content};

pub const DEFAULT_WEB_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36";

/// 每个 source 的上游请求头覆盖（UA / Referer，用于过防盗链）。
#[derive(Debug, Clone, Default)]
pub struct SourceHeaders {
    pub ua: Option<String>,
    pub referer: Option<String>,
    pub disable_ad_filter: bool,
}

/// 代理路由的共享状态。
pub struct ProxyParts {
    pub client: reqwest::Client,
    pub sources: Arc<HashMap<String, SourceHeaders>>,
    pub public_base_url: String,
    pub access_token: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct VodProxyQuery {
    pub source: Option<String>,
    pub url: Option<String>,
    pub token: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct LiveProxyQuery {
    #[serde(rename = "cineharbor-source")]
    pub source: Option<String>,
    pub url: Option<String>,
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
        if let Some(referer) = source_headers.and_then(|s| s.referer.as_deref()) {
            if let Ok(value) = HeaderValue::from_str(referer) {
                headers.insert(header::REFERER, value);
            }
        }
        headers
    }
}

pub fn vod_proxy_router(parts: Arc<ProxyParts>) -> Router {
    Router::new()
        .route("/media/vod/m3u8", get(vod_m3u8))
        .route("/media/vod/segment", get(vod_bytes))
        .route("/media/vod/key", get(vod_bytes))
        .with_state(parts)
}

pub fn live_proxy_router(parts: Arc<ProxyParts>) -> Router {
    Router::new()
        .route("/media/live/m3u8", get(live_m3u8))
        .route("/media/live/segment", get(live_bytes))
        .route("/media/live/key", get(live_bytes))
        .with_state(parts)
}

async fn vod_m3u8(
    State(parts): State<Arc<ProxyParts>>,
    Query(query): Query<VodProxyQuery>,
) -> Response {
    if let Some(response) = authorize(&parts, query.token.as_deref()) {
        return response;
    }
    let Some((source, url)) = parse_query(query.source, query.url) else {
        return bad_request("missing source or url");
    };
    match fetch_manifest(&parts, &source, &url).await {
        Ok(text) => {
            let rewritten =
                rewrite_vod_manifest_content(&text, &url, &source, &parts.public_base_url);
            let rewritten = inject_access_token(&rewritten, parts.access_token.as_deref());
            let disable_ad_filter = parts
                .sources
                .get(&source)
                .is_some_and(|headers| headers.disable_ad_filter);
            let body = if disable_ad_filter {
                rewritten
            } else {
                let filtered = filter_m3u8(&rewritten, &AdFilterConfig::default());
                if filtered.changed {
                    filtered.filtered
                } else {
                    rewritten
                }
            };
            manifest_response(body)
        }
        Err(response) => response,
    }
}

async fn vod_bytes(
    State(parts): State<Arc<ProxyParts>>,
    Query(query): Query<VodProxyQuery>,
) -> Response {
    if let Some(response) = authorize(&parts, query.token.as_deref()) {
        return response;
    }
    let Some((source, url)) = parse_query(query.source, query.url) else {
        return bad_request("missing source or url");
    };
    forward_bytes(&parts, &source, &url).await
}

async fn live_m3u8(
    State(parts): State<Arc<ProxyParts>>,
    Query(query): Query<LiveProxyQuery>,
) -> Response {
    let Some((source, url)) = parse_query(query.source, query.url) else {
        return bad_request("missing source or url");
    };
    match fetch_manifest(&parts, &source, &url).await {
        Ok(text) => {
            let rewritten =
                rewrite_live_manifest_content(&text, &url, &source, &parts.public_base_url, false);
            manifest_response(rewritten)
        }
        Err(response) => response,
    }
}

async fn live_bytes(
    State(parts): State<Arc<ProxyParts>>,
    Query(query): Query<LiveProxyQuery>,
) -> Response {
    let Some((source, url)) = parse_query(query.source, query.url) else {
        return bad_request("missing source or url");
    };
    forward_bytes(&parts, &source, &url).await
}

fn authorize(parts: &ProxyParts, provided: Option<&str>) -> Option<Response> {
    let expected = parts
        .access_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let provided = provided.map(str::trim).unwrap_or("");
    if provided == expected {
        None
    } else {
        Some((StatusCode::UNAUTHORIZED, "unauthorized").into_response())
    }
}

fn inject_access_token(content: &str, token: Option<&str>) -> String {
    let Some(token) = token.map(str::trim).filter(|value| !value.is_empty()) else {
        return content.to_string();
    };
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    serializer.append_pair("token", token);
    let suffix = format!("&{}", serializer.finish());
    content
        .split('\n')
        .map(|line| {
            if line.contains("/media/vod/") && line.contains("source=") && !line.contains("token=")
            {
                if let Some(quote_at) = line.rfind('"') {
                    let (prefix, suffix_quote) = line.split_at(quote_at);
                    format!("{prefix}{suffix}{suffix_quote}")
                } else {
                    format!("{line}{suffix}")
                }
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_query(source: Option<String>, url: Option<String>) -> Option<(String, String)> {
    let source = source
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())?;
    let url = url
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())?;
    Some((source, url))
}

fn bad_request(message: &'static str) -> Response {
    (StatusCode::BAD_REQUEST, message).into_response()
}

fn bad_gateway(message: String) -> Response {
    (StatusCode::BAD_GATEWAY, message).into_response()
}

fn manifest_response(content: String) -> Response {
    let mut response = (StatusCode::OK, content).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/vnd.apple.mpegurl"),
    );
    cors_headers(response.headers_mut());
    response
}

fn bytes_response(status: StatusCode, content_type: &str, body: Vec<u8>) -> Response {
    let mut response = (status, body).into_response();
    if let Ok(value) = HeaderValue::from_str(content_type) {
        response.headers_mut().insert(header::CONTENT_TYPE, value);
    }
    cors_headers(response.headers_mut());
    response
}

fn cors_headers(headers: &mut HeaderMap) {
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    headers.insert(
        header::ACCESS_CONTROL_EXPOSE_HEADERS,
        HeaderValue::from_static("*"),
    );
}

async fn fetch_manifest(parts: &ProxyParts, source: &str, url: &str) -> Result<String, Response> {
    let response = parts
        .client
        .get(url)
        .headers(parts.headers_for(source))
        .send()
        .await
        .map_err(|error| bad_gateway(format!("upstream fetch failed: {error}")))?;
    if !response.status().is_success() {
        return Err(bad_gateway(format!(
            "upstream status: {}",
            response.status()
        )));
    }
    response
        .text()
        .await
        .map_err(|error| bad_gateway(format!("upstream body read failed: {error}")))
}

async fn forward_bytes(parts: &ProxyParts, source: &str, url: &str) -> Response {
    let response = match parts
        .client
        .get(url)
        .headers(parts.headers_for(source))
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => return bad_gateway(format!("upstream fetch failed: {error}")),
    };
    let status = response.status();
    if !status.is_success() {
        return bad_gateway(format!("upstream status: {status}"));
    }
    match response.bytes().await {
        Ok(bytes) => bytes_response(status, "application/octet-stream", bytes.to_vec()),
        Err(error) => bad_gateway(format!("upstream body read failed: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn serve_router(router: Router) -> std::net::SocketAddr {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        addr
    }

    #[tokio::test]
    async fn vod_m3u8_proxies_and_rewrites() {
        let upstream = Router::new().route(
            "/p/index.m3u8",
            get(|| async {
                "#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"key.bin\"\n#EXTINF:4,\nseg1.ts\n"
            }),
        );
        let upstream_addr = serve_router(upstream).await;

        let parts = Arc::new(ProxyParts {
            client: reqwest::Client::new(),
            sources: Arc::new(HashMap::new()),
            public_base_url: "http://proxy.test".into(),
            access_token: None,
        });
        let proxy_addr = serve_router(vod_proxy_router(parts)).await;

        let upstream_url = format!("http://{upstream_addr}/p/index.m3u8");
        let encoded =
            url::form_urlencoded::byte_serialize(upstream_url.as_bytes()).collect::<String>();
        let response = reqwest::get(format!(
            "http://{proxy_addr}/media/vod/m3u8?source=src&url={encoded}"
        ))
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/vnd.apple.mpegurl"
        );
        let body = response.text().await.unwrap();
        assert!(body.contains("http://proxy.test/media/vod/key?source=src&url="));
        assert!(body.contains("http://proxy.test/media/vod/segment?source=src&url="));
        assert!(body.contains("seg1.ts"));
    }

    #[tokio::test]
    async fn vod_proxy_requires_token_when_configured() {
        let parts = Arc::new(ProxyParts {
            client: reqwest::Client::new(),
            sources: Arc::new(HashMap::new()),
            public_base_url: "http://proxy.test".into(),
            access_token: Some("secret".into()),
        });
        let proxy_addr = serve_router(vod_proxy_router(parts)).await;
        let denied = reqwest::get(format!(
            "http://{proxy_addr}/media/vod/segment?source=src&url=http://x/a.ts"
        ))
        .await
        .unwrap();
        assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn live_m3u8_uses_cineharbor_source_param() {
        let upstream = Router::new().route(
            "/live/master.m3u8",
            get(|| async { "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=800000\nch.m3u8\n" }),
        );
        let upstream_addr = serve_router(upstream).await;

        let parts = Arc::new(ProxyParts {
            client: reqwest::Client::new(),
            sources: Arc::new(HashMap::new()),
            public_base_url: "http://proxy.test".into(),
            access_token: None,
        });
        let proxy_addr = serve_router(live_proxy_router(parts)).await;

        let upstream_url = format!("http://{upstream_addr}/live/master.m3u8");
        let encoded =
            url::form_urlencoded::byte_serialize(upstream_url.as_bytes()).collect::<String>();
        let response = reqwest::get(format!(
            "http://{proxy_addr}/media/live/m3u8?cineharbor-source=live1&url={encoded}"
        ))
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.text().await.unwrap();
        assert!(body.contains("http://proxy.test/media/live/m3u8?cineharbor-source=live1&url="));
        assert!(body.contains("ch.m3u8"));
    }
}
