# 2026-08-31 live addon 多源 parity（对齐 native LiveConfig[]）

- 改动 `cineharbor-addon-live`：`LiveAddon` 由单源 `channels` 改为 `sources: Vec<LiveSource>`，
  `LiveSource{key,name,ua,referer,channels}`；频道 id 由 `live:{idx}` 改为 `live:{key}:{idx}`；stream 转链按
  source key（`cineharbor-source=<key>`，可 per-source UA/Referer）。manifest 每源一个 catalog。
- main.rs 源配置：`CINEHARBOR_LIVE_SOURCES`=JSON `{"sources":[{"key","name","source","ua?","referer?"}]}`
  （`source` http(s)/本地文件）；单源 `CINEHARBOR_LIVE_SOURCE` 向后兼容（key=m3u8）；均可回退 demo。
  `ProxyParts.sources` 由 per-source ua/referer 填充。
- 验证（exit 0）：
  - `cargo test -p cineharbor-addon-live` 5 绿（parses / 每源 catalog / 按 key 过滤 / 流 / 转链）+ `cargo check --workspace`。
  - 实跑 curl（多源 JSON）：manifest 2 catalogs `[cctv, hunan]`、`catalog/tv/cctv` 2 频道、`catalog/tv/hunan` 1 频道、
    `stream/tv/live:cctv:1` → `/media/live/m3u8?cineharbor-source=cctv&url=...`。
- 联动：web `addon-live-client.ts` 增 `listSources()`/`listChannels(sourceKey)`；`addon-cross-origin-smoke.mjs`
  改用 `tv/m3u8` + `live:m3u8:0` 后仍 exit 0。
- 剩余外沿（非切断面阻塞）：tvg-id（EPG）、precheck、logo 代理。