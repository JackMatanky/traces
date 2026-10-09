//! Integration test harness: proves cross-module data flow through
//! `traces_pkm`'s `test-utils`-gated public surface alone (no process spawning,
//! no crate-internal imports). See `tests/e2e.rs` for process-boundary coverage
//! of the same command surface.
#![cfg(feature = "test-utils")]
#[path = "integration/persistence.rs"]
mod persistence;
#[path = "integration/query.rs"]
mod query;
#[path = "integration/refresh.rs"]
mod refresh;
#[path = "integration/template_render.rs"]
mod template_render;
