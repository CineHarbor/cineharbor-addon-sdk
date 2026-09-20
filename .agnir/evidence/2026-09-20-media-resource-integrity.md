# Media resource integrity — 2026-09-20

## Source and environment

Base: `CineHarbor/cineharbor-addon-sdk@55856760a0872e50bea4e9a4f0da77f8e30dfb2c`, with Core pinned to `d51414bba4964dd7cee02780e41c73e35190ce76`. A fresh source inventory (facade run `35424839669`, attempt 4, artifact `10596207415`) was checksum-verified. Both edited media files matched the current remote base before changes; unrelated newer version/Bangumi changes were retained.

The existing release-toolbox artifact `10576790461` from facade run `35418243801` supplied Rust 1.98.1 and locked offline registry dependencies. ZIP SHA256: `7635b8d1784ce026ceed4ef5118eed28a2fbf8577127d5d75bf5b4bb80f1e7e5`; nested archive hashes also passed. No credentials or signing keys were changed.

## Repaired defects

- HLS proxy tokens are attached to the actual URI attribute, regardless of attribute order, rather than the last quoted attribute. Complete attributes are parsed so names containing commas or the text `token=` remain unchanged.
- Only the configured proxy origin, exact base-path prefix, and declared VOD m3u8/segment/key endpoints receive the credential. Foreign URLs, userinfo URLs, comments, unrelated attributes and malformed resource query shapes are not credentialed. Top-level stale/duplicate tokens are replaced once; nested upstream query credentials and URL fragments are preserved.
- VOD and Live manifest references resolve against the final response document after redirects. Keeping the full document URL also fixes extensionless playlists and query-only references (RFC 8216 section 4.1; RFC 3986 section 5).
- Public fetch/body error responses no longer include reqwest's URL-bearing error strings, which could expose upstream signatures.

## Observed local validation

All commands ran on the current 1.0.0 workspace plus the exact code blobs below, using the pinned toolchain and offline locked registry:

```
python3 scripts/format-owned.py --check
cargo check --workspace --all-targets --locked --offline
cargo test --workspace --all-targets --all-features --locked --offline
cargo test --workspace --doc --all-features --locked --offline
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
git diff --check
```

Every command passed. Native workspace tests: **47 passed, 0 failed, 0 ignored**; the media crate accounts for 19 tests, including eight new tests. Doc-test execution passed (no examples present).

A separate negative-control run restored the unchanged base implementation while retaining four new HTTP/URL regressions. All four failed as expected: multitrack token placement, redirect-base resolution, signed-URL error redaction and complete-document URI resolution. The repair was then restored and the full workspace passed again. These are deterministic local fixture/logic checks, not deployed-service or real media-decoder acceptance.

### Validated Git blobs

- `crates/cineharbor-media/src/lib.rs`: `1aaab747a05cf4a7eabd822664c83b7ef9ae6efe`
- `crates/cineharbor-media/src/serve.rs`: `1f5a7fc1e246eb256a14e0050486262d0736e125`
- `crates/cineharbor-media/src/serve/token.rs`: `ff108dc3706f78b61cb339674acb5511c9a0dcf0`

## Remaining gates

PR CI and two successful mandatory main CI executions must be observed for the new commit before downstream pins are advanced. Earlier green SDK runs do not certify this changed source.

This repair does not close the separate media egress/SSRF/DNS-rebinding/no-open-proxy gate, Live authorization, byte-range/HEAD/buffering acceptance, production service deployment, real installed playback, signed RC or retained-data updater gates. RELEASE_READY and PUBLIC_RELEASE_EXECUTED remain false.
