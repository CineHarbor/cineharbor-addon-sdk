# 2026-08-31 live addon 媒体转链接线（P2 收尾）

- `cineharbor-addon-live` 接线 `live_proxy_router`（与 vod addon 对称）：
  - `LiveAddon` 增 `public_base_url` + `set_public_base_url` + `stream_url()`（配置后经
    `build_live_proxy_m3u8_url(base, "m3u8", url, false)` 转链，`cineharbor-source=m3u8`；未配置直链）。
  - `main.rs`：`router(addon).merge(live_proxy_router(ProxyParts{client,sources:{},public_base}))`，
    缺省 `public_base = http://127.0.0.1:11472`。
- 验证（exit 0）：`cineharbor-addon-live` 3 测试绿（新 `proxies_stream_url_when_configured`）；冒烟
  manifest 200 / catalog 200（meta id `live:0`）/ `/media/live/m3u8`（无参数）400（代理路由已挂）/
  `/stream/tv/live:0.json` → `{"url":"http://127.0.0.1:11472/media/live/m3u8?cineharbor-source=m3u8&url=http%3A%2F%2Fexample.test%2Fdemo.m3u8"}`。
- 至此 P2 抓取 + 媒体代理 addon 全部独立可跑（douban / live / vod + `cineharbor-media` serve）。
  剩 P2 唯一尾巴 = local-service 删内置版（与 P4 退役强相关，web 薄客户端切走后再执行）。