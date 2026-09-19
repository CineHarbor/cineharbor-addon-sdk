# VOD catalog contract repair — 2026-09-19

Baseline SDK: 92e997eefbf093c921b2a10ca157336088f8663a. Tested sibling Core: e2bb2c6cab5cf620ab4eef34e05b254b26dc3139; ci/dependency.json now pins that exact source.

Web Actions run 35422092467, job 105841534497, passed production WASM/build, WASM persistence smoke and live cross-origin smoke, then failed VOD because the movie catalog returned both a movie and a series (2 != 1). Web requests both declared catalogs, so mixed results also duplicate visible search results. The original strict browser assertion is retained.

Production VOD now validates the declared catalog id/type, filters by inferred content type before applying skip and normalizes English type labels case-insensitively. A real local HTTP upstream regression exercises the production parser and handler, both catalogs, independent pagination, out-of-range skip and undeclared catalogs/types.

Local gates on this candidate: owned rustfmt; cargo test --workspace --all-targets --all-features --locked (38 passed, zero failed/ignored); cargo clippy --workspace --all-targets --all-features --locked -- -D warnings (passed). Web tooling regression suite: 21 passed. WASM release build passed. Local Chromium cannot navigate loopback (net::ERR_BLOCKED_BY_ADMINISTRATOR); this is not a browser pass. Updated Web CI must verify this pinned revision in real browser tests.

Checkpoint: same Project identity/lineage, Core/Profile 1.0 / repository-filesystem/1.0 and operational Agnir 1.0.2. Material implementation/evidence saved together. RELEASE_READY remains false. Remaining protocol, security, product, deployment and real signed Desktop updater acceptance have not been waived. No public release.
