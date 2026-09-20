# cineharbor-addon-sdk Current State

Target: CineHarbor **1.0.0 public release**, with hard release gates enforced. **RELEASE_READY = false; PUBLIC_RELEASE_EXECUTED = false.** Scope and acceptance rules live in `CineHarbor/cineharbor/docs/releases/1.0.0/`.

## Repository boundary

This workspace owns the Stremio-compatible protocol, SDK/router/client, standalone Douban, Bangumi, Live and VOD addons, shared content API parser and media proxy. Remote addons are the ADR-0006 content data plane.

## Verified inputs

Main `e5f7a3a289ceb978d559910b5bf9f176809ada04` passed the complete SDK CI matrix twice: push run `35446032830` and clean workflow-dispatch run `35446059543`. It includes deterministic Bangumi upstream injection while preserving bgm.tv as the production default.

Core release source `d51414bba4964dd7cee02780e41c73e35190ce76` has separately passed its final main matrix twice.

## 1.0.0 version alignment

The release branch changes the SDK workspace package version to `1.0.0`, pins Core to verified `d51414bba4964dd7cee02780e41c73e35190ce76`, updates all eight owned workspace lock entries plus the pinned Core path entry to `1.0.0`, and aligns standalone Bangumi, Douban, Live and VOD manifest versions to `1.0.0`.

Protocol examples/tests already advertise `1.0.0`. Complete PR CI and two post-merge main runs are required before Web/Desktop pins move.

Media authorization/SSRF, production deployment smoke and final security/license review remain separate release obligations.

See `.agnir/evidence/2026-09-19-version-1.0.0.md`.

## Media resource integrity — 2026-09-20

The current 1.0.0 source now has scoped, attribute-aware VOD token propagation, final-response/document-relative HLS resolution for VOD and Live, and sanitized upstream error responses. Eight new tests cover URI ordering, credential scope/idempotence, redirects, extensionless/query-only references and diagnostic redaction. On the current workspace plus the recorded code blobs, all 47 native tests passed with no failures/ignores; owned formatting, locked check, doc tests and strict Clippy also passed. Restoring the previous implementation made all four new handler/URL regression checks fail, then restoring the repair returned the workspace to green.

See `.agnir/evidence/2026-09-20-media-resource-integrity.md` for immutable blob and toolchain evidence. Hosted PR CI and two complete runs at the eventual new main revision are still required. This is not a claim of SSRF/no-open-proxy, production or signed release acceptance.

## Media boundary candidate — 2026-09-20

The resource-integrity predecessor is verified on main at ca2ffa7c (two complete runs, 35483666706 and 35483692103). This candidate adds mandatory HMAC resource capabilities shared by initial VOD/Live metadata and child media resources, connection-time public DNS/literal/redirect policy, bounded authenticated Range/HEAD streaming, overload/cancellation handling and response privacy. The server signing secret is never embedded in URLs. No production private-network bypass, anonymous mode, lint suppression, skipped tests or updater-key change was introduced.

Local pinned/locked gates pass: 65 native workspace tests (35 media), owned fmt, check, doc-test execution and strict Clippy. Three negative controls failed as intended; the full restored candidate passed again. See `.agnir/evidence/2026-09-20-media-boundary.md` and `docs/media-security.md`. Hosted exact-tree PR CI and two final main executions are still required. This is a client wire-contract change: Web/Desktop must preserve issued signatures, not construct unsigned URLs or expose the server secret. Production egress/firewall configuration, actual player/download/expiry behavior and broader release acceptance remain open.

Project `urn:cineharbor:project:cineharbor-addon-sdk`; lineage `urn:cineharbor:lineage:cineharbor-addon-sdk`. Agnir Core/Profile 1.0 / repository-filesystem/1.0 and operations v1.0.2 remain unchanged. License: CC-BY-NC-SA-4.0.
