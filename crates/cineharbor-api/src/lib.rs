//! vod 抓取的共享核心（`stremio` CustomAPI 视频站协议的解析/URL 构造，纯逻辑、无 IO）。
//!
//! P2 的第三块：把 local-service 的 `content_search`/`content_detail` 抓取逻辑抽为共享
//! crate，供 standalone vod addon 与（后续）local-service 共用。当前与 local-service 内
//! 置版并行（strangler）；后续 local-service 改依赖本 crate 后删除其内部拷贝。
//!
//! 纯函数、可离线单测：网络请求（`search_site`/`fetch_content_detail`）随 addon 落地。

use std::sync::OnceLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use cineharbor_core::model::SearchResult;

/// 单个 CustomAPI 视频站配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiSite {
    pub key: String,
    pub api: String,
    pub name: String,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub ua: Option<String>,
    #[serde(default)]
    pub referer: Option<String>,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub disable_ad_filter: bool,
}

// —— 值规整 ——

pub fn value_to_string(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::String(inner)) => Some(inner.to_string()),
        Some(Value::Number(inner)) => Some(inner.to_string()),
        Some(Value::Bool(inner)) => Some(inner.to_string()),
        _ => None,
    }
}

pub fn value_to_i64(value: Option<&Value>) -> Option<i64> {
    match value {
        Some(Value::Number(inner)) => inner.as_i64(),
        Some(Value::String(inner)) => inner.parse::<i64>().ok(),
        _ => None,
    }
}

pub fn parse_usize(value: Option<&Value>) -> Option<usize> {
    match value {
        Some(Value::Number(inner)) => inner.as_u64().map(|item| item as usize),
        Some(Value::String(inner)) => inner.parse::<usize>().ok(),
        _ => None,
    }
}

