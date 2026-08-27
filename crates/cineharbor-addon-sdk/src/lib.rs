//! CineHarbor addon 开发 SDK。
//!
//! - 消费侧：以 addon base URL 拉取 Stremio 兼容 addon（[`AddonClient`]）。
//! - 供给侧：实现 [`Addon`] trait，用 [`router`] 生成 Stremio 兼容 HTTP 路由。

pub mod addon;
pub mod client;

pub use cineharbor_addon_protocol::*;
pub use addon::{router, Addon, CatalogRequest};
pub use client::{AddonClient, AddonClientError};