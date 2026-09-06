# Agnir Decisions

## 2026-08-31 — Agnir initialization

- 本仓库以 `CineHarbor/cineharbor-addon-sdk` 作为 Project 身份，identity `urn:cineharbor:project:cineharbor-addon-sdk`。
- 采用 `repository-filesystem/0.1`，durable memory 落于 `.agnir/`；`AGNIR.yaml` 为 discovery anchor；根 `AGENTS.md` 仅作 locator；README `## Agnir Project Instructions` 为 canonical activation instruction。

## 2026-08-31 — 既有 Project 决策（源自 README）

- 协议完全跟随 Stremio addon 协议，双向互操。

## 2026-09-01 — Agnir 兼容操作升级到 v0.1.0

- 升级已应用的 Agnir 操作包到稳定发布 `v0.1.0`（source `iorLab/agnir`，immutable revision `2a0cb7bf2068b11f361e315670b2f2dc497b2588`）。
- 分类：compatible operational upgrade —— Core 兼容线仍为 `0.1`，profile 仍为 `repository-filesystem/0.1`；`project.identity`、memory locators、durable memory 内容与 `agnir/repository` 扩展均保留。
- 变更：`AGNIR.yaml` 增加 `extensions.agnir/operations` 操作出处；README `## Agnir Project Instructions` 追加 commit-boundary 规则；state.md 记录操作基线；新增升级证据文件。

## 2026-09-01 — Agnir 兼容线迁移到 1.0（Principal 授权）

- 按已发布契约 `CORE_0_1_TO_0_2_MIGRATION` + `CORE_0_2_TO_1_0_PROMOTION` 组合执行：0.1 → 0.2（隐式连续线显式化为初始 lineage）→ 0.2 → 1.0（语义保持晋升）。
- 授权与目标：Principal 明确选择迁移到最新稳定版 `v1.0.0`（iorLab/agnir tag `v1.0.0`，revision `6d16dcfd17b8e9f22fd25804e22b9f8a516d06c3`）。
- 兼容线声明改为 Core `1.0` / `repository-filesystem/1.0`；新增 `continuity.lineage: "urn:cineharbor:lineage:cineharbor-addon-sdk"`（0.1 唯一隐式连续线的显式初始 lineage）。
- 保留：`project.identity`、memory locators 与 durable memory 内容、policy、`agnir/repository` 扩展、README/`AGENTS.md` 无关内容。
