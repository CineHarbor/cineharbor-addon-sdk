//! douban addon 独立进程入口：把 `DoubanAddon` 暴露为 Stremio 兼容 HTTP 服务。
//!
//! 运行：`cargo run -p cineharbor-addon-douban`，默认监听
//! `http://127.0.0.1:11471/manifest.json`（可用 `CINEHARBOR_ADDON_PORT` 覆盖）。

use std::sync::Arc;

use cineharbor_addon_douban::DoubanAddon;
use cineharbor_addon_sdk::addon::router;

#[tokio::main]
async fn main() {
    let port = std::env::var("CINEHARBOR_ADDON_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(11471);
    let addon = std::env::var("CINEHARBOR_DOUBAN_SEARCH_BASE_URL")
        .ok()
        .map(|base| base.trim().to_string())
        .filter(|base| !base.is_empty())
        .map(DoubanAddon::with_search_base_url)
        .unwrap_or_else(DoubanAddon::new);
    let app = router(Arc::new(addon));
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .expect("bind douban addon port");
    println!("douban addon listening on http://127.0.0.1:{port}/manifest.json");
    axum::serve(listener, app.into_make_service())
        .await
        .expect("serve douban addon");
}