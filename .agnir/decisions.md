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
