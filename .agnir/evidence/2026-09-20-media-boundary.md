# Resource authorization, public egress and bounded media — 2026-09-20

Base main: `ca2ffa7c34c7c5e212f24c36b8defe2ea11256d0`, base tree `350baa04311f5ba3b0934a069d9ec7c8a26cd511`. Core remains pinned to `d51414bba4964dd7cee02780e41c73e35190ce76`. Scope: media service and standalone VOD/Live wiring, not public release or deployment.

## Implementation

- VOD and Live issue HMAC-SHA256 capabilities for exact source, upstream URL, media kind, endpoint, configured public base and expiry. Child manifests carry scoped signatures, not the shared signing key. Missing, expired, forged, duplicated or altered credentials fail closed; unknown sources cannot obtain egress. `ring` supplies the MAC, constant-time verification and OS entropy. The updater signing identity is not touched.
- A private HTTP client disables ambient proxies, automatic redirects, automatic Referer and decompression. At connection time a custom resolver vets every returned address and returns those same addresses to reqwest; no separate validation/connect DNS lookup exists. Literal URLs are checked before every hop. Non-global, mapped/transition and special addresses are denied, mixed DNS answers rejected, and HTTPS downgrades blocked. Redirects, DNS/connect/read/total request time and concurrent requests are bounded. Explicit operator port configuration does not bypass address policy.
- Authenticated GET/HEAD and single Range/If-Range forwarding preserve byte extents, validators and 206/416 semantics. Browser cookies, Authorization and upstream Set-Cookie are not relayed. Backpressure, cancellation and concurrency permits track the response stream. Input manifests, rewrite expansion, keys and media responses have explicit bounds; private/no-store and no-referrer headers protect capabilities.
- Initial VOD meta/stream and Live stream responses share the exact authorization instance with their media routers. Live single-source/demo source maps are included. Existing HLS URL/attribute integrity and advertisement-filter behavior are retained.

## Observed local verification

The source/toolbox ZIPs and nested archives were SHA256-verified. Source snapshot artifact `10596529141` (audit attempt 5) contains the same SDK base as the live ref. Pinned Rust/Cargo 1.98.1 and the project's offline locked registry were used.

Owned rustfmt check, all-target check, all-target/all-feature workspace tests, doc-test execution, strict Clippy `-D warnings`, and whitespace checks passed. **65 native tests passed, 0 failed, 0 ignored**; media has **35** tests. Three intentional negative controls (MAC verification bypass, public-address policy bypass, stream limit bypass) each made its targeted regression fail. All mutations were restored and the complete gates passed again. No lint suppression or test skipping was added. Cargo.lock only adds three direct dependency edges to already locked base64/futures-util/ring packages; package versions/checksums are unchanged.

Hosted PR and two successful executions at the eventual main SHA remain required. A fixed-content, hash-bound transfer job may materialize the reviewed candidate when this execution environment cannot directly reach GitHub; it must verify the complete candidate tree and all gates, publish only a new candidate branch, and leave no transfer helper in that tree. Main publication remains ordinary PR integration, never an unreviewed writer to main.

## Acceptance limits

Loopback HTTP fixtures have one exact socket exception under `cfg(test)` only. They exercise real HTTP handlers/redirects/streaming, not deployed service or real browser/player acceptance. The live DNS test verifies local resolution rejection; synthetic public/mixed/rebound answer tests verify the resolver's selection rule, not an external DNS penetration test. Web/Desktop must preserve addon-issued signatures and refresh expired resources rather than synthesize unsigned URLs. Signed installed RC/updater, production deployment smoke, full security/license review and final cross-repository acceptance remain open. `RELEASE_READY=false`; `PUBLIC_RELEASE_EXECUTED=false`.
