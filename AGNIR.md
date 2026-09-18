# Agnir Project Instructions

This file is the canonical Executor-facing activation and Project-operation surface for this Agnir-enabled Project. It is Project content, not Agnir Core state and not a replacement for `AGNIR.yaml` or `.agnir/` durable continuity.

## Activation

1. **Enter through the authorized Project Entry Point.** Treat this repository root, when supplied by the Principal or execution surface, as the discovery boundary. Do not guess parent, sibling, or neighboring Project roots.
2. **Discover.** Read top-level `AGNIR.yaml`. Validate the declared Core/profile compatibility, Project identity, selected logical Continuity Lineage, and any selector/binding independently. Dispatch according to the compatibility line actually declared.
3. **Load.** Load Current State and Next Actions from the declared selected continuity. Load Decisions and Evidence when they materially constrain the operation. Prefer durable Project truth over private conversational memory unless superseded by newer Principal intent or directly observed Project facts.
4. **Work.** Perform the actual Project task. Install, upgrade, migration, promotion, or repair must follow the Agnir distribution procedure pinned by the provenance recorded in `AGNIR.yaml`.

## Checkpoint

At an intentional checkpoint, save-progress, finish, or repository commit boundary:

1. reconcile Project truth rather than the transcript;
2. classify only material continuity changes for the selected lineage;
3. treat unchanged durable truth as a checkpoint no-op;
4. construct the complete coherent checkpoint candidate before publication when material truth changed;
5. reject stale publication with `AGNIR_CHECKPOINT_CONFLICT`, then re-resolve and reconcile rather than overwriting newer truth;
6. fresh-resolve the same Project identity and selected lineage after publication so a fresh Executor can resume.

Checkpoint evaluation is mandatory at the boundary; mutation of `.agnir/` is not. Never manufacture State or Evidence changes only to make a commit appear checkpointed.

## Repository operations

- `commit`, `提交`, or `提交代码` in repository context means: **checkpoint evaluation → Project-defined pre-commit policy when declared → commit**.
- `commit and push` or `提交推送` means: **checkpoint evaluation → Project-defined pre-commit policy when declared → commit → push → verify the actual destination ref**.
- Do not execute an isolated Git commit path that bypasses checkpoint evaluation.
- Prefer one coherent VCS revision for Project changes and material Agnir checkpoint changes. A legitimate checkpoint no-op does not require an `.agnir/` diff.

## Continuity Lineage integration

For Core `0.2`/`1.0` parallel continuity, source continuity is reconciliation input, not automatic target truth. Validate source and target Project identity, reconcile the target against the integrated Project result, and publish the integrated Project plus the reconciled target checkpoint coherently.

## Compatibility locator

`README.md#Agnir-Project-Instructions` is retained only as a backward-compatible locator for older Agnir `1.0.0` activation paths. It points here and must not contain a second copy of this procedure.
