//! vod addon 独立进程入口：暴露 Stremio 兼容 HTTP 服务 + 媒体代理转链服务。
//!
//! 运行：`cargo run -p cineharbor-addon-vod`，默认监听
//! `http://127.0.0.1:11473/manifest.json`（`CINEHARBOR_ADDON_PORT` 覆盖）。
//! 站点配置：`CINEHARBOR_VOD_SITES` 指 JSON 文件
//! （`{"sites":[{"key","name","api","detail?","ua?","referer?","disabled?","disable_ad_filter?"}],"max_search_pages":3,"public_base_url":"http://host:port"}`）。
//! stream/meta 播单 url 经 `/media/vod/{m3u8,segment,key}` 转链（`public_base_url` 缺省为
//! `http://127.0.0.1:{port}`）。

use std::collections::HashMap;
use std::sync::Arc;

use cineharbor_addon_sdk::addon::router;
use cineharbor_addon_vod::{VodAddon, VodConfig};
use cineharbor_media::{DEFAULT_WEB_UA, ProxyParts, SourceHeaders, vod_proxy_router};

fn load_config() -> VodConfig {
    let Some(path) = std::env::var("CINEHARBOR_VOD_SITES").ok() else {
        return VodConfig::default();
    };
    match std::fs::read_to_string(&path) {
        Ok(json) => VodConfig::from_json(&json).unwrap_or_default(),
        Err(_) => VodConfig::default(),
    }
}

#[tokio::main]
async fn main() {
    let port = std::env::var("CINEHARBOR_ADDON_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(11473);

    let mut config = load_config();
    let public_base = config
        .public_base_url
        .clone()
        .unwrap_or_else(|| format!("http://127.0.0.1:{port}"));

    let sources = Arc::new(
        config
            .sites
            .iter()
            .map(|site| {
                (
                    site.key.clone(),
                    SourceHeaders {
                        ua: site.ua.clone(),
                        referer: site.referer.clone(),
                        disable_ad_filter: site.disable_ad_filter,
                    },
                )
            })
            .collect::<HashMap<_, _>>(),
    );
    let access_token = std::env::var("CINEHARBOR_MEDIA_PROXY_TOKEN")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let proxy_parts = Arc::new(ProxyParts {
        client: reqwest::Client::builder()
            .user_agent(DEFAULT_WEB_UA)
            .build()
            .expect("build proxy client"),
        sources,
        public_base_url: public_base.clone(),
        access_token,
    });

    config.public_base_url = Some(public_base);
    let app = router(Arc::new(VodAddon::new(config))).merge(vod_proxy_router(proxy_parts));

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .expect("bind vod addon port");
    println!("vod addon listening on http://127.0.0.1:{port}/manifest.json");
    axum::serve(listener, app.into_make_service())
        .await
        .expect("serve vod addon");
}
