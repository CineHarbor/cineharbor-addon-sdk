use std::sync::Arc;

use axum::{Json, Router, routing::get};
use cineharbor_addon_protocol::ContentType;
use cineharbor_addon_sdk::addon::{Addon, CatalogRequest};
use cineharbor_addon_vod::{VodAddon, VodConfig};
use serde_json::json;

// Real HTTP upstream and production parser, not a mocked catalog implementation.
#[tokio::test]
async fn catalogs_filter_type_before_skip_and_reject_undeclared_catalogs() {
    let payload = Arc::new(json!({
        "pagecount": 1,
        "list": [
            {"vod_id": "101", "vod_name": "Movie one", "type_name": "电影", "vod_play_url": "正片$https://cdn.test/101.m3u8"},
            {"vod_id": "202", "vod_name": "Series one", "type_name": "连续剧", "vod_play_url": "1$https://cdn.test/202.m3u8"},
            {"vod_id": "103", "vod_name": "Movie two", "type_name": "电影", "vod_play_url": "正片$https://cdn.test/103.m3u8"},
            {"vod_id": "204", "vod_name": "Series two", "type_name": "TV Series", "vod_play_url": "1$https://cdn.test/204.m3u8"}
        ]
    }));
    let app = Router::new().route(
        "/api",
        get(move || {
            let payload = payload.clone();
            async move { Json((*payload).clone()) }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let config = VodConfig::from_json(
        &json!({
            "sites": [{"key": "mock", "name": "Mock", "api": format!("http://{address}/api")}]
        })
        .to_string(),
    )
    .unwrap();
    let addon = VodAddon::new(config);
    let request = |ty, id: &str, skip| CatalogRequest {
        ty,
        id: id.into(),
        extra: Some(("search".into(), "one".into())),
        skip: Some(skip),
    };
    let movies = addon
        .catalog(request(ContentType::Movie, "search", 0))
        .await;
    let series = addon
        .catalog(request(ContentType::Series, "search", 0))
        .await;
    assert_eq!(
        movies
            .metas
            .iter()
            .map(|m| m.id.as_str())
            .collect::<Vec<_>>(),
        ["vod:mock:101", "vod:mock:103"]
    );
    assert!(movies.metas.iter().all(|m| m.r#type == ContentType::Movie));
    assert_eq!(
        series
            .metas
            .iter()
            .map(|m| m.id.as_str())
            .collect::<Vec<_>>(),
        ["vod:mock:202", "vod:mock:204"]
    );
    assert!(series.metas.iter().all(|m| m.r#type == ContentType::Series));
    assert_eq!(
        addon
            .catalog(request(ContentType::Movie, "search", 1))
            .await
            .metas[0]
            .id,
        "vod:mock:103"
    );
    assert_eq!(
        addon
            .catalog(request(ContentType::Series, "search", 1))
            .await
            .metas[0]
            .id,
        "vod:mock:204"
    );
    assert!(
        addon
            .catalog(request(ContentType::Movie, "search", 2))
            .await
            .metas
            .is_empty()
    );
    assert!(
        addon
            .catalog(request(ContentType::Movie, "unknown", 0))
            .await
            .metas
            .is_empty()
    );
    assert!(
        addon
            .catalog(request(ContentType::Tv, "search", 0))
            .await
            .metas
            .is_empty()
    );
    server.abort();
}
