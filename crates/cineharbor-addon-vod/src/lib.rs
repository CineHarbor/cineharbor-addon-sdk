//! vod 抓取 addon（standalone）：CustomAPI 视频站聚合搜索/详情/播单（catalog/meta/stream）。
//!
//! P2 的 vod 独立化（ADR-0007）：网络层 reqwest 抓取 + 配置驱动 multi-site；纯解析复用
//! `cineharbor-api`。与 local-service 内置 `ContentSearch`/`ContentDetail` 对齐（strangler
//! 并行）。媒体代理：配置 `public_base_url` 后 stream/meta 播单 url 经 `cineharbor-media`
//! 转链到自身 `/media/vod/{m3u8,segment,key}`（转链服务见 `main.rs`）；未配置则直链 url。

use std::time::Duration;

use async_trait::async_trait;
use cineharbor_addon_protocol::{
    Catalog, CatalogResponse, ContentType, Manifest, MetaDetail, MetaResponse, MetaPreview,
    Resource, Stream, StreamsResponse, Video,
};
use cineharbor_addon_sdk::addon::{Addon, CatalogRequest};
use cineharbor_api::{self as api, ApiSite, SearchResult};
use tokio::task::JoinSet;

const DEFAULT_WEB_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36";
const DEFAULT_SEARCH_TIMEOUT_MS: u64 = 8_000;
const DEFAULT_DETAIL_TIMEOUT_MS: u64 = 10_000;
const ID_PREFIX: &str = "vod:";
const SEARCH_PAGE_SIZE: usize = 50;

#[derive(Debug, thiserror::Error)]
pub enum VodError {
    #[error("http 请求失败: {0}")]
    Http(#[from] reqwest::Error),
    #[error("响应解析失败: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Other(String),
}

async fn parse_json(response: reqwest::Response) -> Result<serde_json::Value, VodError> {
    let text = response.text().await?;
    Ok(serde_json::from_str(&text)?)
}

/// addon 配置：多视频站 + 搜索翻页上限。
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct VodConfig {
    pub sites: Vec<ApiSite>,
    pub max_search_pages: usize,
    /// 客户端访问本 addon 的 base url；配置后 stream/meta 播单 url 经媒体代理转链。
    pub public_base_url: Option<String>,
}

impl Default for VodConfig {
    fn default() -> Self {
        Self {
            sites: Vec::new(),
            max_search_pages: 3,
            public_base_url: None,
        }
    }
}

impl VodConfig {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    fn enabled_sites(&self) -> impl Iterator<Item = &ApiSite> {
        self.sites.iter().filter(|site| !site.disabled)
    }
}

pub struct VodAddon {
    http: reqwest::Client,
    config: VodConfig,
}

impl VodAddon {
    pub fn new(config: VodConfig) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(DEFAULT_WEB_UA)
            .timeout(Duration::from_secs(10))
            .build()
            .expect("build vod http client");
        Self { http, config }
    }

    pub fn config(&self) -> &VodConfig {
        &self.config
    }
}

// —— ID 编解码 ——

fn vod_id(source: &str, vid: &str) -> String {
    format!("{ID_PREFIX}{source}:{vid}")
}

fn split_vod_id(id: &str) -> Option<(String, String)> {
    let rest = id.strip_prefix(ID_PREFIX)?;
    let (source, vid) = rest.split_once(':')?;
    Some((source.to_string(), vid.to_string()))
}

fn content_type_for(result: &SearchResult) -> ContentType {
    let type_name = result.type_name.as_deref().unwrap_or_default();
    if type_name.contains('剧') || type_name.contains("series") || type_name.contains("tv") {
        ContentType::Series
    } else {
        ContentType::Movie
    }
}

fn stream_url(public_base: Option<&str>, source: &str, url: &str) -> String {
    match public_base {
        Some(base) => cineharbor_media::build_vod_proxy_m3u8_url(base, source, url),
        None => url.to_string(),
    }
}

// —— 网络层（reqwest，与 local-service 对齐）——

