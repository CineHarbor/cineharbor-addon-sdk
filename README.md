# cineharbor-addon-sdk

CineHarbor 内容源 addon 协议与 SDK，对应 Stremio 的 `stremio-addon-sdk`。

- **协议契约**：见 [protocol.md](protocol.md)（完全跟随 Stremio addon 协议，双向互操）。
- crate：
  - `cineharbor-addon-protocol`：manifest/catalog/meta/stream/subtitles 类型与校验
  - `cineharbor-addon-sdk`：addon 构建与本地 host 接线

```bash
cargo check --workspace
```

## 许可证

CC BY-NC-SA 4.0
