//! Integration test harness: proves cross-module data flow through
//! `traces_pkm`'s `test-utils`-gated public surface alone (no process spawning,
//! no crate-internal imports). See `tests/e2e.rs` for process-boundary coverage
//! of the same command surface.
//!
//! The former blanket `#![allow(clippy::expect_used)]` here was vestigial
//! (clippy.toml's `allow-expect-in-tests` already covers every test target)
//! and was removed in ticket 17; `.expect` style still follows the
//! `tests/common` convention: fixture/setup invariants only.
#![cfg(feature = "test-utils")]
#[path = "common/mod.rs"]
mod common;
#[path = "integration/persistence.rs"]
mod persistence;
#[path = "integration/query.rs"]
mod query;
#[path = "integration/refresh.rs"]
mod refresh;
#[path = "integration/template_render.rs"]
mod template_render;