fn build_downstream_headers(api_site: &ApiSite, default_user_agent: &str) -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();

    if let Ok(value) =
        reqwest::header::HeaderValue::from_str(api_site.ua.as_deref().unwrap_or(default_user_agent))
    {
        headers.insert(reqwest::header::USER_AGENT, value);
    }

    if let Some(referer) = api_site.referer.as_deref() {
        if let Ok(value) = reqwest::header::HeaderValue::from_str(referer) {
            headers.insert(reqwest::header::REFERER, value);
        }
    }

    headers
}

pub async fn search_site(
    client: &reqwest::Client,
    api_site: &ApiSite,
    query: &str,
    max_search_pages: usize,
) -> Result<Vec<SearchResult>, VodError> {
    let first_page_url = api::build_collection_api_url(&api_site.api, &[("ac", "videolist"), ("wd", query)])
        .map_err(VodError::Other)?;
    let first_response = client
        .get(&first_page_url)
        .headers(build_downstream_headers(api_site, DEFAULT_WEB_UA))
        .timeout(Duration::from_millis(DEFAULT_SEARCH_TIMEOUT_MS))
        .send()
        .await?;

    if !first_response.status().is_success() {
        return Ok(Vec::new());
    }

    let first_payload = parse_json(first_response).await?;
    let mut results = api::parse_search_payload(&first_payload, api_site);
    let total_pages = api::parse_usize(first_payload.get("pagecount")).unwrap_or(1);
    let pages_to_fetch = total_pages
        .saturating_sub(1)
        .min(max_search_pages.saturating_sub(1));

    for page_number in 2..=(pages_to_fetch + 1) {
        let page_url = api::build_collection_api_url(
            &api_site.api,
            &[
                ("ac", "videolist"),
                ("wd", query),
                ("pg", page_number.to_string().as_str()),
            ],
        )
        .map_err(VodError::Other)?;

        let response = client
            .get(page_url)
            .headers(build_downstream_headers(api_site, DEFAULT_WEB_UA))
            .timeout(Duration::from_millis(DEFAULT_SEARCH_TIMEOUT_MS))
            .send()
            .await?;

        if !response.status().is_success() {
            continue;
        }

        let payload = parse_json(response).await?;
        results.extend(api::parse_search_payload(&payload, api_site));
    }

    Ok(results)
}

pub async fn search_all_sites(
    client: &reqwest::Client,
    config: &VodConfig,
    query: &str,
) -> Vec<SearchResult> {
    let mut tasks = JoinSet::new();

    for api_site in config.enabled_sites() {
        let client = client.clone();
        let api_site = api_site.clone();
        let query = query.to_string();
        let max_search_pages = config.max_search_pages;

        tasks.spawn(async move {
            search_site(&client, &api_site, &query, max_search_pages)
                .await
                .unwrap_or_default()
        });
    }

    let mut results = Vec::new();
    while let Some(item) = tasks.join_next().await {
        if let Ok(items) = item {
            results.extend(items);
        }
    }
    // 确定性排序：JoinSet 汇合顺序不保证，跨 `skip` 分页需要稳定次序（相关性排序/去重归聚合侧）。
    results.sort_by(|a, b| a.source.cmp(&b.source).then_with(|| a.id.cmp(&b.id)));
    results
}

pub async fn fetch_content_detail(
    client: &reqwest::Client,
    api_site: &ApiSite,
    id: &str,
) -> Result<SearchResult, VodError> {
    if api::has_custom_detail_url(api_site) {
        fetch_custom_detail(client, api_site, id).await
    } else {
        fetch_json_detail(client, api_site, id).await
    }
}

async fn fetch_json_detail(
    client: &reqwest::Client,
    api_site: &ApiSite,
    id: &str,
) -> Result<SearchResult, VodError> {
    let detail_url = api::build_collection_api_url(&api_site.api, &[("ac", "videolist"), ("ids", id)])
        .map_err(VodError::Other)?;
    let response = client
        .get(detail_url)
        .headers(build_downstream_headers(api_site, DEFAULT_WEB_UA))
        .timeout(Duration::from_millis(DEFAULT_DETAIL_TIMEOUT_MS))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(VodError::Other(format!("详情请求失败: {}", response.status())));
    }

    let payload = parse_json(response).await?;
    api::parse_detail_payload(&payload, api_site, id)
        .ok_or_else(|| VodError::Other("获取到的详情内容无效".to_string()))
}

