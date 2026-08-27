//! addon 清单：manifest / resources / catalogs。

use serde::{Deserialize, Serialize};

use crate::ContentType;

/// 资源类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resource {
    Catalog,
    Meta,
    Stream,
    Subtitles,
    AddonCatalog,
}

/// 目录扩展参数（search / genre / skip 等）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Extra {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    #[serde(default)]
    pub is_required: bool,
}

/// 目录声明。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Catalog {
    #[serde(rename = "type")]
    pub r#type: ContentType,
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<Extra>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_supported: Vec<String>,
}

/// 宿主行为提示：宽松透传 JSON，不在此穷举字段。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BehaviorHints(pub serde_json::Value);

/// addon 清单。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub id: String,
    pub version: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub resources: Vec<Resource>,
    pub types: Vec<ContentType>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub catalogs: Vec<Catalog>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_prefixes: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub behavior_hints: Option<BehaviorHints>,
}

impl Manifest {
    /// 基础校验：id/name/version 非空、至少一个资源。
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.id.trim().is_empty() {
            return Err(ManifestError::MissingField("id"));
        }
        if self.name.trim().is_empty() {
            return Err(ManifestError::MissingField("name"));
        }
        if self.version.trim().is_empty() {
            return Err(ManifestError::MissingField("version"));
        }
        if self.resources.is_empty() {
            return Err(ManifestError::NoResources);
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ManifestError {
    #[error("manifest 缺少必填字段: {0}")]
    MissingField(&'static str),
    #[error("manifest 必须声明至少一个资源")]
    NoResources,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_manifest() {
        let json = r#"{
            "id": "community.bangumi",
            "version": "1.0.0",
            "name": "Bangumi",
            "resources": ["catalog", "meta", "stream"],
            "types": ["series", "movie"],
            "catalogs": [
                { "type": "series", "id": "top", "name": "Top" }
            ]
        }"#;
        let m: Manifest = serde_json::from_str(json).expect("parse manifest");
        assert_eq!(m.id, "community.bangumi");
        assert!(m.validate().is_ok());
        assert_eq!(m.resources.len(), 3);
        assert_eq!(m.types[0], ContentType::Series);
        assert_eq!(m.catalogs[0].r#type, ContentType::Series);
    }

    #[test]
    fn rejects_invalid_manifest() {
        let m = Manifest {
            id: String::new(),
            version: "1.0.0".into(),
            name: String::new(),
            description: None,
            resources: vec![],
            types: vec![],
            catalogs: vec![],
            id_prefixes: None,
            icon: None,
            logo: None,
            background: None,
            behavior_hints: None,
        };
        assert_eq!(m.validate(), Err(ManifestError::MissingField("id")));
    }
}
