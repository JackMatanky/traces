//! End-to-end test harness: spawns the real `traces` binary (or, for `init`'s
//! inherently-interactive flow, drives it in-process) against fully isolated
//! sandboxes. See `support` for isolation guarantees.

#[path = "common/mod.rs"]
mod common;
#[path = "e2e/support.rs"]
mod support;

#[path = "e2e/completions.rs"]
mod completions;
#[path = "e2e/config.rs"]
mod config;
#[path = "e2e/env_policy.rs"]
mod env_policy;
#[path = "e2e/index.rs"]
mod index;
#[path = "e2e/init.rs"]
mod init;
#[path = "e2e/process_contract.rs"]
mod process_contract;
#[path = "e2e/query.rs"]
mod query;
#[path = "e2e/template.rs"]
mod template;
#[path = "e2e/tracked.rs"]
mod tracked;
#[path = "e2e/trust.rs"]
mod trust;
#[path = "e2e/workflow.rs"]
mod workflow;
