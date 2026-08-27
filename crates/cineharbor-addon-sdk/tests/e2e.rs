//! 端到端：供给侧 [`router`] 服务 ↔ 消费侧 [`AddonClient`] 拉取，走真实回环 HTTP。

use std::sync::Arc;

use async_trait::async_trait;
use cineharbor_addon_protocol::{
    CatalogResponse, ContentType, Manifest, MetaResponse, Resource, Stream, StreamsResponse,
};
use cineharbor_addon_sdk::addon::{router, Addon, CatalogRequest};
use cineharbor_addon_sdk::AddonClient;

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
        axum::serve(listener, app.into_make_service()).await.unwrap();
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