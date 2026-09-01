# 2026-08-31 standalone addon 跨源直连（CORS）

- 需求：web 薄客户端经 WASM 内 `fetch` 直连 standalone addon 属跨源请求，必需 CORS 头（否则浏览器
  拦截，P3.c「addon HTTP 直连」无法落）。
- 改动：`cineharbor-addon-sdk/src/addon.rs` 的 `router()` 挂 `axum::middleware::from_fn` CORS 中间件
  —— 响应补 `Access-Control-Allow-Origin: *` + `Allow-Methods: GET,HEAD,OPTIONS` + `Allow-Headers: *` +
  `Expose-Headers: *`；`OPTIONS` 预检直接回 204。媒体代理 `serve.rs` 原有 allow-origin 未变。
  （axum 0.8 `Next`/`Request` 非泛型：`async fn cors_middleware(request: Request, next: Next)`。）
- 验证（exit 0）：
  - `cargo test -p cineharbor-addon-sdk`：新 `manifest_serves_cors_headers`（GET 200 + allow-origin、OPTIONS 204）。
  - `cargo check --workspace`：全 addon 用共享 `router()` 编译通过。
  - 真实 `cineharbor-addon-live`（cargo run + curl）：`/manifest.json` 200 且
    `access-control-allow-origin: *` / methods / headers / expose；OPTIONS 204。
- 结论：standalone addon 的 Stremio 端点可被浏览器跨源 fetch，web 薄客户端「addon HTTP 直连」解挡。