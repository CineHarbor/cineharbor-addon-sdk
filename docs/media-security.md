# Media service security contract

This is the pre-1.0 VOD/Live media contract. Do not independently deploy it alongside a client that still synthesizes unsigned media URLs.

## Capabilities and integration

Standalone VOD/Live processes share a `MediaAuthorization` instance between addon metadata and media routers. Playback clients must preserve the exact URL from addon `stream` / `meta.videos[].stream` responses and the exact child URLs in rewritten HLS manifests. `sig` and `expires` are resource-bound, six-hour HMAC-SHA256 capabilities. A child URL cannot be changed into another source, upstream URL, endpoint, provider or public-base context. GET and HEAD intentionally share a capability; Range does not change the resource.

`CINEHARBOR_MEDIA_PROXY_TOKEN` is now a **server-only signing secret**, at least 32 UTF-8 bytes after trimming. Never put it in a NEXT_PUBLIC variable or a browser bundle. An explicitly invalid value aborts startup. An absent value generates an ephemeral OS-random 256-bit media key: access is still authenticated, but outstanding URLs expire on process restart. Configure one stable strong secret across replicas and restarts. This is independent of, and must never replace, Desktop updater signing keys. Rotation invalidates issued URLs; clients must reload addon metadata. There is no anonymous/bearer-token compatibility bypass and no arbitrary-URL signing HTTP endpoint.

Public-base URLs must match the externally visible proxy deployment. VOD uses the source JSON's `public_base_url`; Live also supports `CINEHARBOR_ADDON_PUBLIC_BASE_URL`. A reverse proxy may strip its configured prefix, but must not modify signed source/URL/expiry fields. TLS and secure operator secret provisioning remain deployment requirements. Do not log complete capability or upstream signed URLs.

## Egress and limits

Only HTTP(S), without URL credentials/fragments, is fetched. Private, loopback, link-local, multicast, documentation, mapped/transition and other denied special IP ranges are not allowed, including after redirects or in DNS answers. Mixed public/private DNS answers are rejected as a whole; checked addresses are supplied directly to the connector. Ambient HTTP(S) proxies and implicit redirects are disabled. There is no production private-address bypass.

Ports default to 80 and 443. Operators may explicitly set `CINEHARBOR_MEDIA_ALLOWED_PORTS=80,443,8080,8443` for trusted nonstandard public CDN services; this is a bounded list, not permission to reach private networks. At most five redirects are followed; HTTPS-to-HTTP downgrade is denied. DNS and connect limits are five seconds, read inactivity ten seconds, each upstream request sixty seconds. At most sixteen upstream operations are active; overload returns 503, without an unbounded waiting queue. A stream retains its concurrency slot until completion or cancellation.

Manifest input is limited to 2 MiB. Rewrite/signature expansion is budgeted before rewriting and capped at 16 MiB, with at most 20,000 conservative resource/line units. Keys are limited to 64 KiB. A byte response is limited to 128 MiB; clients downloading larger objects must request supported bounded ranges. Unknown-length streams are counted as they arrive and terminated on overflow/read failure, without buffering the complete object. Content encoding must remain identity so byte offsets are meaningful.

Single `bytes` ranges, If-Range, Content-Range, Content-Length, Accept-Ranges, ETag and Last-Modified are supported. Invalid or inconsistent upstream ranges fail closed. HEAD sends HEAD upstream and no body downstream; transformed manifests do not advertise the raw upstream length. CORS preflight permits GET/HEAD/OPTIONS and Range/If-Range, without triggering upstream requests. Responses use private/no-store, no-referrer and nosniff. User cookies/Authorization and upstream cookies are not forwarded.

## Verification and remaining operational controls

Run `python3 scripts/format-owned.py --check`, `cargo check --workspace --all-targets --locked`, `cargo test --workspace --all-targets --all-features --locked`, `cargo test --workspace --doc --all-features --locked`, and `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` using the pinned toolchain and Core revision.

The test-only exact-socket exception is not compiled into production. Unit/fixture results do not certify a production DNS topology, operator configuration, reverse proxy, real player or installed application. Deploy behind an egress firewall as defense in depth; apply ingress rate limiting and audience/account controls as appropriate. Resource capabilities restrict what may be fetched, not which audiences may consume an otherwise public addon catalog. Production smoke, actual client downloads/playback/expiry refresh, security review and final release matrix are mandatory before release-ready.

Design references: OWASP Server Side Request Forgery Prevention Cheat Sheet; reqwest 0.12.28 ClientBuilder/dns::Resolve documentation; ring 0.17.14 HMAC API. The implementation uses the lockfile-resolved versions, not floating documentation examples.
