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
