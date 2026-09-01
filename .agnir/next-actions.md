# cineharbor-addon-sdk Next Actions

0. **提交并推送本次 Agnir 初始化**（`AGNIR.yaml` / `AGENTS.md` / `.agnir/` / README 段），当前均为未提交改动。

1. P2 抓取外置：douban ✅ / live ✅（**多源 M3U8** + 每源 catalog + per-source 转链）/ vod ✅（含媒体转链）/ 媒体代理 ✅（`cineharbor-media` serve 转链 router，mock 上游端到端测试）。跨源直连 CORS ✅（`router()` 挂 `Access-Control-Allow-Origin: *` + 预检 204）。续：local-service 移除内置版（P4，web 薄客户端切走后执行）；live 外沿（tvg-id/EPG/precheck/logo 代理）。
2. P3 起 `cineharbor-local-service`（core 仓）依赖本 SDK 作 addon host 接线；届时移除 local-service 内置 douban/live/vod。
3. 保持与 Stremio addon 协议双向互操，协议变化同步 `protocol.md`。
4. `cargo test --workspace --all-targets`。
