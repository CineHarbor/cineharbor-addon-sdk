# 1.0.0 SDK CI foundation checkpoint

Baseline `2a0a8929d8b363b75c27b85eec1dc767f91917aa` was retrieved in source audit run `35417560069`, artifact `10576242656`, whose archive checksum was verified locally. Read AGENTS/AGNIR/AGNIR.yaml and selected continuity. No identity, lineage, profile or operations migration is performed.

Source inspection confirms `cineharbor-addon-vod → cineharbor-api → ../cineharbor-core` while old CI checked out only this repository. This checkpoint explicitly checks out immutable Core revision `05989fb6f2fe388d8eaeb4128445ac4c61662f5a`, pins Rust, scopes formatting to all owned workspace members and separates fmt/check/test/clippy jobs. No test or capability is removed. Formatting maintenance is deterministic, rejects non-Rust source changes and stale publication, and triggers CI for the published revision.

Local Python syntax and workflow YAML validation passed. Actual compiler/test/clippy/production results remain unverified until observed Actions or service evidence. Historical Core formatting log run `35382333866` includes SDK formatting drift; no old local success is treated as current acceptance.
