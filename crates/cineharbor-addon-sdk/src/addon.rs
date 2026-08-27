//! 供给侧：实现 [`Addon`] trait，用 [`router`] 生成 Stremio 兼容的 axum 路由。
//!
//! addon 端点以 `.json` 结尾（Stremio 契约），axum 路径参数不识别字面量后缀，
//! 因此在 handler 内剥离 `.json`，并在约定处解析 `/skip=N` 与 `/name=value`
//! 扩展参数。与消费侧 [`crate::client::AddonClient`] 的 URL 构造保持一致。

use std::sync::Arc;

use async_trait::async_trait;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};

use cineharbor_addon_protocol::{
    CatalogResponse, ContentType, Manifest, MetaResponse, StreamsResponse, SubtitlesResponse,
};

/// 目录查询参数。
#[derive(Debug, Clone)]
pub struct CatalogRequest {
    pub ty: ContentType,
    pub id: String,
    pub extra: Option<(String, String)>,
    pub skip: Option<u32>,
}

/// 供给侧 addon 接口。单 addon 进程内共享实现，需 `Send + Sync`。
#[async_trait]
pub trait Addon: Send + Sync {
    async fn manifest(&self) -> Manifest;
    async fn catalog(&self, req: CatalogRequest) -> CatalogResponse;
    /// 返回 `None` 代表 404。
    async fn meta(&self, ty: ContentType, id: &str) -> Option<MetaResponse>;
    async fn streams(&self, ty: ContentType, id: &str) -> StreamsResponse;
    async fn subtitles(&self, ty: ContentType, id: &str, extra: &str) -> SubtitlesResponse {
        let _ = (ty, id, extra);
        SubtitlesResponse::default()
    }
}

/// 由 addon 生成 Stremio 兼容路由。P3 由 `cineharbor-local-service` 作 host 复用。
pub fn router(addon: Arc<dyn Addon>) -> Router {
    Router::new()
        .route("/manifest.json", get(manifest_handler))
        .route("/catalog/{ty}/{id}", get(catalog_handler))
        .route("/catalog/{ty}/{id}/{seg}", get(catalog_extra_handler))
        .route("/meta/{ty}/{id}", get(meta_handler))
        .route("/stream/{ty}/{id}", get(stream_handler))
        .route("/subtitles/{ty}/{id}/{seg}", get(subtitles_handler))
        .with_state(addon)
}

fn strip_json(s: &str) -> &str {
    s.strip_suffix(".json").unwrap_or(s)
}

fn parse_ty(s: &str) -> Option<ContentType> {
    s.parse().ok()
}

async fn manifest_handler(State(addon): State<Arc<dyn Addon>>) -> Json<Manifest> {
    Json(addon.manifest().await)
}

async fn catalog_handler(
    State(addon): State<Arc<dyn Addon>>,
    Path((ty, id)): Path<(String, String)>,
) -> Result<Json<CatalogResponse>, StatusCode> {
    let ty = parse_ty(&ty).ok_or(StatusCode::BAD_REQUEST)?;
    let req = CatalogRequest {
        ty,
        id: strip_json(&id).to_string(),
        extra: None,
        skip: None,
    };
    Ok(Json(addon.catalog(req).await))
}

async fn catalog_extra_handler(
    State(addon): State<Arc<dyn Addon>>,
    Path((ty, id, seg)): Path<(String, String, String)>,
) -> Result<Json<CatalogResponse>, StatusCode> {
    let ty = parse_ty(&ty).ok_or(StatusCode::BAD_REQUEST)?;
    let seg = strip_json(&seg);
    let mut req = CatalogRequest {
        ty,
        id,
        extra: None,
        skip: None,
    };
    if let Some(n) = seg.strip_prefix("skip=") {
        req.skip = n.parse::<u32>().ok();
    } else {
        let (name, value) = seg.split_once('=').ok_or(StatusCode::BAD_REQUEST)?;
        req.extra = Some((name.to_string(), value.to_string()));
    }
    Ok(Json(addon.catalog(req).await))
}

async fn meta_handler(
    State(addon): State<Arc<dyn Addon>>,
    Path((ty, id)): Path<(String, String)>,
) -> Result<Json<MetaResponse>, StatusCode> {
    let ty = parse_ty(&ty).ok_or(StatusCode::BAD_REQUEST)?;
    addon
        .meta(ty, strip_json(&id))
        .await
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn stream_handler(
    State(addon): State<Arc<dyn Addon>>,
    Path((ty, id)): Path<(String, String)>,
) -> Result<Json<StreamsResponse>, StatusCode> {
    let ty = parse_ty(&ty).ok_or(StatusCode::BAD_REQUEST)?;
    Ok(Json(addon.streams(ty, strip_json(&id)).await))
}

async fn subtitles_handler(
    State(addon): State<Arc<dyn Addon>>,
    Path((ty, id, seg)): Path<(String, String, String)>,
) -> Result<Json<SubtitlesResponse>, StatusCode> {
    let ty = parse_ty(&ty).ok_or(StatusCode::BAD_REQUEST)?;
    Ok(Json(
        addon.subtitles(ty, strip_json(&id), strip_json(&seg)).await,
    ))
}