async fn fetch_custom_detail(
    client: &reqwest::Client,
    api_site: &ApiSite,
    id: &str,
) -> Result<SearchResult, VodError> {
    let detail_base = api_site
        .detail
        .as_deref()
        .ok_or_else(|| VodError::Other("detail 配置缺失".to_string()))?;
    let detail_url = format!(
        "{}/index.php/vod/detail/id/{}.html",
        detail_base.trim_end_matches('/'),
        id
    );
    let response = client
        .get(detail_url)
        .headers(build_downstream_headers(api_site, DEFAULT_WEB_UA))
        .timeout(Duration::from_millis(DEFAULT_DETAIL_TIMEOUT_MS))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(VodError::Other(format!("详情页请求失败: {}", response.status())));
    }

    let html = response.text().await?;
    Ok(api::parse_custom_detail_html(&html, api_site, id))
}

// —— Addon 实现 ——

#[async_trait]
impl Addon for VodAddon {
    async fn manifest(&self) -> Manifest {
        Manifest {
            id: "cineharbor.vod".into(),
            version: "0.1.0".into(),
            name: "CineHarbor Vod".into(),
            description: Some("CustomAPI 视频站聚合 addon（catalog/meta/stream，standalone）".into()),
            resources: vec![Resource::Catalog, Resource::Meta, Resource::Stream],
            types: vec![ContentType::Movie, ContentType::Series],
            catalogs: vec![
                Catalog {
                    r#type: ContentType::Movie,
                    id: "search".into(),
                    name: "搜索".into(),
                    extra: vec![],
                    extra_supported: vec!["search".into()],
                },
                Catalog {
                    r#type: ContentType::Series,
                    id: "search".into(),
                    name: "搜索".into(),
                    extra: vec![],
                    extra_supported: vec!["search".into()],
                },
            ],
            id_prefixes: Some(vec!["vod".into()]),
            icon: None,
            logo: None,
            background: None,
            behavior_hints: None,
        }
    }

    async fn catalog(&self, req: CatalogRequest) -> CatalogResponse {
        let Some((name, query)) = req.extra else {
            return CatalogResponse::default();
        };
        if name != "search" || query.trim().is_empty() {
            return CatalogResponse::default();
        }

        let results = search_all_sites(&self.http, &self.config, query.trim()).await;
        // Stremio 协议 `skip` 分页：窗口受 max_search_pages × 站点数 上界约束（每站点最多 max_search_pages
        // 页已抓取），远端深分页需上游 page-offset 支持——当前以截断窗口内的 skip 切片实现。
        let skip = req.skip.unwrap_or(0) as usize;
        let metas = results
            .into_iter()
            .skip(skip)
            .take(SEARCH_PAGE_SIZE)
            .map(|result| {
                let ty = content_type_for(&result);
                let preview = MetaPreview {
                    poster: (!result.poster.is_empty()).then_some(result.poster),
                    year: (result.year != "unknown").then_some(result.year),
                    description: result.desc,
                    ..MetaPreview::new(vod_id(&result.source, &result.id), ty, &result.title)
                };
                preview
            })
            .collect();
        CatalogResponse { metas }
    }

    async fn meta(&self, _ty: ContentType, id: &str) -> Option<MetaResponse> {
        let (source, vid) = split_vod_id(id)?;
        let api_site = self
            .config
            .sites
            .iter()
            .find(|site| site.key == source && !site.disabled)?;
        let detail = fetch_content_detail(&self.http, api_site, &vid).await.ok()?;
        // 类型由抓取结果 type_name 自证（详情请求不携带 movie/series 语义），而非信任路径 ty。
        let actual_ty = content_type_for(&detail);

        let mut meta = MetaDetail::new(id.to_string(), actual_ty, detail.title.clone());
        meta.description = detail.desc.clone();
        meta.poster = (!detail.poster.is_empty()).then_some(detail.poster.clone());
        meta.year = (detail.year != "unknown").then_some(detail.year.clone());
        meta.genres = detail.class.clone().into_iter().collect();
        meta.videos = detail
            .episodes
            .iter()
            .zip(detail.episodes_titles.iter())
            .enumerate()
            .map(|(index, (url, title))| Video {
                id: format!("{}:{}", id, index + 1),
                name: title.clone(),
                episode: Some((index + 1).to_string()),
                stream: Some(Stream {
                    name: Some(title.clone()),
                    url: Some(stream_url(
                        self.config.public_base_url.as_deref(),
                        &detail.source,
                        url,
                    )),
                    ..Stream::default()
                }),
                ..Video::default()
            })
            .collect();

        Some(MetaResponse { meta })
    }

