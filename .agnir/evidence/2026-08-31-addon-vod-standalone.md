# 2026-08-31 vod standalone addon 落地（P2 第四块）

- 新增 `crates/cineharbor-addon-vod`（lib + bin）：CustomAPI 视频站聚合 addon（catalog/meta/stream）。
  - 网络层（reqwest）：`search_site`/`search_all_sites`（JoinSet 多站 fan-out + 翻页）、
    `fetch_content_detail`/`fetch_json_detail`/`fetch_custom_detail`、`build_downstream_headers`。
  - `VodConfig`（JSON 加载 `sites`/`max_search_pages`）+ `VodAddon`（`Addon` trait）。
  - ID：`vod:{source}:{vid}`；content_type：名称含「剧/series/tv」→ Series，否则 Movie。
  - bin：`CINEHARBOR_VOD_SITES` 指 JSON 文件；默认 `127.0.0.1:11473`。
- `cineharbor-api` 增配：`ApiSite` 加 `Serialize/Deserialize`（`#[serde(default)]` 缺省字段），
  `pub use SearchResult`；`Cargo.toml` 加 serde。
- 验证（exit 0）：api 5 + vod 4 测试绿；`cargo build`；二进制冒烟 `/manifest.json`
  （`cineharbor.vod`，catalog/meta/stream，idPrefixes `vod`）与 `/catalog/movie/search/search=test.json`
  （空站点空结果）均 HTTP 200 不触网；`cargo check --workspace` 无警告。
- 修复：reqwest 无 `json` feature → `parse_json`（`.text()` + `serde_json::from_str`）。
- P2 剩余：媒体代理 addon（m3u8/vod/key/logo/image 转链）、local-service 删内置 douban/live/vod。