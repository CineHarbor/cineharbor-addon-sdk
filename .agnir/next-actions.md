# cineharbor-addon-sdk Next Actions

1. Run the Bangumi upstream-injection change through the complete PR CI matrix; after merge, require two complete successful CI runs on the exact new main SHA before using it as release evidence.
2. Pin the verified Addon SDK SHA into Web and add deterministic real-browser Bangumi plus product-path acceptance using isolated fixtures.
3. Preserve protocol compatibility for manifest/catalog/meta/streams, paging, ratings, error semantics, CORS and media routes; retain native/WASM client compatibility.
4. Verify media token enforcement, HLS segment/key rewriting, ad filtering, SSRF/redirect boundaries and non-open-proxy deployment defaults with negative tests.
5. Produce and execute documented production deployment configuration for Douban/Bangumi/Live/VOD/media, including real health/CORS/auth/timeout smoke. Unavailable credentials or production access are explicit external blockers, not passes.
6. Align 1.0.0 release metadata, security/license inventory and evidence-bound Agnir state. Public release is authorized by the Principal only after all hard gates genuinely pass; do not weaken or bypass a failed gate.

Continue autonomously under the Principal's release authorization. Retain the selected Project identity/lineage and historical decision/evidence records.
