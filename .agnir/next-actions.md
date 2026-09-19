# cineharbor-addon-sdk Next Actions

1. Run the 1.0.0 version/Core-pin branch through the complete PR matrix. After merge, require two complete successful CI runs on the exact new main SHA.
2. Publish the verified SDK SHA to Web/Desktop dependency pins; downstream revisions must not pin an unverified candidate.
3. Preserve protocol compatibility and complete media token/SSRF/redirect/open-proxy negative review.
4. Execute documented production health/CORS/auth/timeout smoke for deployed Douban/Bangumi/Live/VOD/media services. Missing production credentials/access remain explicit external blockers.
5. Reconcile security/license evidence and the seven-repository release matrix before public publication.

Continue autonomously under the Principal's 2026-09-19 release authorization.
