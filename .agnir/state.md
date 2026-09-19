# cineharbor-addon-sdk Current State

Target: CineHarbor **1.0.0 public release**, with hard release gates enforced. **RELEASE_READY = false; PUBLIC_RELEASE_EXECUTED = false.** Scope and acceptance rules live in `CineHarbor/cineharbor/docs/releases/1.0.0/`.

## Repository boundary

This workspace owns the Stremio-compatible protocol, SDK/router/client, standalone Douban, Bangumi, Live and VOD addons, shared content API parser and media proxy. Remote addons are the ADR-0006 content data plane.

## Verified inputs

Main `e5f7a3a289ceb978d559910b5bf9f176809ada04` passed the complete SDK CI matrix twice: push run `35446032830` and clean workflow-dispatch run `35446059543`. It includes deterministic Bangumi upstream injection while preserving bgm.tv as the production default.

Core release source `d51414bba4964dd7cee02780e41c73e35190ce76` has separately passed its final main matrix twice.

## 1.0.0 version alignment

The release branch changes the SDK workspace package version to `1.0.0`, pins Core to verified `d51414bba4964dd7cee02780e41c73e35190ce76`, updates all eight owned workspace lock entries plus the pinned Core path entry to `1.0.0`, and aligns standalone Bangumi, Douban, Live and VOD manifest versions to `1.0.0`.

Protocol examples/tests already advertise `1.0.0`. Complete PR CI and two post-merge main runs are required before Web/Desktop pins move.

Media authorization/SSRF, production deployment smoke and final security/license review remain separate release obligations.

See `.agnir/evidence/2026-09-19-version-1.0.0.md`.

Project `urn:cineharbor:project:cineharbor-addon-sdk`; lineage `urn:cineharbor:lineage:cineharbor-addon-sdk`. Agnir Core/Profile 1.0 / repository-filesystem/1.0 and operations v1.0.2 remain unchanged. License: CC-BY-NC-SA-4.0.