pub fn normalize_year(value: Option<&str>) -> String {
    let Some(value) = value else {
        return "unknown".to_string();
    };
    year_value_regex()
        .captures(value)
        .and_then(|capture| capture.get(1))
        .map(|item| item.as_str().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

pub fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn clean_html_tags(value: &str) -> String {
    if value.trim().is_empty() {
        return String::new();
    }
    let cleaned = html_tag_regex().replace_all(value, "\n").replace('\r', "\n");
    let lines = cleaned
        .split('\n')
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    html_escape::decode_html_entities(&lines.join("\n")).to_string()
}

pub fn is_valid_content_id(id: &str) -> bool {
    id.chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_' || character == '-')
}

// —— 剧集抽取 ——

/// 解析 CustomAPI 的 `vod_play_url`（`$$$` 分源、`#` 分集、`剧名$url`）。
pub fn extract_episodes_from_play_url(play_url: Option<&str>) -> (Vec<String>, Vec<String>) {
    let Some(play_url) = play_url else {
        return (Vec::new(), Vec::new());
    };

    let mut episodes = Vec::new();
    let mut titles = Vec::new();

    for candidate_group in play_url.split("$$$") {
        let mut candidate_episodes = Vec::new();
        let mut candidate_titles = Vec::new();

        for title_url in candidate_group.split('#') {
            let mut parts = title_url.splitn(2, '$');
            let Some(title) = parts.next() else {
                continue;
            };
            let Some(url) = parts.next() else {
                continue;
            };

            if looks_like_manifest_url(url.trim()) {
                candidate_titles.push(title.trim().to_string());
                candidate_episodes.push(url.trim().to_string());
            }
        }

        if candidate_episodes.len() > episodes.len() {
            episodes = candidate_episodes;
            titles = candidate_titles;
        }
    }

    (episodes, titles)
}

pub fn looks_like_manifest_url(url: &str) -> bool {
    manifest_url_regex().is_match(url)
}

// —— URL 构造 ——

pub fn build_collection_api_url(api_base_url: &str, params: &[(&str, &str)]) -> Result<String, String> {
    let api_base_url = api_base_url.trim();
    url::Url::parse(api_base_url)
        .map_err(|error| format!("invalid api url: {api_base_url}: {error}"))?;

    if params.is_empty() {
        return Ok(api_base_url.to_string());
    }

    let separator = if api_base_url.ends_with('?') || api_base_url.ends_with('&') {
        ""
    } else if collection_api_url_uses_wrapped_target(api_base_url) {
        "?"
    } else if api_base_url.contains('?') {
        "&"
    } else {
        "?"
    };

    Ok(format!(
        "{api_base_url}{separator}{}",
        build_collection_api_query(params)
    ))
}

// —— 目录解析 ——

pub fn parse_search_payload(payload: &Value, api_site: &ApiSite) -> Vec<SearchResult> {
    let Some(list) = payload.get("list").and_then(Value::as_array) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|item| parse_search_item(item, api_site))
        .filter(|result| !result.episodes.is_empty())
        .collect()
}

pub fn parse_search_item(item: &Value, api_site: &ApiSite) -> Option<SearchResult> {
    let id = value_to_string(item.get("vod_id"))?;
    let title = collapse_whitespace(&value_to_string(item.get("vod_name"))?);
    let poster = value_to_string(item.get("vod_pic")).unwrap_or_default();
    let (episodes, episode_titles) =
        extract_episodes_from_play_url(value_to_string(item.get("vod_play_url")).as_deref());

    Some(SearchResult {
        id,
        title,
        poster,
        episodes,
        episodes_titles: episode_titles,
        source: api_site.key.clone(),
        source_name: api_site.name.clone(),
        class: value_to_string(item.get("vod_class")),
        year: normalize_year(value_to_string(item.get("vod_year")).as_deref()),
        desc: value_to_string(item.get("vod_content")).map(|value| clean_html_tags(&value)),
        type_name: value_to_string(item.get("type_name")),
        douban_id: value_to_i64(item.get("vod_douban_id")),
    })
}

// —— 详情解析 ——

pub fn parse_detail_payload(payload: &Value, api_site: &ApiSite, id: &str) -> Option<SearchResult> {
    let list = payload.get("list")?.as_array()?;
    let video_detail = list.first()?;
    let (mut episodes, mut episode_titles) = extract_episodes_from_play_url(
        value_to_string(video_detail.get("vod_play_url")).as_deref(),
    );

    if episodes.is_empty() {
        if let Some(content) = value_to_string(video_detail.get("vod_content")) {
            episodes = extract_m3u8_matches(&content);
            episode_titles = (1..=episodes.len()).map(|index| index.to_string()).collect();
        }
    }

    Some(SearchResult {
        id: id.to_string(),
        title: value_to_string(video_detail.get("vod_name")).unwrap_or_default(),
        poster: value_to_string(video_detail.get("vod_pic")).unwrap_or_default(),
        episodes,
        episodes_titles: episode_titles,
        source: api_site.key.clone(),
        source_name: api_site.name.clone(),
        class: value_to_string(video_detail.get("vod_class")),
        year: normalize_year(value_to_string(video_detail.get("vod_year")).as_deref()),
        desc: value_to_string(video_detail.get("vod_content")).map(|value| clean_html_tags(&value)),
        type_name: value_to_string(video_detail.get("type_name")),
        douban_id: value_to_i64(video_detail.get("vod_douban_id")),
    })
}

/// 从 detail HTML 页抽取剧集/标题/描述/海报/年份（ffzy/feifan 走专用 m3u8 规则）。
pub fn parse_custom_detail_html(html: &str, api_site: &ApiSite, id: &str) -> SearchResult {
    let mut matches = if matches!(api_site.key.as_str(), "ffzy" | "feifan") {
        let special = special_ffzy_m3u8_regex()
            .captures_iter(html)
            .filter_map(|capture| capture.get(1).map(|item| item.as_str().to_string()))
            .collect::<Vec<_>>();
        special
    } else {
        Vec::new()
    };

    if matches.is_empty() {
        matches = m3u8_regex()
            .captures_iter(html)
            .filter_map(|capture| capture.get(1).map(|item| item.as_str().to_string()))
            .collect();
    }

    let mut deduped_matches = Vec::new();
    for raw_match in matches {
        let cleaned_match = raw_match
            .trim()
            .trim_start_matches('$')
            .split('(')
            .next()
            .unwrap_or_default()
            .trim()
            .to_string();

        if !cleaned_match.is_empty() && !deduped_matches.contains(&cleaned_match) {
            deduped_matches.push(cleaned_match);
        }
    }

    let title = title_regex()
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|item| item.as_str().trim().to_string())
        .unwrap_or_default();
    let desc = detail_desc_regex()
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|item| clean_html_tags(item.as_str()));
    let poster = cover_regex()
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|item| item.as_str().trim().to_string())
        .unwrap_or_default();
    let year = year_regex()
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|item| item.as_str().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    SearchResult {
        id: id.to_string(),
        title,
        poster,
        episodes_titles: (1..=deduped_matches.len()).map(|index| index.to_string()).collect(),
        episodes: deduped_matches,
        source: api_site.key.clone(),
        source_name: api_site.name.clone(),
        class: Some(String::new()),
        year,
        desc,
        type_name: Some(String::new()),
        douban_id: Some(0),
    }
}

pub fn has_custom_detail_url(api_site: &ApiSite) -> bool {
    api_site
        .detail
        .as_deref()
        .map(|detail| detail.starts_with("http://") || detail.starts_with("https://"))
        .unwrap_or(false)
}

// —— 内部：URL 构造 ——

fn collection_api_url_uses_wrapped_target(api_base_url: &str) -> bool {
    match url::Url::parse(api_base_url) {
        Ok(url) => url.query_pairs().any(|(key, _)| key == "url"),
        Err(_) => false,
    }
}

fn build_collection_api_query(params: &[(&str, &str)]) -> String {
    params
        .iter()
        .map(|(key, value)| {
            format!(
                "{}={}",
                encode_collection_api_query_component(key),
                encode_collection_api_query_component(value)
            )
        })
        .collect::<Vec<_>>()
        .join("&")
}

fn encode_collection_api_query_component(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes())
        .collect::<String>()
        .replace('+', "%20")
}

// —— 内部：regex（OnceLock 缓存） ——

fn year_value_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(\d{4})").expect("valid year value regex"))
}

