# cineharbor-addon-sdk Current State

CineHarbor 内容源 addon 协议与 SDK，对应 Stremio `stremio-addon-sdk`。

- 协议完全跟随 Stremio addon 协议，双向互操；契约见 `protocol.md`。
- crates：`cineharbor-addon-protocol`（类型与校验）、`cineharbor-addon-sdk`（`AddonClient` + `Addon` trait/`router`）、`cineharbor-addon-bangumi`（bgm.tv 动画）、`cineharbor-addon-live`（M3U8 直播）。
- 完整示例：`cargo run --example hello`。
- 许可证：CC BY-NC-SA 4.0。
