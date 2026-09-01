# 2026-08-31 vod meta 类型自证（详情接线前置）

- 改 `cineharbor-addon-vod` `meta()`：`MetaDetail` 的 type 由抓取结果 `type_name` 自证
  （`content_type_for(&detail)`），不再信任请求路径 `ty`。这样 web 详情可统一 `detail('movie',
  vod:{site}:{vid})` 也能拿到正确的 movie/series 类型（详情请求本身不携带类型语义）。
- 验证：`cargo test -p cineharbor-addon-vod` 5 绿（vod_id_roundtrip / proxies / parses_config +2）+
  `cargo check --workspace` 干净。
- 剩余 `detail(...)` 接线两点 wrinkle（下轮处理，勿盲焊）：
  1. **source-key 语义**：原生 `SearchResult.source` = 站点 key（媒体代理 `?source=` 用），addon
     `buildDetail` 现用泛型 `'vod'`；需从 `vod:{source}:{vid}` 提取真实站点 key 覆写。
  2. **4 处消费**：`fetchContentDetail({source,id})` 被 play / DownloadsClient / follow-updates /
     downloadable 共用，接线要覆盖全部 + 不破坏 `episodes` 已转链 url 的下游（下载/prefetch）。