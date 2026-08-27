//! 播放流。

use serde::{Deserialize, Serialize};

use crate::manifest::BehaviorHints;
use crate::subtitles::Subtitle;

/// stream 端点响应（字段名为 `streams`）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StreamsResponse {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub streams: Vec<Stream>,
}

/// 单条播放流：直链 url，或 torrent（infoHash + fileIdx），或 ytId。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stream {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yt_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub info_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_idx: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subtitles: Vec<Subtitle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub behavior_hints: Option<BehaviorHints>,
}
