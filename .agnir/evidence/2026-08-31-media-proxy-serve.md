# 2026-08-31 媒体代理 native 转链服务接线（P2 第六块）

- `cineharbor-media` 增 `src/serve.rs`（native，axum + reqwest，依赖增 axum/reqwest/serde）：
  - `vod_proxy_router` / `live_proxy_router`（`/media/{vod,live}/{m3u8,segment,key}`）。
  - `ProxyParts`（`client` + `sources: HashMap<source, SourceHeaders{ua,referer}>` + `public_base_url`）。
  - m3u8：抓上游（带 source UA/Referer）→ `rewrite_*_manifest_content` → 回 `application/vnd.apple.mpegurl` + CORS。
  - segment/key：抓上游字节 → 回 `application/octet-stream` + CORS（暂全量读字节，非流式；与 local-service
    的流式/磁盘缓存/广告过滤/identity-encoding 回退相异的本地优化未迁，见 serve.rs 顶注）。
- 验证（exit 0）：`cineharbor-media` 9 测试全绿，含 2 个 **mock 上游端到端** test（本地 axum 起 fixture
  m3u8 → 经代理路由 → 断言重写后 segment/key 指向代理端点 + 200 + content-type）。
- `cineharbor-addon-vod` 接线：`VodConfig.public_base_url` + `stream_url()`（配置后 stream/meta 播单 url
  经 `build_vod_proxy_m3u8_url` 转链，未配置直链）+ `main.rs` `router(addon).merge(vod_proxy_router(...))`。
  验证：vod 5 测试绿；冒烟 `/manifest.json` 200、`/catalog` 200、`/media/vod/m3u8`（无参数）返回
  `missing source or url`（证明代理路由已挂）。
- P2 剩余：live addon 接线 `live_proxy_router`（复用 serve）；local-service 删内置 douban/live/vod/代理。