    async fn streams(&self, _ty: ContentType, id: &str) -> StreamsResponse {
        let Some((source, vid)) = split_vod_id(id) else {
            return StreamsResponse::default();
        };
        let Some(api_site) = self
            .config
            .sites
            .iter()
            .find(|site| site.key == source && !site.disabled)
        else {
            return StreamsResponse::default();
        };
        let Ok(detail) = fetch_content_detail(&self.http, api_site, &vid).await else {
            return StreamsResponse::default();
        };

        let source = detail.source.clone();
        let streams = detail
            .episodes
            .into_iter()
            .zip(detail.episodes_titles.into_iter())
            .enumerate()
            .map(|(index, (url, title))| Stream {
                name: Some(title),
                title: Some(format!("第{}集", index + 1)),
                url: Some(stream_url(
                    self.config.public_base_url.as_deref(),
                    &source,
                    &url,
                )),
                ..Stream::default()
            })
            .collect();
        StreamsResponse { streams }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result_with_type(type_name: &str) -> SearchResult {
        SearchResult {
            id: "1".into(),
            title: "T".into(),
            poster: String::new(),
            episodes: vec!["http://e.test/1.m3u8".into()],
            episodes_titles: vec!["1".into()],
            source: "demo".into(),
            source_name: "Demo".into(),
            class: None,
            year: "2024".into(),
            desc: None,
            type_name: Some(type_name.into()),
            douban_id: None,
        }
    }

    #[test]
    fn vod_id_roundtrip() {
        let id = "vod:demo:123";
        assert_eq!(split_vod_id(id), Some(("demo".into(), "123".into())));
        assert_eq!(split_vod_id("meta:demo:123"), None);
        assert_eq!(split_vod_id("vod:no-colon"), None);
    }

    #[test]
    fn infers_content_type() {
        assert_eq!(content_type_for(&result_with_type("电影")), ContentType::Movie);
        assert_eq!(content_type_for(&result_with_type("电视剧")), ContentType::Series);
    }

    #[test]
    fn proxies_stream_url_when_configured() {
        assert_eq!(stream_url(None, "s", "http://x/a.m3u8"), "http://x/a.m3u8");
        let proxied = stream_url(Some("http://addon"), "s", "http://x/a.m3u8");
        assert!(proxied.starts_with("http://addon/media/vod/m3u8?source=s&url="));
        assert!(proxied.contains("http%3A%2F%2Fx%2Fa.m3u8"));
    }

    #[test]
    fn parses_config_json() {
        let config = VodConfig::from_json(
            r#"{"max_search_pages": 5, "sites": [{"key":"demo","name":"D","api":"http://a.test/x","disabled":false}]}"#,
        )
        .unwrap();
        assert_eq!(config.max_search_pages, 5);
        assert_eq!(config.sites.len(), 1);
        assert_eq!(config.sites[0].key, "demo");
        assert_eq!(config.sites[0].detail, None);
        assert!(!config.sites[0].disable_ad_filter);
    }

    #[test]
    fn disabled_sites_are_filtered() {
        let config = VodConfig::from_json(
            r#"{"sites":[{"key":"a","name":"A","api":"http://a/x"},{"key":"b","name":"B","api":"http://b/x","disabled":true}]}"#,
        )
        .unwrap();
        let enabled: Vec<_> = config.enabled_sites().map(|site| site.key.as_str()).collect();
        assert_eq!(enabled, vec!["a"]);
    }
}