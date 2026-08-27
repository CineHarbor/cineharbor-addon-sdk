//! 元数据详情。

use serde::{Deserialize, Serialize};

use crate::manifest::BehaviorHints;
use crate::streams::Stream;
use crate::ContentType;

/// meta 端点响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetaResponse {
    pub meta: MetaDetail,
}

/// 内容元数据详情。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaDetail {
    pub id: String,
    #[serde(rename = "type")]
    pub r#type: ContentType,
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub genres: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poster: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_info: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imdb_rating: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub director: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cast: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub videos: Vec<Video>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Link>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub behavior_hints: Option<BehaviorHints>,
}

impl MetaDetail {
    /// 最小构造：只填 id/type/name，其余字段留默认空值。
    pub fn new(id: impl Into<String>, r#type: ContentType, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            r#type,
            name: name.into(),
            genres: Vec::new(),
            poster: None,
            background: None,
            logo: None,
            description: None,
            release_info: None,
            year: None,
            country: None,
            runtime: None,
            imdb_rating: None,
            director: Vec::new(),
            cast: Vec::new(),
            slug: None,
            videos: Vec::new(),
            links: Vec::new(),
            behavior_hints: None,
        }
    }
}

/// 视频条目（集/预告片等）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Video {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub season: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub episode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub released: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<Stream>,
}

/// 外部链接。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub name: String,
    pub category: String,
    pub url: String,
}
