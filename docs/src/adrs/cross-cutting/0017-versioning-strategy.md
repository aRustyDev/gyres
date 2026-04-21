---
number: 17
title: Independent versioning with shared MSRV
date: 2026-04-21
status: accepted
---

# 17. Independent versioning with shared MSRV

Date: 2026-04-21

## Status

Accepted

## Context

With 7 crates published (growing to 10), we need a versioning strategy. The main question: lockstep (all crates same version) vs independent (each crate versions separately).

Research findings:
- **Independent is the norm.** Tokio, axum, tower, serde all version crates independently. Tokio is at `1.52.1` while tokio-macros is at `2.7.0`.
- **Bevy is the sole lockstep example** (30+ crates, same version). This makes sense for Bevy — it ships as a single product. Gyres is infrastructure, not a monolith.
- No major workspace uses `version.workspace = true` for crate versions. Axum uses `workspace.package` only for `rust-version`.
- Pre-1.0 convention: `0.0.x` = experimental. `0.x.y` (x >= 1) = usable, breaking changes at minor bumps.
- MSRV: serde pins `1.56` (conservative), bevy pins `1.95` (aggressive), axum pins `1.80` (moderate).

## Decision

### Independent versioning

Each crate has its own version in its `Cargo.toml`. No `version.workspace = true`. When `gyres-store` has a breaking change, `gyres-polar` doesn't need a version bump.

### Pre-1.0 strategy

- Stay at `0.0.x` during initial prototyping (current state — API shape not committed).
- Move to `0.1.0` per-crate when that crate has a usable API shape and is ready for early adopters.
- Use `0.x.y` with minor bumps for breaking changes (semver pre-1.0 rules).
- `1.0.0` signals "this API is stable and we commit to semver."

### MSRV policy

- Set `rust-version` in `[workspace.package]`, inherited by all crates.
- Target N-4 stable releases behind latest. Currently `1.85` (edition 2024).
- Bump MSRV in a dedicated PR, not mixed with feature work.

### Release process

- Manual releases for now (like tokio, serde, axum).
- Adopt `cargo-release` or `release-plz` when release frequency justifies automation.

## Consequences

- Breaking changes in one crate don't cascade version bumps across the workspace.
- Users can pin specific crate versions without being forced to upgrade everything.
- Internal `path` + `version` dependencies in Cargo.toml must specify version ranges that are kept in sync manually.
- No coordinated "gyres 0.5.0 release" — each crate evolves at its own pace.
- The `gyres` umbrella re-export crate tracks compatible version ranges of all sub-crates, providing a "meta-version" for users who want one dependency.
