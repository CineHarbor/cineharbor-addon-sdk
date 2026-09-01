# 2026-08-31 addon parity 复核（纠正早前低估）

- 实机复核发现早前「三个 addon 只是 PoC、parity 缺口大」的判断**过时/偏悲观**，逐 crate 核验 + live 实跑：
  - `cineharbor-addon-live`：**M3U 摄入已落地**（`CINEHARBOR_LIVE_SOURCE` = http(s) URL 或本地文件 →
    `parse_m3u8` → `catalog/tv/channels` → `meta`/`stream`(proxied)）。实跑（3 频道 M3U）：manifest
    `community.live/tv`、`catalog/tv/channels.json` 回 `CCTV-1/CCTV-5/湖南卫视`、`stream/tv/live:1` 回转链 URL。
    缺口（外沿）：单源（非多源数组）、`tvg-id` 未入 `Channel`（EPG 键）、EPG/precheck/logo 代理。
  - `cineharbor-addon-vod`：**多 CustomAPI 聚合已落地**（`VodConfig.sites: Vec<ApiSite>` + enabled_sites +
    search/detail/stream + `cineharbor-api` 纯解析 + 媒体代理转链）。缺口：分页/建议/成人过滤等打磨。
  - `cineharbor-addon-douban`：**真实搜索已落地**（`search.douban.com` HTML `window.__DATA__` 抓取 + 映射 →
    catalog）。缺口：ratings/recommends/categories 尚未 parity。
- 结论：**addon 核心 parity 基本到位**；真正剩余 = 页面级数据流切换（shape-bridge + 接线）+ 外沿功能
  （douban ratings、live EPG/多源）+ 退役 TS `/api`，而非「先补 addon parity」的大欠账。