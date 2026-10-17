# cineharbor-addon-sdk Next Actions — 2026-10-18

1. Require strict locked SDK fmt/check/test/clippy PR CI at the rustls 0.23.45 lock candidate; merge only after green and require two complete main CI executions at identical SHA. Run full RustSec audit using updated DB and classify residual findings.
2. Advance Core's SDK source pin to new verified SDK and update its own rustls lock; then Web and Desktop to immutable verified revisions. Preserve signed-media wire contracts.
3. Complete deployed public DNS/egress, VOD/Live/Douban/Bangumi and real player/download functional smoke (not CI-only).
4. Resolve other vulnerability/license/brand/architecture review findings; production code signing/updater postponed to separately authorized public release, not waived. RELEASE_READY=false; PUBLIC_RELEASE_EXECUTED=false.
