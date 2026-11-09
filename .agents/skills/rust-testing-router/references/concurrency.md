# Concurrency Model Checking (`loom`)

> Use `loom` to check lock-free atomics and custom synchronization across all valid C11 memory-model interleavings.

## 1. When to Use

Use `loom` for `Atomic*` operations, lock-free queues, or custom primitives where `Relaxed` vs. `Release`/`Acquire` bugs fail under rare schedules.

Do not use `loom` for ordinary business logic protected by `std::sync::Mutex` or `RwLock`.

## 2. Configuring `check-cfg` in `Cargo.toml`

Rust 1.80+ warns on unknown `cfg(loom)` gates unless you register `loom` in `Cargo.toml`:

```toml
[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(loom)'] }

[target.'cfg(loom)'.dependencies]
loom = "0.7"
```

## 3. Non-Vacuous Two-Atomic Message-Passing Model

A single-atomic test after `thread.join()` is vacuous because `join()` already synchronizes memory. A valid `loom` test requires at least two shared locations where one atomic synchronizes visibility of another:

```rust
#[cfg(loom)]
use loom::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
#[cfg(not(loom))]
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub struct Channel {
    data: AtomicUsize,
    ready: AtomicBool,
}

impl Channel {
    pub fn new() -> Self {
        Self {
            data: AtomicUsize::new(0),
            ready: AtomicBool::new(false),
        }
    }

    pub fn publish(&self, value: usize) {
        self.data.store(value, Ordering::Relaxed);
        // Release publishes prior Relaxed writes to any Acquire reader.
        self.ready.store(true, Ordering::Release);
    }

    pub fn try_read(&self) -> Option<usize> {
        // Weakening Acquire or Release to Relaxed makes loom fail below.
        if self.ready.load(Ordering::Acquire) {
            Some(self.data.load(Ordering::Relaxed))
        } else {
            None
        }
    }
}

#[cfg(loom)]
#[test]
fn acquire_release_orders_payload_visibility() {
    loom::model(|| {
        let chan = loom::sync::Arc::new(Channel::new());
        let writer = loom::sync::Arc::clone(&chan);

        let handle = loom::thread::spawn(move || {
            writer.publish(42);
        });

        if let Some(val) = chan.try_read() {
            assert_eq!(val, 42, "observed ready=true before data=42");
        }

        handle.join().expect("join writer");
        assert_eq!(chan.try_read(), Some(42));
    });
}
```

## 4. Running `loom`

Keep thread counts (2 to 3 threads) and loop bounds small inside `loom::model` so state-space exploration finishes quickly:

```bash
RUSTFLAGS="--cfg loom" cargo nextest run --lib acquire_release_orders_payload_visibility
```

## Completion

Completion: `cfg(loom)` is registered in `check-cfg`, the model fails when `Ordering` is weakened to `Relaxed`, and `RUSTFLAGS="--cfg loom"` passes.
