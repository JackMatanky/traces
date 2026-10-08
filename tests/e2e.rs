//! End-to-end test harness: spawns the real `traces` binary (or, for `init`'s
//! inherently-interactive flow, drives it in-process) against fully isolated
//! sandboxes. See `support` for isolation guarantees.

#[path = "common/mod.rs"]
mod common;
#[path = "e2e/support.rs"]
mod support;

#[path = "e2e/config_capability.rs"]
mod config_capability;
#[path = "e2e/dispatch.rs"]
mod dispatch;
#[path = "e2e/env_policy.rs"]
mod env_policy;
#[path = "e2e/golden_path.rs"]
mod golden_path;
#[path = "e2e/init.rs"]
mod init;
#[path = "e2e/process_contract.rs"]
mod process_contract;
#[path = "e2e/template_write.rs"]
mod template_write;
#[path = "e2e/tracked.rs"]
mod tracked;
#[path = "e2e/untrust.rs"]
mod untrust;
