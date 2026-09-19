# cineharbor-addon-sdk Current State

Target: CineHarbor **1.0.0**, release work in progress; **RELEASE_READY = false**. Current scope and acceptance rules are in `CineHarbor/cineharbor/docs/releases/1.0.0/`.

## Repository boundaries

This workspace owns the Stremio-compatible protocol, SDK/router/client, standalone Douban, Bangumi, Live and VOD addons, the shared content API parser and media proxy. Remote addons are the ADR-0006 content data plane; pure state/native/WASM bridges remain in cineharbor-core. Existing protocol fixtures and addon/media tests must remain enforced during release work.

VOD depends on cineharbor-api, which consumes pure-core model types through a sibling Cargo path. `ci/dependency.json` pins the required Core revision and `scripts/ci-checkout.py` materializes the sibling layout. Rust is pinned to 1.98.1. Independent fmt/check/test/clippy lanes prevent one failed lane from hiding another. A successful main push dispatches exactly one second clean CI run; manual runs never recursively dispatch.

## Validation status

Main `982d9148b30274e94ec83fe40f32f83a890cfa70` passed the complete CI matrix twice (push run `35425301949`, workflow-dispatch run `35425324050`). Those results establish the previous baseline only. Any source change below requires fresh CI and, after merge, two successful runs on the new immutable main SHA.

The VOD catalog contract fix remains in place: movie/series searches filter the requested type before pagination and reject undeclared catalog ids/types. Core is pinned to `e2bb2c6cab5cf620ab4eef34e05b254b26dc3139`.

## Bangumi deterministic acceptance checkpoint

The Bangumi addon previously hard-coded `https://api.bgm.tv`, which prevented deterministic browser/addon acceptance against an isolated fixture. The release branch now adds `CINEHARBOR_BANGUMI_BASE_URL` as an upstream-base override while preserving bgm.tv as the production default. `BangumiAddon::with_base_url` normalizes the injected base and has an owned unit regression. This is testability/configuration only; no release gate is claimed passed by this change. PR CI, two post-merge main runs, Web real-browser Bangumi coverage, media security review, production deployment and final version alignment remain gates.

See `.agnir/evidence/2026-09-19-bangumi-deterministic-upstream.md`.

## Continuity

Project `urn:cineharbor:project:cineharbor-addon-sdk`; lineage `urn:cineharbor:lineage:cineharbor-addon-sdk`. Agnir Core/Profile 1.0 / repository-filesystem/1.0, operations v1.0.2 at `b5626394ec40a5cb7a28c01892acde07cc0adc8e`, remain unchanged. License baseline: CC-BY-NC-SA-4.0. Historical initialization is complete.
