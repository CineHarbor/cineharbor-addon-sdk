//! Bangumi（bgm.tv）参考 addon：动画元数据目录/详情。
//!
//! 纯 catalog/meta addon（bangumi.tv 不提供视频流），演示把上游公开源包装成
//! Stremio 兼容 addon。`search` 端点需要 bgm.tv 应用令牌，此处只实现公开的
//! `/calendar` 与 `/v0/subjects/{id}`。

use async_trait::async_trait;
use cineharbor_addon_protocol::{
    Catalog, CatalogResponse, ContentType, Manifest, MetaDetail, MetaPreview, MetaResponse,
    Resource, StreamsResponse,
};
use cineharbor_addon_sdk::addon::{Addon, CatalogRequest};
use serde::Deserialize;

const DEFAULT_BASE_URL: &str = "https://api.bgm.tv";
const ID_PREFIX: &str = "bangumi:";

#[derive(Debug, thiserror::Error)]
pub enum BangumiError {
    #[error("HTTP 请求失败: {0}")]
    Http(#[from] reqwest::Error),
}

pub struct BangumiAddon {
    http: reqwest::Client,
    base_url: String,
}

impl Default for BangumiAddon {
    fn default() -> Self {
        Self::new()
    }
}

impl BangumiAddon {
    pub fn new() -> Self {
        let base_url = std::env::var("CINEHARBOR_BANGUMI_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
        Self::with_base_url(base_url)
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        let configured = base_url.into();
        let trimmed = configured.trim().trim_end_matches('/');
        let base_url = if trimmed.is_empty() {
            DEFAULT_BASE_URL.to_string()
        } else {
            trimmed.to_string()
        };

        Self {
            http: reqwest::ClientBuilder::new()
                .user_agent("CineHarborAddon/0.1 (https://github.com/CineHarbor)")
                .build()
                .expect("build http client"),
            base_url,
        }
    }

    async fn fetch(&self, path: &str) -> Result<String, BangumiError> {
        let body = self
            .http
            .get(format!("{}{path}", self.base_url))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        Ok(body)
    }
}

#[async_trait]
impl Addon for BangumiAddon {
    async fn manifest(&self) -> Manifest {
        Manifest {
            id: "community.bangumi".into(),
            version: "1.0.0".into(),
            name: "Bangumi".into(),
            description: Some("收录 bangumi.tv 动画条目的元数据参考 addon".into()),
            resources: vec![Resource::Catalog, Resource::Meta],
            types: vec![ContentType::Series],
            catalogs: vec![Catalog {
                r#type: ContentType::Series,
                id: "calendar".into(),
                name: "每日放送".into(),
                extra: vec![],
                extra_supported: vec![],
            }],
            id_prefixes: Some(vec!["bangumi".into()]),
            icon: None,
            logo: None,
            background: None,
            behavior_hints: None,
        }
    }

    async fn catalog(&self, _req: CatalogRequest) -> CatalogResponse {
        match self.fetch("/calendar").await {
            Ok(body) => map_calendar(&body).unwrap_or_default(),
            Err(_) => CatalogResponse::default(),
        }
    }

    async fn meta(&self, _ty: ContentType, id: &str) -> Option<MetaResponse> {
        let subject_id = id.strip_prefix(ID_PREFIX)?;
        let body = self
            .fetch(&format!("/v0/subjects/{subject_id}"))
            .await
            .ok()?;
        map_subject(&body).ok()
    }

