//! douban（豆瓣）搜索 catalog addon（standalone，自抓取，无 local-service 依赖）。
//!
//! P2 的第一块「抓取外置 remote addon」：把豆瓣搜索（`search.douban.com` 的
//! `window.__DATA__` HTML 抓取 + 条目映射）做成可独立运行的 Stremio 兼容 addon。
//! 抓取与映射逻辑对齐 local-service 内置 `BuiltinDoubanAddon`（strangler 并行阶段，
//! 后续由本 crate 取代内置版）。

use std::sync::OnceLock;
use std::time::Duration;

use async_trait::async_trait;
use cineharbor_addon_protocol::{
    Catalog, CatalogResponse, ContentType, Manifest, MetaPreview, MetaResponse, Resource,
    StreamsResponse,
};
use cineharbor_addon_sdk::addon::{Addon, CatalogRequest};
use serde::Deserialize;
use url::Url;

const SEARCH_BASE_URL: &str = "https://search.douban.com";
const ID_PREFIX: &str = "douban:";
const TIMEOUT: Duration = Duration::from_secs(10);
const USER_AGENT: &str = "Mozilla/5.0 (CineHarbor douban addon)";

#[derive(Debug, thiserror::Error)]
pub enum DoubanAddonError {
    #[error("HTTP 请求失败: {0}")]
    Http(#[from] reqwest::Error),
    #[error("搜索数据解析失败: {0}")]
    Parse(String),
}

/// 抓取后的最小条目（目录项）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoubanPreview {
    pub id: String,
    pub title: String,
    pub poster: String,
    pub year: String,
    pub play_type: &'static str,
    pub rating: Option<String>,
}

pub struct DoubanAddon {
    http: reqwest::Client,
    search_base_url: String,
}

impl Default for DoubanAddon {
    fn default() -> Self {
        Self::new()
    }
}

impl DoubanAddon {
    pub fn new() -> Self {
        Self::with_search_base_url(SEARCH_BASE_URL)
    }

    /// 可配搜索基址，供 hermetic E2E 指向本地 mock 或自托管镜像（默认豆瓣官方搜索）。
    pub fn with_search_base_url(base: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(TIMEOUT)
            .build()
            .expect("build douban http client");
        Self {
            http,
            search_base_url: base.into(),
        }
    }

    /// 豆瓣搜索：返回经主体过滤 + 映射后的条目。
    pub async fn search(&self, query: &str, start: usize) -> Result<Vec<DoubanPreview>, DoubanAddonError> {
        let url = build_search_url(&self.search_base_url, query, start);
        let html = self
            .http
            .get(url)
            .header("Referer", self.search_base_url.as_str())
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let data = extract_douban_search_data(&html).map_err(DoubanAddonError::Parse)?;
        Ok(data
            .items
            .iter()
            .filter(|item| is_douban_search_subject_item(item))
            .map(map_douban_search_item)
            .collect())
    }
}

fn build_search_url(base: &str, query: &str, start: usize) -> String {
    let mut url = Url::parse(base).expect("douban search base url is valid");
    url.set_path("/movie/subject_search");
    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("search_text", query);
        pairs.append_pair("cat", "1002");
        if start > 0 {
            pairs.append_pair("start", &start.to_string());
        }
    }
    url.to_string()
}

fn content_type_for(play_type: &'static str) -> ContentType {
    match play_type {
        "tv" => ContentType::Series,
        _ => ContentType::Movie,
    }
}

// —— 豆瓣搜索页结构（`window.__DATA__`）——

#[derive(Debug, Deserialize)]
struct DoubanSearchPageLabel {
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DoubanSearchPageRating {
    #[serde(default)]
    value: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct DoubanSearchPageItem {
    #[serde(default)]
    tpl_name: Option<String>,
    #[serde(default)]
    id: Option<i64>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    cover_url: Option<String>,
    #[serde(default)]
    labels: Vec<DoubanSearchPageLabel>,
    #[serde(default)]
    rating: Option<DoubanSearchPageRating>,
}

#[derive(Debug, Deserialize)]
struct DoubanSearchPageData {
    #[serde(default)]
    items: Vec<DoubanSearchPageItem>,
}

// —— 解析 + 映射（纯函数，可离线单测）——

fn douban_search_data_regex() -> &'static regex::Regex {
    static REGEX: OnceLock<regex::Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        regex::Regex::new(r#"(?s)window\.__DATA__\s*=\s*(\{.*?\})\s*;"#)
            .expect("valid douban search data regex")
    })
}

fn douban_search_year_suffix_regex() -> &'static regex::Regex {
    static REGEX: OnceLock<regex::Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        regex::Regex::new(r#"\s*[（(]\d{4}[)）]\s*$"#).expect("valid douban search year suffix regex")
    })
}

fn douban_search_year_regex() -> &'static regex::Regex {
    static REGEX: OnceLock<regex::Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        regex::Regex::new(r#"[（(](\d{4})[)）]\s*$"#).expect("valid douban search year regex")
    })
}

/// 从 HTML 抽出 `window.__DATA__`。
fn extract_douban_search_data(html: &str) -> Result<DoubanSearchPageData, String> {
    let payload = douban_search_data_regex()
        .captures(html)
        .and_then(|captures| captures.get(1))
        .map(|value| value.as_str())
        .ok_or_else(|| "未找到豆瓣搜索结果数据".to_string())?;
    serde_json::from_str::<DoubanSearchPageData>(payload).map_err(|error| error.to_string())
}