fn manifest_url_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?i)\.m3u8($|[?#])").expect("valid manifest url regex"))
}

fn html_tag_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"<[^>]+>").expect("valid html tag regex"))
}

fn m3u8_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"\$(https?://[^"'\s]+?\.m3u8(?:\?[^"'\s]*)?)"#).expect("valid m3u8 regex")
    })
}

fn special_ffzy_m3u8_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"\$(https?://[^"'\s]+?/\d{8}/\d+_[a-f0-9]+/index\.m3u8)"#)
            .expect("valid ffzy detail regex")
    })
}

fn title_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r#"<h1[^>]*>([^<]+)</h1>"#).expect("valid title regex"))
}

fn detail_desc_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"<div[^>]*class=["']sketch["'][^>]*>([\s\S]*?)</div>"#).expect("valid desc regex")
    })
}

fn cover_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"(https?://[^"'\s]+?\.(jpg|jpeg|png|webp))"#).expect("valid cover regex")
    })
}

fn year_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r#">(\d{4})<"#).expect("valid year regex"))
}

fn extract_m3u8_matches(content: &str) -> Vec<String> {
    m3u8_regex()
        .captures_iter(content)
        .filter_map(|capture| capture.get(1).map(|item| item.as_str().to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn site() -> ApiSite {
        ApiSite {
            key: "demo".into(),
            api: "http://api.test/provide/vod/".into(),
            name: "Demo".into(),
            detail: None,
            ua: None,
            referer: None,
            disabled: false,
            disable_ad_filter: false,
        }
    }

    #[test]
    fn extracts_episodes_from_play_url() {
        let (episodes, titles) = extract_episodes_from_play_url(Some(
            "第01集$http://e.test/1.m3u8#第02集$http://e.test/2.m3u8",
        ));
        assert_eq!(episodes, vec!["http://e.test/1.m3u8", "http://e.test/2.m3u8"]);
        assert_eq!(titles, vec!["第01集", "第02集"]);
    }

    #[test]
    fn parses_search_payload() {
        let payload = json!({
            "pagecount": 1,
            "list": [{
                "vod_id": 123,
                "vod_name": "矩阵 The Matrix",
                "vod_pic": "http://p.test/x.jpg",
                "vod_play_url": "第01集$http://e.test/1.m3u8",
                "vod_class": "科幻",
                "vod_year": "1999",
                "vod_content": "<p>剧情简介</p>",
                "type_name": "电影",
                "vod_douban_id": 1291843
            }]
        });
        let results = parse_search_payload(&payload, &site());
        assert_eq!(results.len(), 1);
        let r = &results[0];
        assert_eq!(r.id, "123");
        assert_eq!(r.title, "矩阵 The Matrix");
        assert_eq!(r.year, "1999");
        assert_eq!(r.desc.as_deref(), Some("剧情简介"));
        assert_eq!(r.douban_id, Some(1291843));
        assert_eq!(r.episodes.len(), 1);
    }

    #[test]
    fn parses_detail_payload() {
        let payload = json!({
            "list": [{
                "vod_name": "矩阵 The Matrix",
                "vod_pic": "http://p.test/x.jpg",
                "vod_play_url": "第01集$http://e.test/1.m3u8#第02集$http://e.test/2.m3u8",
                "vod_year": "1999",
                "type_name": "电影"
            }]
        });
        let detail = parse_detail_payload(&payload, &site(), "123").unwrap();
        assert_eq!(detail.id, "123");
        assert_eq!(detail.episodes.len(), 2);
        assert_eq!(detail.year, "1999");
    }

    #[test]
    fn parses_custom_detail_html() {
        let html = r#"
            <html><head><title>t</title></head><body>
            <h1>矩阵 The Matrix</h1>
            <div class="sketch">剧情简介内容</div>
            <img src="http://p.test/x.jpg">
            <div class="year">(1999)</div>
            <script>var a = "$http://cdn.test/20240101/abc123/index.m3u8";</script>
            </body></html>
        "#;
        let result = parse_custom_detail_html(html, &site(), "123");
        assert_eq!(result.title, "矩阵 The Matrix");
        assert_eq!(result.episodes.len(), 1);
        assert!(result.episodes[0].contains("index.m3u8"));
    }

    #[test]
    fn builds_collection_urls() {
        assert_eq!(
            build_collection_api_url("http://api.test/provide/vod/", &[]).unwrap(),
            "http://api.test/provide/vod/"
        );
        assert_eq!(
            build_collection_api_url("http://api.test/provide/vod/", &[("ac", "videolist")]).unwrap(),
            "http://api.test/provide/vod/?ac=videolist"
        );
        let encoded = build_collection_api_url("http://api.test/x", &[("wd", "矩阵")]).unwrap();
        assert!(encoded.contains("wd=%E7%9F%A9%E9%98%B5"), "{encoded}");
        let with_query = build_collection_api_url(
            "http://api.test/x?fixed=1",
            &[("ac", "videolist")],
        )
        .unwrap();
        assert_eq!(with_query, "http://api.test/x?fixed=1&ac=videolist");
    }
}