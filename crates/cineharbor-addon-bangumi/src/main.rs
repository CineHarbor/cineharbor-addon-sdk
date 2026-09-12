//! bangumi addon 独立进程入口：把 `BangumiAddon` 暴露为 Stremio 兼容 HTTP 服务。
//!
//! 运行：`cargo run -p cineharbor-addon-bangumi`，默认监听
//! `http://127.0.0.1:11474/manifest.json`（可用 `CINEHARBOR_ADDON_PORT` 覆盖）。

use std::sync::Arc;

use cineharbor_addon_bangumi::BangumiAddon;
use cineharbor_addon_sdk::addon::router;

#[tokio::main]
async fn main() {
    let port = std::env::var("CINEHARBOR_ADDON_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(11474);
    let app = router(Arc::new(BangumiAddon::new()));
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .expect("bind bangumi addon port");
    println!("bangumi addon listening on http://127.0.0.1:{port}/manifest.json");
    axum::serve(listener, app.into_make_service())
        .await
        .expect("serve bangumi addon");
}
