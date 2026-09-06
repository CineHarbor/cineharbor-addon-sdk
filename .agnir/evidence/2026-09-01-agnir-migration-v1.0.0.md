# Evidence — Agnir 兼容线迁移到 v1.0.0（2026-09-01）

- 操作：将 `CineHarbor/cineharbor-addon-sdk` 从 Core `0.1` / `repository-filesystem/0.1`（操作包 `v0.1.0`）迁移到最新稳定版。
- 授权：Principal 明确选择迁移目标 `v1.0.0`（iorLab/agnir tag `v1.0.0`，revision `6d16dcfd17b8e9f22fd25804e22b9f8a516d06c3`）；未把 `main` 当稳定版。
- 路径：按已发布契约组合执行 `CORE_0_1_TO_0_2_MIGRATION`（0.1 → 0.2，隐式连续线显式化为初始 lineage）→ `CORE_0_2_TO_1_0_PROMOTION`（0.2 → 1.0，语义保持晋升）。
- 兼容线变更：`agnir.version: "0.1"` → `"1.0"`；`discovery_profile: "repository-filesystem/0.1"` → `"repository-filesystem/1.0"`；新增 `continuity.lineage: "urn:cineharbor:lineage:cineharbor-addon-sdk"`。
- 保留：`project.identity`（`urn:cineharbor:project:cineharbor-addon-sdk`）、四个 memory locator 与 durable memory 内容、policy、`agnir/repository` 扩展、无关 README/`AGENTS.md` 内容；memory 未迁移位置。
- 其他变更：`agnir/operations` 更新为 release `1.0.0` / applied_revision `6d16dcf…`；README 第 1 条增加兼容线/identity/lineage 校验要求；decisions/state 记录本次迁移。
- 验证：`AGNIR.yaml` 通过 `agnir-manifest-1.0.schema.json` 语义校验；冷启动激活链路（Project root → `AGENTS.md` → README `## Agnir Project Instructions` → `AGNIR.yaml` → `.agnir/` memory）fresh 解析通过。