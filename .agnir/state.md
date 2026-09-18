# cineharbor-addon-sdk Current State

CineHarbor 内容源 addon 协议与 SDK，对应 Stremio `stremio-addon-sdk`。

- 协议完全跟随 Stremio addon 协议，双向互操；契约见 `protocol.md`。
- crates：`cineharbor-addon-protocol`（类型与校验）、`cineharbor-addon-sdk`（`AddonClient` + `Addon` trait/`router`，`router()` 内置 permissive CORS 供浏览器跨源直连）、`cineharbor-addon-bangumi`（bgm.tv 动画）、`cineharbor-addon-live`（多源 M3U8 直播，standalone + bin + 媒体转链，每源一 catalog）、`cineharbor-addon-douban`（豆瓣搜索，standalone + bin，自抓取）、`cineharbor-api`（vod 抓取共享核心，纯逻辑）、`cineharbor-addon-vod`（CustomAPI 聚合，standalone + bin + 媒体转链）、`cineharbor-media`（媒体代理：HLS 重写纯逻辑 + native 转链 serve）。
- 完整示例：`cargo run --example hello`。
- 许可证：CC BY-NC-SA 4.0。
- Agnir 操作基线：`iorLab/agnir` 稳定发布 `v1.0.2`（revision `b5626394ec40a5cb7a28c01892acde07cc0adc8e`，distribution `agnir-agent-skill`）；Core/Profile 保持 `1.0` / `repository-filesystem/1.0`，2026-09-19 完成 compatible operational upgrade，canonical 激活路径为 `AGENTS.md → AGNIR.md → AGNIR.yaml`；既有 Project identity、lineage 与 durable memory 保持不变。
