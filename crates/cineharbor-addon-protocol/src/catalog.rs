//! 目录响应与元数据预览。

use serde::{Deserialize, Serialize};

use crate::ContentType;
use crate::manifest::BehaviorHints;

/// catalog 端点响应。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CatalogResponse {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metas: Vec<MetaPreview>,
}

/// 元数据预览（catalog 列表项）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaPreview {
    pub id: String,
    #[serde(rename = "type")]
    pub r#type: ContentType,
    pub name: String,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub genres: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imdb_rating: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poster_shape: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub behavior_hints: Option<BehaviorHints>,
}

impl MetaPreview {
    /// 最小构造：只填 id/type/name，其余字段留默认空值。
    pub fn new(id: impl Into<String>, r#type: ContentType, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            r#type,
            name: name.into(),
            poster: None,
            background: None,
            logo: None,
            description: None,
            release_info: None,
            year: None,
            genres: Vec::new(),
            imdb_rating: None,
            poster_shape: None,
            behavior_hints: None,
        }
    }
}
