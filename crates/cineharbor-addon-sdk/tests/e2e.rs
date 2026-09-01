//! 端到端：供给侧 [`router`] 服务 ↔ 消费侧 [`AddonClient`] 拉取，走真实回环 HTTP。

use std::sync::Arc;

use async_trait::async_trait;
use cineharbor_addon_protocol::{
    CatalogResponse, ContentType, Manifest, MetaResponse, Resource, Stream, StreamsResponse,
};
use cineharbor_addon_sdk::AddonClient;
use cineharbor_addon_sdk::addon::{Addon, CatalogRequest, router};

struct Hello;

#[async_trait]
impl Addon for Hello {
    async fn manifest(&self) -> Manifest {
        Manifest {
            id: "local.hello".into(),
            version: "1.0.0".into(),
            name: "Hello".into(),
            description: None,
            resources: vec![Resource::Catalog, Resource::Stream],
            types: vec![ContentType::Movie],
            catalogs: vec![],
            id_prefixes: None,
            icon: None,
            logo: None,
            background: None,
            behavior_hints: None,
        }
    }

    async fn catalog(&self, _req: CatalogRequest) -> CatalogResponse {
        CatalogResponse::default()
    }

    async fn meta(&self, _ty: ContentType, _id: &str) -> Option<MetaResponse> {
        None
    }

    async fn streams(&self, _ty: ContentType, _id: &str) -> StreamsResponse {
        StreamsResponse {
            streams: vec![Stream {
                name: Some("720p".into()),
                url: Some("https://example.test/d.m3u8".into()),
                ..Stream::default()
            }],
        }
    }
}

#[tokio::test]
async fn roundtrip_serve_and_consume() {
    let app = router(Arc::new(Hello));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();
    });

    let client = AddonClient::new(format!("http://{addr}")).unwrap();

    let manifest = client.manifest().await.unwrap();
    assert_eq!(manifest.id, "local.hello");
    assert_eq!(manifest.types, vec![ContentType::Movie]);

    let streams = client.streams(ContentType::Movie, "tt1").await.unwrap();
    assert_eq!(streams.streams.len(), 1);
    assert_eq!(streams.streams[0].name.as_deref(), Some("720p"));

    // 不存在的 meta 走 404 → 客户端报错。
    assert!(client.meta(ContentType::Movie, "missing").await.is_err());
}

#[tokio::test]
async fn manifest_serves_cors_headers() {
    let app = router(Arc::new(Hello));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();
    });

    let origin = "https://cineharbor.example";

    // 简单 GET：响应带 allow-origin（跨源 `fetch` 直连必需）。
    let response = reqwest::Client::new()
        .get(format!("http://{addr}/manifest.json"))
        .header("Origin", origin)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        response
            .headers()
            .get("access-control-allow-origin")
            .and_then(|value| value.to_str().ok()),
        Some("*")
    );

    // 预检 OPTIONS：204 + allow-origin。
    let preflight = reqwest::Client::new()
        .request(reqwest::Method::OPTIONS, format!("http://{addr}/manifest.json"))
        .header("Origin", origin)
        .header("Access-Control-Request-Method", "GET")
        .send()
        .await
        .unwrap();
    assert_eq!(preflight.status(), 204);
    assert_eq!(
        preflight
            .headers()
            .get("access-control-allow-origin")
            .and_then(|value| value.to_str().ok()),
        Some("*")
    );
}