fn sanitize_douban_title(raw_title: &str) -> String {
    let without_mark = raw_title.replace('\u{200e}', "");
    douban_search_year_suffix_regex()
        .replace_all(without_mark.trim(), "")
        .trim()
        .to_string()
}

fn extract_search_title_year(raw_title: &str) -> String {
    douban_search_year_regex()
        .captures(raw_title)
        .and_then(|captures| captures.get(1))
        .map(|value| value.as_str().to_string())
        .unwrap_or_default()
}

/// 把豆瓣评分（如 `9.4` / `8.0`）格式化为定长字符串：整数不带小数点，非整数保留 1 位小数。
fn format_douban_rating(value: f64) -> String {
    let tenths = (value * 10.0).round() as i64;
    if tenths % 10 == 0 {
        (tenths / 10).to_string()
    } else {
        format!("{}.{}", tenths / 10, tenths % 10)
    }
}

fn is_douban_search_subject_item(item: &DoubanSearchPageItem) -> bool {
    item.tpl_name.as_deref() == Some("search_subject")
        && item.id.is_some_and(|value| value > 0)
        && item
            .title
            .as_deref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
}

fn infer_douban_search_play_type(item: &DoubanSearchPageItem) -> &'static str {
    if item
        .labels
        .iter()
        .any(|label| label.text.as_deref() == Some("剧集"))
    {
        "tv"
    } else {
        "movie"
    }
}

fn map_douban_search_item(item: &DoubanSearchPageItem) -> DoubanPreview {
    DoubanPreview {
        id: item.id.unwrap_or_default().to_string(),
        title: sanitize_douban_title(item.title.as_deref().unwrap_or_default()),
        poster: item.cover_url.clone().unwrap_or_default(),
        year: extract_search_title_year(item.title.as_deref().unwrap_or_default()),
        play_type: infer_douban_search_play_type(item),
        rating: item
            .rating
            .as_ref()
            .and_then(|rating| rating.value)
            .filter(|value| *value > 0.0)
            .map(format_douban_rating),
    }
}

#[async_trait]
impl Addon for DoubanAddon {
    async fn manifest(&self) -> Manifest {
        Manifest {
            id: "cineharbor.douban".into(),
            version: "0.1.0".into(),
            name: "CineHarbor Douban".into(),
            description: Some("豆瓣检索 addon（catalog/search，standalone）".into()),
            resources: vec![Resource::Catalog],
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
            id_prefixes: Some(vec!["douban".into()]),
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
        let start = req.skip.map(|s| s as usize).unwrap_or(0);
        let previews = match self.search(query.trim(), start).await {
            Ok(previews) => previews,
            Err(_) => return CatalogResponse::default(),
        };
        let metas = previews
            .into_iter()
            .map(|preview| {
                let ty = content_type_for(preview.play_type);
                MetaPreview {
                    poster: (!preview.poster.is_empty()).then_some(preview.poster),
                    year: (!preview.year.is_empty()).then_some(preview.year),
                    rating: preview.rating,
                    ..MetaPreview::new(format!("{ID_PREFIX}{}", preview.id), ty, &preview.title)
                }
            })
            .collect();
        CatalogResponse { metas }
    }

    async fn meta(&self, _ty: ContentType, _id: &str) -> Option<MetaResponse> {
        None
    }

    async fn streams(&self, _ty: ContentType, _id: &str) -> StreamsResponse {
        StreamsResponse::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEARCH_HTML: &str = r#"<html><head></head><body><script>
      window.__DATA__ = {"total": 2, "items": [
        {"tpl_name": "search_subject", "id": 3541415, "title": "星际穿越 Interstellar (2014)",
         "cover_url": "https://img.doubanio.com/view/p.jpg", "labels": [{"text": "电影"}],
         "rating": {"value": 9.4, "count": 800000}},
        {"tpl_name": "movie", "id": 0, "title": "", "labels": []}
      ]};
    </script></body></html>"#;

    #[test]
    fn extracts_and_maps_search_html() {
        let data = extract_douban_search_data(SEARCH_HTML).expect("parse __DATA__");

        let previews: Vec<DoubanPreview> = data
            .items
            .iter()
            .filter(|item| is_douban_search_subject_item(item))
            .map(map_douban_search_item)
            .collect();
        assert_eq!(previews.len(), 1);

        let preview = &previews[0];
        assert_eq!(preview.id, "3541415");
        assert_eq!(preview.title, "星际穿越 Interstellar");
        assert_eq!(preview.year, "2014");
        assert_eq!(preview.play_type, "movie");
        assert_eq!(preview.rating.as_deref(), Some("9.4"));
        assert!(preview.poster.ends_with("p.jpg"));
    }

    #[test]
    fn maps_play_type() {
        assert_eq!(content_type_for("tv"), ContentType::Series);
        assert_eq!(content_type_for("movie"), ContentType::Movie);
    }

    #[test]
    fn builds_search_url() {
        let url = build_search_url(SEARCH_BASE_URL, "星际穿越", 0);
        assert_eq!(
            url,
            "https://search.douban.com/movie/subject_search?search_text=%E6%98%9F%E9%99%85%E7%A9%BF%E8%B6%8A&cat=1002"
        );
        let paged = build_search_url(SEARCH_BASE_URL, "星际穿越", 20);
        assert!(paged.contains("start=20"));
    }
}