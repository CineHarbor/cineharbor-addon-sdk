# 2026-08-31 live addon standalone（P2 第二块）

- `cineharbor-addon-live` 加独立进程入口 `src/main.rs`：复用既有 `LiveAddon`（M3U8 解析）经 `router()` 暴露 HTTP，默认 `127.0.0.1:11472`（`CINEHARBOR_ADDON_PORT` 覆盖）。
- 频道源：`CINEHARBOR_LIVE_SOURCE` 为 http(s) URL 时远程拉取 M3U8，否则按本地文件路径读取；未设置/失败回退内置演示列表。
- `Cargo.toml` 增加 `axum`/`tokio`（bin 依赖）。
- 验证（exit 0）：`cargo test -p cineharbor-addon-live`（2 测试）；`cargo build`；二进制冒烟 `/manifest.json`（`community.live`）、`/catalog/tv/channels.json`（1 演示频道）、`/stream/tv/live:0.json` 均 HTTP 200，不触网。
- P2 剩余：vod 抓取 addon（multi-site 配置系统，跨多轮）、媒体代理 addon、local-service 移除内置版。