# Bangumi deterministic upstream injection — 2026-09-19

## Release problem

The standalone Bangumi addon used a compile-time fixed `https://api.bgm.tv` upstream. That makes a CI browser/addon acceptance depend on a third-party service and prevents deterministic fixtures for failure, timeout and content-shape cases.

## Change

- Keep `https://api.bgm.tv` as the default production upstream.
- Allow `CINEHARBOR_BANGUMI_BASE_URL` to override the upstream base for controlled deployments/tests.
- Add `BangumiAddon::with_base_url` so the same normalization is directly testable.
- Normalize whitespace and trailing slash; an empty override falls back to the production default.
- Add an owned unit regression for normalization.

## Release semantics

This checkpoint does **not** mark Bangumi or the repository release-ready. The previous main SHA `982d9148b30274e94ec83fe40f32f83a890cfa70` had two complete successful CI runs, but this source change invalidates that evidence for the new revision. Required next evidence is full PR CI, merge, two complete successful main runs on one exact SHA, then a Web real-browser Bangumi/product-path smoke using an isolated fixture.

No production endpoint is changed unless the new environment variable is explicitly set.
