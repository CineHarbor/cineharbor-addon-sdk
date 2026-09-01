# Evidence — Agnir compatible operational upgrade to v0.1.0（2026-09-01）

- 操作：将 `CineHarbor/cineharbor-addon-sdk` 已应用的 Agnir 操作包升级到最新稳定发布。
- 目标解析：最新稳定版 = 已发布标签 `v0.1.0`（commit `2a0cb7bf2068b11f361e315670b2f2dc497b2588`，source `iorLab/agnir`）。未把 `main` 当作稳定版。
- 先前基线：Agnir Core `0.1` / `repository-filesystem/0.1`，2026-08-31 初始化，未记录 operational provenance。
- 分类：compatible operational upgrade —— Core 兼容线 `0.1`、profile `repository-filesystem/0.1` 均未变；非迁移、非重新初始化。
- 保留项：`project.identity`（`urn:cineharbor:project:cineharbor-addon-sdk`）、四个 memory locator、`.agnir/*` 持久内存、`agnir/repository` 扩展及无关 README/`AGENTS.md` 内容均保留。
- 变更项：
  - `AGNIR.yaml` 增加 `extensions.agnir/operations`（distribution / release / source / applied_revision）记录已应用操作包。
  - README `## Agnir Project Instructions` 追加 commit-boundary 第 6 条（commit 请求 = checkpoint 边界，非破坏合并）。
  - `.agnir/decisions.md` 新增 2026-09-01 升级决策；`.agnir/state.md` 记录操作基线。
- 验证：冷启动激活链路（Project root → `AGENTS.md` → README `## Agnir Project Instructions` → `AGNIR.yaml` → `.agnir/` memory）重新解析通过（fresh activation）。