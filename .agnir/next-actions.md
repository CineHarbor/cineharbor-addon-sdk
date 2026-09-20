# cineharbor-addon-sdk Next Actions

1. Run the media-resource-integrity repair through the complete PR matrix. After merge, require two complete successful CI runs on the exact new main SHA; the prior version/Core-pin CI does not cover this repair.
2. Publish the verified SDK SHA to Web/Desktop dependency pins; downstream revisions must not pin an unverified candidate.
3. Complete the remaining media egress/SSRF/DNS-rebinding/no-open-proxy boundary and Live authorization, then verify Range/HEAD and bounded/streaming byte handling. HLS token propagation/redirect resolution/error redaction now have regression evidence, not blanket media-security certification.
4. Execute documented production health/CORS/auth/timeout smoke for deployed Douban/Bangumi/Live/VOD/media services. Missing production credentials/access remain explicit external blockers.
5. Reconcile security/license evidence and the seven-repository release matrix before public publication.

Continue autonomously under the Principal's 2026-09-19 release authorization.
