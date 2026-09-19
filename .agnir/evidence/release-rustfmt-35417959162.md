# Owned Rust formatting checkpoint

Baseline: `47745eade69fbbd9c1b1d24ab46aa039fd1ccf60`

Normalized all owned workspace members with the pinned toolchain and verified `scripts/format-owned.py --check`. Sibling sources were not modified. This is formatting evidence only; compile, tests, security and release acceptance remain separate gates. Project identity and selected lineage are unchanged.

Changed sources:
- `crates/cineharbor-addon-douban/src/lib.rs`
- `crates/cineharbor-addon-douban/src/main.rs`
- `crates/cineharbor-addon-live/src/lib.rs`
- `crates/cineharbor-addon-sdk/src/addon.rs`
- `crates/cineharbor-addon-sdk/tests/e2e.rs`
- `crates/cineharbor-addon-vod/src/lib.rs`
- `crates/cineharbor-api/src/lib.rs`
