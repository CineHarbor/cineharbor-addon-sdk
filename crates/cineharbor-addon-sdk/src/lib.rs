//! CineHarbor addon 开发 SDK。
//!
//! 消费侧：以 addon base URL 拉取 Stremio 兼容 addon（[`AddonClient`]）。
//!
//! 供给侧（构建自有 addon 的 host 接线）在 P3 随 `cineharbor-local-service`
//! 一并落地，本 crate 先提供协议类型 + 客户端。

pub mod client;

pub use cineharbor_addon_protocol::*;
pub use client::{AddonClient, AddonClientError};
