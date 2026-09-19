# cineharbor-addon-sdk Current State

Target: CineHarbor **1.0.0**, release preparation in progress; **RELEASE_READY = false**. Current scope and acceptance rules are in `CineHarbor/cineharbor/docs/releases/1.0.0/`.

## Repository boundaries

This workspace owns the Stremio-compatible protocol, SDK/router/client, standalone Douban, Bangumi, Live and VOD addons, the shared content API parser and media proxy. Remote addons are the ADR-0006 content data plane; pure state/native/WASM bridges remain in cineharbor-core. Existing protocol fixtures and addon/media tests must remain enforced during release work.

VOD depends on cineharbor-api, which consumes pure-core model types through a sibling Cargo path. The former CI did not materialize that Core repository. `ci/dependency.json` now pins the required Core revision; `scripts/ci-checkout.py` creates and verifies the documented sibling layout. Rust is pinned to 1.98.1. All owned workspace members are format-checked without formatting sibling dependencies. Independent fmt/check/test/clippy lanes prevent one failed lane from hiding another. The one-time formatter is now retired. A successful main push dispatches exactly one second clean CI run; manual runs never recursively dispatch.

## Validation status

After dependency checkout/format repairs, main `9ebd1304769c9476cd4d8a60d49619aec423b195`, run `35417976333`, actually passed fmt/check/test and failed clippy. This checkpoint applies compiler-suggested simplifications, boxes the large media error response and fixes doc formatting without disabling warnings or removing tests. Local strict clippy passed; all 37 native tests passed with zero failures/ignores; the doc-test command completed (no examples present). Remote main results for this changed source, full browser compatibility, media authorization/SSRF review, production deployment and final version alignment remain release gates. These local results are not production or complete release acceptance.

## VOD catalog correctness checkpoint

Web CI exposed a real catalog type leak: movie and series searches each returned the same mixed results. The handler now filters the requested type before pagination and rejects undeclared catalog ids/types. A production-parser/HTTP-fixture regression and all 38 SDK native tests pass locally, as does strict clippy. The Core dependency is pinned to e2bb2c6cab5cf620ab4eef34e05b254b26dc3139. See evidence/2026-09-19-vod-catalog-contract.md; current-main remote browser/CI acceptance is still pending, not inferred from the local results.

## Continuity

Project `urn:cineharbor:project:cineharbor-addon-sdk`; lineage `urn:cineharbor:lineage:cineharbor-addon-sdk`. Agnir Core/Profile 1.0 / repository-filesystem/1.0, operations v1.0.2 at `b5626394ec40a5cb7a28c01892acde07cc0adc8e`, remain unchanged. License baseline: CC-BY-NC-SA-4.0. Historical initialization is complete; no uncommitted-initialization prerequisite remains.
