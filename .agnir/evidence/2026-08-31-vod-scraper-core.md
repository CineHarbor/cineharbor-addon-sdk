# 2026-08-31 vod 抓取共享核心 `cineharbor-api` 落地（P2 第三块）

- 新增 `crates/cineharbor-api`：vod 抓取的**纯逻辑共享核心**（无 IO，可离线单测），供 standalone
  vod addon 与（后续）local-service 共用（ADR-0007）。
  - `ApiSite` DTO；`parse_search_payload`/`parse_search_item`/`parse_detail_payload`/
    `parse_custom_detail_html`（含 ffzy/feifan 专用 m3u8 规则）；`extract_episodes_from_play_url`；
    `build_collection_api_url`（wrapped-target 判定 + query 编码）；值规整/标题/HTML/年份/ID 校验。
  - 依赖：`cineharbor-core`（复用 `model::SearchResult`，跨仓 path）、serde_json、regex、url、html-escape。
- 验证（exit 0）：`cargo test -p cineharbor-api` 5 passed（search/detail/HTML/episodes/URL，全离线）；
  `cargo check --workspace`（addon-sdk）无警告。
- 与 local-service 内置版并行（strangler）；本地未删，待切依赖后移除。
- 后续：网络层（`search_site`/`fetch_content_detail`，reqwest）→ standalone `cineharbor-addon-vod`
  （Addon trait + bin）→ local-service 删内置 vod → 媒体代理 addon。