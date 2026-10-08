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
#[path = "integration/index_persistence_roundtrip.rs"]
mod index_persistence_roundtrip;
#[path = "integration/index_query.rs"]
mod index_query;
#[path = "integration/index_refresh.rs"]
mod index_refresh;
#[path = "integration/template_render.rs"]
mod template_render;
