# 2026-08-31 vod `skip` 分页（Stremio 协议）+ 确定性排序

- `cineharbor-addon-vod` `catalog`：honor `req.skip`（`Option<u32>`）——`search_all_sites` 结果切片
  `.skip(skip).take(SEARCH_PAGE_SIZE=50)`。窗口受 `max_search_pages × 站点数` 上界约束（每站点
  max_search_pages 页已抓取），远端深分页需上游 page-offset 支持。
- `search_all_sites` 加确定性排序（`source,id`）：JoinSet 汇合顺序不稳定，跨 `skip` 分页要求稳定次序；
  相关性排序/去重归聚合侧（既有口径）。
- 验证：`cargo test -p cineharbor-addon-vod` 5 绿 + `cargo check --workspace` 干净。
- 闭合点播外沿「分页」项（adon 侧）。剩外沿：vod 建议（suggest）、live EPG/logo、douban rating 槽位。