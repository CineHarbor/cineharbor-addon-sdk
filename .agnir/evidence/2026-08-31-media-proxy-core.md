# 2026-08-31 媒体代理共享纯核心 `cineharbor-media`（P2 第五块）

- 新增 `crates/cineharbor-media`：媒体代理的**纯逻辑共享核心**（无 IO，离线单测），
  与 local-service `lib.rs` 的 `rewrite_vod_manifest_content`/`rewrite_live_manifest_content`/
  `build_vod_proxy_*`/`build_live_proxy_*` 逐行对齐。
  - VOD：`rewrite_vod_manifest_content`（STREAM-INF/MEDIA/KEY/SESSION-KEY/MAP/PART/PRELOAD-HINT
    与裸 URI 分类重写）、`sanitize_vod_manifest_lines`（`video/adjump/` 广告跳段清理）、
    `build_vod_proxy_{m3u8,segment,key}_url`（`source` 参数）。
  - Live：`rewrite_live_manifest_content`（`cineharbor-source` 参数、`allowCORS` 透传）、
    `build_live_proxy_{m3u8,segment,key}_url`。
  - URL：`get_base_url`/`resolve_url`/`fallback_resolve_url`/`extract_quoted_attribute`/
    `looks_like_manifest_url`。
- 依赖：url、regex（纯，无 reqwest/axum）。
- 验证（exit 0）：`cargo test -p cineharbor-media` 7 passed（vod/live master+media 重写、
  key/segment/map 代理、adjump 清洗、URL 解析、编码）；`cargo check --workspace` 无警告。
- strangler：local-service 仍存自身拷贝（`vod_proxy.rs`/`live_proxy.rs` handler + lib.rs 重写函数），
  并行；后续由 standalone vod/live addon 依赖本 crate 并自挂 HTTP 代理 handler（fetch/流式/CORS），
  再删 local-service 内置代理。
- P2 剩余：媒体代理 HTTP handler 挂 addon（reqwest fetch/流式/range/CORS + 清单重写接线）、
  local-service 删内置 douban/live/vod/代理。