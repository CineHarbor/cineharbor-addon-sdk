# cineharbor-addon-sdk Next Actions

1. Publish the reviewed media-boundary candidate with its coherent checkpoint; require the complete PR matrix and two observed successful runs on the exact resulting main SHA. Preserve the strict gates; do not count transfer-job or prior-source CI as final main evidence.
2. Migrate Web/Desktop to preserve addon-issued `sig`/`expires` URLs, remove browser exposure of the media signing secret, refresh expired metadata, and exercise signed initial/child/download/Range paths before coordinated deployment. Then align final Core/SDK/Web/Desktop pins and versions with fresh downstream matrices.
3. Validate deployed VOD/Live/Douban/Bangumi services, public-base/port configuration, TLS, secret provisioning, egress firewall and rate limits. Unit and loopback tests are not production DNS or player acceptance.
4. Finish the ADR consumer audit without capability loss, full browser/installed client acceptance, signed RC/updater data preservation, and security/license/brand review. External credentials and OS/production access must remain explicit blockers when unavailable.
5. Reconcile the facade's seven-repository evidence matrix; RELEASE_READY and PUBLIC_RELEASE_EXECUTED stay false until their actual gates are satisfied. Do not publicly release during this preparation run.
