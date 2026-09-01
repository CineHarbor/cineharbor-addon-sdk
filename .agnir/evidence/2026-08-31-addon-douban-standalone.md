# 2026-08-31 douban 抓取 standalone addon 落地（P2 首块）

- 新增 `crates/cineharbor-addon-douban`（lib + bin）：豆瓣搜索 catalog addon，**self-contained 自抓取，无 local-service 依赖**。
  - lib：`DoubanAddon`（自建 reqwest client + UA/超时）+ `search()`（拉 `search.douban.com` 的 `window.__DATA__` HTML）+ 纯解析/映射（regex 抽年份、标题清洗、tpl 主体过滤、play_type 推断）+ `Addon` trait 实现（catalog/search，id 前缀 `douban:`）。
  - bin：`src/main.rs` 用 `cineharbor-addon-sdk::addon::router` 暴露 HTTP，默认 `127.0.0.1:11471`（`CINEHARBOR_ADDON_PORT` 覆盖）。
- 验证（exit 0）：`cargo test -p cineharbor-addon-douban`（3 离线 fixture 测试，无警告）；`cargo build -p cineharbor-addon-douban`；二进制冒烟：`/manifest.json` HTTP 200（`id=cineharbor.douban`），`/catalog/movie/search.json`（无 extra）HTTP 200 空结果不触网。
- strangler 关系：与 `cineharbor-local-service` 内置 `BuiltinDoubanAddon` 逻辑对齐，并行阶段；后续移除 local-service 内置版，改由本 addon 独立部署。
- 后续 P2：live / vod 抓取 addon、媒体代理 addon（m3u8/vod/key/logo/image）。