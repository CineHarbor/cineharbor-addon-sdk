# cineharbor-addon-sdk

CineHarbor 内容源 addon 协议与 SDK，对应 Stremio 的 `stremio-addon-sdk`。

- **协议契约**：见 [protocol.md](protocol.md)（完全跟随 Stremio addon 协议，双向互操）。
- crate：
  - `cineharbor-addon-protocol`：manifest/catalog/meta/stream/subtitles 类型与校验
  - `cineharbor-addon-sdk`：消费侧 `AddonClient` + 供给侧 `Addon` trait / `router`
  - `cineharbor-addon-bangumi`：bgm.tv 动画元数据参考 addon（catalog/meta）
  - `cineharbor-addon-live`：M3U8 直播参考 addon（catalog/meta/stream）

## 用法

```rust
use std::sync::Arc;
use cineharbor_addon_sdk::addon::{router, Addon, CatalogRequest};
use cineharbor_addon_protocol::{Manifest, /* ... */};

struct MyAddon;
impl Addon for MyAddon {
    fn manifest(&self) -> Manifest { /* ... */ }
    fn catalog(&self, req: CatalogRequest) -> CatalogResponse { /* ... */ }
    // meta()/streams()/subtitles() 视需要实现
}

let app = router(Arc::new(MyAddon)); // axum::Router，交给 local-service（P3）host
```

消费任意 Stremio 兼容 addon：

```rust
use cineharbor_addon_sdk::AddonClient;
let client = AddonClient::new("https://addon.example.test")?;
let manifest = client.manifest().await?;
let streams = client.streams(ContentType::Movie, "tt0133093").await?;
```

完整可运行示例：`cargo run --example hello`。

```bash
cargo test --workspace --all-targets
```

## 许可证

CC BY-NC-SA 4.0

## Agnir Project Instructions

本项目使用 **Agnir**（project-owned durable continuity protocol）持久保存可恢复的 Project 连续性，本仓库根目录是已授权的 Project Entry Point。开始任何 Project 工作前：

1. 读取顶层 `AGNIR.yaml`；
2. 加载 Current State（`.agnir/state.md`）与 Next Actions（`.agnir/next-actions.md`）；
3. 需要时再加载 Decisions（`.agnir/decisions.md`）与 Evidence（`.agnir/evidence/`）；
4. durable Agnir Project truth 优先于聊天记录与 Agent 私有记忆，除非被更新的 Principal 指令或直接观测到的当前 Project 事实覆盖；
5. 在保存进度、checkpoint 或结束工作时，把重要的 state / next-action / decision / evidence 变更写回 `AGNIR.yaml` 声明的 durable memory 位置。
