//! Stremio 兼容的 addon 协议：类型定义与基础校验。
//!
//! 契约冻结于本仓库根目录 [`protocol.md`](../../../protocol.md)，与官方
//! `stremio-addon-sdk` 逐项对齐。不扩展私有字段：自有扩展一律进 `behaviorHints`。

mod catalog;
mod content_type;
mod manifest;
mod meta;
mod streams;
mod subtitles;

pub use catalog::{CatalogResponse, MetaPreview};
pub use content_type::ContentType;
pub use manifest::{BehaviorHints, Catalog, Extra, Manifest, ManifestError, Resource};
pub use meta::{Link, MetaDetail, MetaResponse, Video};
pub use streams::{Stream, StreamsResponse};
pub use subtitles::{Subtitle, SubtitlesResponse};
