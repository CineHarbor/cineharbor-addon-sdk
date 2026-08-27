//! 最小参考 addon：实现 [`Addon`] trait，用 [`router`] 暴露为 HTTP 服务。
//!
//! 运行：`cargo run --example hello`，监听 `http://127.0.0.1:11470/manifest.json`。

use std::sync::Arc;

use async_trait::async_trait;
use cineharbor_addon_protocol::{
    Catalog, CatalogResponse, ContentType, Manifest, MetaDetail, MetaPreview, MetaResponse,
    Resource, Stream, StreamsResponse, Video,
};
use cineharbor_addon_sdk::addon::{Addon, CatalogRequest, router};

struct Hello;

#[async_trait]
impl Addon for Hello {
    async fn manifest(&self) -> Manifest {
        Manifest {
            id: "local.hello".into(),
            version: "1.0.0".into(),
            name: "Hello CineHarbor".into(),
            description: Some("参考 addon：演示 Stremio 兼容供给侧".into()),
            resources: vec![Resource::Catalog, Resource::Meta, Resource::Stream],
            types: vec![ContentType::Movie, ContentType::Series],
            catalogs: vec![Catalog {
                r#type: ContentType::Movie,
                id: "hello".into(),
                name: "Hello".into(),
                extra: vec![],
                extra_supported: vec!["search".into()],
            }],
            id_prefixes: None,
            icon: None,
            logo: None,
            background: None,
            behavior_hints: None,
        }
    }

    async fn catalog(&self, _req: CatalogRequest) -> CatalogResponse {
        CatalogResponse {
            metas: vec![MetaPreview {
                poster: Some("https://example.test/poster.jpg".into()),
                description: Some("Hello addon 的示例条目".into()),
                year: Some("2024".into()),
                genres: vec!["demo".into()],
                ..MetaPreview::new("tt-demo", ContentType::Movie, "Demo Movie")
            }],
        }
    }

    async fn meta(&self, _ty: ContentType, id: &str) -> Option<MetaResponse> {
        if id != "tt-demo" {
            return None;
        }
        Some(MetaResponse {
            meta: MetaDetail {
                genres: vec!["demo".into()],
                poster: Some("https://example.test/poster.jpg".into()),
                description: Some("Hello addon 的示例条目".into()),
                year: Some("2024".into()),
                videos: vec![Video {
                    id: "tt-demo".into(),
                    name: "Demo Movie".into(),
                    ..Video::default()
                }],
                ..MetaDetail::new(id, ContentType::Movie, "Demo Movie")
            },
        })
    }

    async fn streams(&self, _ty: ContentType, _id: &str) -> StreamsResponse {
        StreamsResponse {
            streams: vec![Stream {
                name: Some("720p".into()),
                title: Some("hello addon".into()),
                url: Some("https://example.test/demo.m3u8".into()),
                ..Stream::default()
            }],
        }
    }
}

#[tokio::main]
async fn main() {
    let app = router(Arc::new(Hello));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:11470")
        .await
        .expect("bind 11470");
    println!("Hello addon listening on http://127.0.0.1:11470/manifest.json");
    axum::serve(listener, app.into_make_service())
        .await
        .expect("serve hello addon");
}