    async fn streams(&self, _ty: ContentType, _id: &str) -> StreamsResponse {
        StreamsResponse::default()
    }
}

// —— 上游 JSON → 协议类型（纯函数，可离线单测）——

#[derive(Deserialize, Default)]
struct CalendarWeekday {
    #[serde(default)]
    en: String,
}

#[derive(Deserialize)]
struct CalendarDay {
    #[serde(default)]
    weekday: CalendarWeekday,
    #[serde(default)]
    items: Vec<CalendarItem>,
}

#[derive(Deserialize, Default)]
struct CalendarRating {
    #[serde(default)]
    score: f64,
}

#[derive(Deserialize)]
struct CalendarItem {
    id: u64,
    name: String,
    #[serde(default)]
    name_cn: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    air_date: String,
    #[serde(default)]
    images: BgmImages,
    #[serde(default)]
    rating: CalendarRating,
}

#[derive(Deserialize)]
struct Subject {
    id: u64,
    name: String,
    #[serde(default)]
    name_cn: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    date: String,
    images: BgmImages,
}

#[derive(Deserialize, Default)]
struct BgmImages {
    #[serde(default)]
    large: Option<String>,
}

fn display_name(name: &str, name_cn: &str) -> String {
    if name_cn.is_empty() {
        name.to_string()
    } else {
        name_cn.to_string()
    }
}

fn year_of(date: &str) -> Option<String> {
    (date.len() >= 4).then(|| date.chars().take(4).collect())
}

fn nonempty(s: &str) -> Option<String> {
    if s.trim().is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn format_score(score: f64) -> Option<String> {
    if score <= 0.0 {
        return None;
    }
    let tenths = (score * 10.0).round() as i64;
    if tenths % 10 == 0 {
        Some((tenths / 10).to_string())
    } else {
        Some(format!("{}.{}", tenths / 10, tenths % 10))
    }
}

/// 把 bgm.tv `/calendar` 响应映射为目录（摊平 7 天）。
///
/// 页面需要按星期分组：`genres[0]` = `weekday.en`（Mon…Sun）；
/// `name` = 中文名（无则原名），`description` = 原名（有中文名时），
/// `release_info` = `air_date`，`rating` = 评分。
pub fn map_calendar(json: &str) -> Result<CatalogResponse, serde_json::Error> {
    let days: Vec<CalendarDay> = serde_json::from_str(json)?;
    let metas = days
        .into_iter()
        .flat_map(|day| {
            let weekday = day.weekday.en;
            day.items.into_iter().map(move |it| {
                let has_cn = !it.name_cn.trim().is_empty();
                MetaPreview {
                    poster: it.images.large.filter(|value| !value.is_empty()),
                    description: if has_cn {
                        nonempty(&it.name)
                    } else {
                        nonempty(&it.summary)
                    },
                    year: year_of(&it.air_date),
                    release_info: nonempty(&it.air_date),
                    rating: format_score(it.rating.score),
                    genres: nonempty(&weekday)
                        .map(|value| vec![value])
                        .unwrap_or_default(),
                    ..MetaPreview::new(
                        format!("{ID_PREFIX}{}", it.id),
                        ContentType::Series,
                        display_name(&it.name, &it.name_cn),
                    )
                }
            })
        })
        .collect();
    Ok(CatalogResponse { metas })
}

/// 把 bgm.tv `/v0/subjects/{id}` 响应映射为详情。
pub fn map_subject(json: &str) -> Result<MetaResponse, serde_json::Error> {
    let s: Subject = serde_json::from_str(json)?;
    Ok(MetaResponse {
        meta: MetaDetail {
            poster: s.images.large,
            description: nonempty(&s.summary),
            year: year_of(&s.date),
            ..MetaDetail::new(
                format!("{ID_PREFIX}{}", s.id),
                ContentType::Series,
                display_name(&s.name, &s.name_cn),
            )
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CALENDAR: &str = r#"[
      { "weekday": { "en": "Fri", "id": 5 },
        "items": [
          { "id": 40748, "name": "Sousou no Frieren", "name_cn": "葬送的芙莉莲",
            "summary": "魔法使芙莉莲的旅途", "air_date": "2023-09-29",
            "images": { "large": "https://lain.bgm.tv/pic/cover/l/frieren.jpg" },
            "rating": { "score": 9.0 } }
        ] }
    ]"#;

    const SUBJECT: &str = r#"{
      "id": 40748, "name": "Sousou no Frieren", "name_cn": "葬送的芙莉莲",
      "summary": "魔法使芙莉莲的旅途", "date": "2023-09-29",
      "images": { "large": "https://lain.bgm.tv/pic/cover/l/frieren.jpg" }
    }"#;

    #[test]
    fn normalizes_configured_upstream_base_url() {
        let addon = BangumiAddon::with_base_url(" http://127.0.0.1:49174/ ");
        assert_eq!(addon.base_url, "http://127.0.0.1:49174");
    }

    #[test]
    fn maps_calendar() {
        let resp = map_calendar(CALENDAR).unwrap();
        assert_eq!(resp.metas.len(), 1);
        let m = &resp.metas[0];
        assert_eq!(m.id, "bangumi:40748");
        assert_eq!(m.r#type, ContentType::Series);
        assert_eq!(m.name, "葬送的芙莉莲");
        assert_eq!(m.year.as_deref(), Some("2023"));
        assert_eq!(m.release_info.as_deref(), Some("2023-09-29"));
        assert_eq!(m.description.as_deref(), Some("Sousou no Frieren"));
        assert_eq!(m.genres, vec!["Fri"]);
        assert_eq!(m.rating.as_deref(), Some("9"));
        assert!(m.poster.is_some());
    }

    #[test]
    fn maps_subject() {
        let resp = map_subject(SUBJECT).unwrap();
        assert_eq!(resp.meta.id, "bangumi:40748");
        assert_eq!(resp.meta.name, "葬送的芙莉莲");
        assert_eq!(resp.meta.year.as_deref(), Some("2023"));
    }
}
