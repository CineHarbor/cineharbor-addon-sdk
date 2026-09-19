# cineharbor-addon-sdk Next Actions

1. Verify the published lint-fix revision through every independent CI lane and its automatic second clean main run. Local strict clippy and native tests passed; only observed remote runs count as current main evidence.
2. Lock and test protocol compatibility for manifest/catalog/meta/streams, paging, ratings, error semantics, CORS and media routes. Preserve native/WASM client compatibility.
3. Verify media token enforcement, HLS segment/key rewriting, ad filtering, SSRF/redirect boundaries and non-open-proxy deployment defaults with negative tests.
4. Complete remote addon parity required for retiring remaining Web/local-service duplicate content paths; update pinned cross-repository revisions deliberately.
5. Produce documented production deployment configuration for Douban/Bangumi/Live/VOD/media; execute real health/CORS/auth/timeout smoke. Unavailable credentials or production access are explicit external blockers, not passes.
6. Align release metadata, security/license inventory and evidence-bound Agnir state. Do not claim release-ready until all 1.0.0 gates pass; do not publicly release during preparation.

Continue autonomously under the Principal's release authorization. Retain the selected Project identity/lineage and historical decision/evidence records.
