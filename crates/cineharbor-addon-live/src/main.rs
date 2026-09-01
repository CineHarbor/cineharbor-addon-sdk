//! live addon 独立进程入口：把多源 `LiveAddon` 暴露为 Stremio 兼容 HTTP 服务 + 媒体代理转链。
//!
//! 运行：`cargo run -p cineharbor-addon-live`，默认监听
//! `http://127.0.0.1:11472/manifest.json`（可用 `CINEHARBOR_ADDON_PORT` 覆盖）。
//! 源配置：
//! - 多源：`CINEHARBOR_LIVE_SOURCES` 指 JSON 文件
//!   `{"sources":[{"key","name","source","ua?","referer?"}]}`，`source` 为 http(s) URL 时远程
//!   拉取 M3U8，否则按本地文件路径读取。
//! - 单源（向后兼容）：`CINEHARBOR_LIVE_SOURCE` = URL 或本地文件（source key=m3u8）。
//! 均未设置或加载失败则回退内置演示列表。
//! stream url 经 `/media/live/{m3u8,segment,key}` 转链（`cineharbor-source=<source key>`）。

use std::collections::HashMap;
use std::sync::Arc;

use cineharbor_addon_live::{parse_m3u8, LiveAddon, LiveSource};
use cineharbor_addon_sdk::addon::router;
use cineharbor_media::{live_proxy_router, ProxyParts, SourceHeaders, DEFAULT_WEB_UA};

const DEMO_PLAYLIST: &str = "\
#EXTM3U
#EXTINF:-1 tvg-id=\"demo\" group-title=\"Demo\",Demo Channel
http://example.test/demo.m3u8
";

#[derive(serde::Deserialize, Default)]
struct LiveSourcesConfig {
    #[serde(default)]
    sources: Vec<LiveSourceConfig>,
}

#[derive(serde::Deserialize)]
struct LiveSourceConfig {
    key: String,
    name: String,
    source: String,
    #[serde(default)]
    ua: Option<String>,
    #[serde(default)]
    referer: Option<String>,
}

async fn load_playlist_text(source: &str) -> Option<String> {
    if source.starts_with("http://") || source.starts_with("https://") {
        return reqwest::get(source)
            .await
            .ok()?
            .error_for_status()
            .ok()?
            .text()
            .await
            .ok();
    }
    std::fs::read_to_string(source).ok()
}

async fn build_addon() -> (LiveAddon, HashMap<String, SourceHeaders>) {
    if let Ok(path) = std::env::var("CINEHARBOR_LIVE_SOURCES") {
        if let Ok(json) = std::fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str::<LiveSourcesConfig>(&json) {
                let mut sources = Vec::new();
                for sc in config.sources {
                    let Some(text) = load_playlist_text(&sc.source).await else {
                        continue;
                    };
                    sources.push(LiveSource {
                        key: sc.key,
                        name: sc.name,
                        ua: sc.ua,
                        referer: sc.referer,
                        channels: parse_m3u8(&text),
                    });
                }
                if !sources.is_empty() {
                    let headers = sources
                        .iter()
                        .map(|source| {
                            (
                                source.key.clone(),
                                SourceHeaders {
                                    ua: source.ua.clone(),
                                    referer: source.referer.clone(),
                                },
                            )
                        })
                        .collect::<HashMap<_, _>>();
                    return (LiveAddon::from_sources("live", sources), headers);
                }
            }
        }
    }

    if let Ok(source) = std::env::var("CINEHARBOR_LIVE_SOURCE") {
        if let Some(text) = load_playlist_text(&source).await {
            let src = LiveSource {
                key: "m3u8".into(),
                name: "live".into(),
                ua: None,
                referer: None,
                channels: parse_m3u8(&text),
            };
            return (LiveAddon::from_sources("live", vec![src]), HashMap::new());
        }
    }

    (LiveAddon::from_playlist("demo", DEMO_PLAYLIST), HashMap::new())
}

#[tokio::main]
async fn main() {
    let port = std::env::var("CINEHARBOR_ADDON_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(11472);

    let (mut addon, source_headers) = build_addon().await;
    let public_base = format!("http://127.0.0.1:{port}");
    addon.set_public_base_url(Some(public_base.clone()));

    let proxy_parts = Arc::new(ProxyParts {
        client: reqwest::Client::builder()
            .user_agent(DEFAULT_WEB_UA)
            .build()
            .expect("build proxy client"),
        sources: Arc::new(source_headers),
        public_base_url: public_base,
    });

    let app = router(Arc::new(addon)).merge(live_proxy_router(proxy_parts));
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .expect("bind live addon port");
    println!("live addon listening on http://127.0.0.1:{port}/manifest.json");
    axum::serve(listener, app.into_make_service())
        .await
        .expect("serve live addon");
}