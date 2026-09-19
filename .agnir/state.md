# cineharbor-addon-sdk Current State

Target: CineHarbor **1.0.0**, release preparation in progress; **RELEASE_READY = false**. Current scope and acceptance rules are in `CineHarbor/cineharbor/docs/releases/1.0.0/`.

## Repository boundaries

This workspace owns the Stremio-compatible protocol, SDK/router/client, standalone Douban, Bangumi, Live and VOD addons, the shared content API parser and media proxy. Remote addons are the ADR-0006 content data plane; pure state/native/WASM bridges remain in cineharbor-core. Existing protocol fixtures and addon/media tests must remain enforced during release work.

VOD depends on cineharbor-api, which consumes pure-core model types through a sibling Cargo path. The former CI did not materialize that Core repository. `ci/dependency.json` now pins the required Core revision; `scripts/ci-checkout.py` creates and verifies the documented sibling layout. Rust is pinned to 1.98.1. All owned workspace members are format-checked without formatting sibling dependencies. Independent fmt/check/test/clippy lanes prevent one failed lane from hiding another. Deterministic formatting maintenance commits only owned Rust formatting and its evidence, verifies the destination ref and dispatches CI for the published revision.

## Validation status

Baseline inspected: `2a0a8929d8b363b75c27b85eec1dc767f91917aa`. Source inspection confirms the missing cross-repository checkout; Core's baseline formatting log also reports SDK formatting drift. New gates and normalization require actual Actions execution before any success claim. Protocol/CORS/media-auth/security/production deployment and final version alignment remain release gates; prepared workflows are not acceptance evidence.

## Continuity

Project `urn:cineharbor:project:cineharbor-addon-sdk`; lineage `urn:cineharbor:lineage:cineharbor-addon-sdk`. Agnir Core/Profile 1.0 / repository-filesystem/1.0, operations v1.0.2 at `b5626394ec40a5cb7a28c01892acde07cc0adc8e`, remain unchanged. License baseline: CC-BY-NC-SA-4.0. Historical initialization is complete; no uncommitted-initialization prerequisite remains.
