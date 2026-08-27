//! 内容类型。

use serde::{Deserialize, Serialize};

/// Stremio 内容类型：movie / series / channel / tv。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    Movie,
    Series,
    Channel,
    Tv,
}

impl ContentType {
    pub const ALL: [ContentType; 4] = [
        ContentType::Movie,
        ContentType::Series,
        ContentType::Channel,
        ContentType::Tv,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ContentType::Movie => "movie",
            ContentType::Series => "series",
            ContentType::Channel => "channel",
            ContentType::Tv => "tv",
        }
    }
}

impl std::fmt::Display for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
