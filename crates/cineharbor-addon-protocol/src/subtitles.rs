//! 字幕。

use serde::{Deserialize, Serialize};

/// subtitles 端点响应。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SubtitlesResponse {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subtitles: Vec<Subtitle>,
}

/// 单条字幕。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subtitle {
    pub id: String,
    pub url: String,
    pub lang: String,
}
