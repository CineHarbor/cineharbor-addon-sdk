# cineharbor-addon-sdk Next Actions

0. **提交并推送本次 Agnir 初始化**（`AGNIR.yaml` / `AGENTS.md` / `.agnir/` / README 段），当前均为未提交改动。

1. P3 起 `cineharbor-local-service`（core 仓）将依赖本 SDK 作 addon host 接线。
2. 保持与 Stremio addon 协议双向互操，协议变化同步 `protocol.md`。
3. `cargo test --workspace --all-targets`。
