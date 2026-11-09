# Async Testing (`tokio`)

> Use `#[tokio::test]` with deterministic single-threaded execution and paused virtual time for timers and intervals.

## 1. Required Cargo Features

To use `#[tokio::test]` and `start_paused = true`, enable the `test-util`, `macros`, and `rt` features under `[dev-dependencies]`:

```toml
[dev-dependencies]
tokio = { version = "1", features = ["test-util", "macros", "rt"] }
```

Without `"test-util"`, `#[tokio::test(start_paused = true)]` and `tokio::time::advance` fail to compile.

## 2. Deterministic Runtime and Paused Time

Use the default single-threaded runtime (`flavor = "current_thread"`) unless multi-threaded task stealing is the behavior under test. Never call real `std::thread::sleep` in async tests. Pause the clock with `start_paused = true`:

```rust
use std::time::Duration;

#[tokio::test(start_paused = true)]
async fn retries_request_after_backoff_interval() {
    let client = TestClient::flaky_once();

    let task = tokio::spawn(async move { client.fetch_with_retry().await });
    tokio::time::advance(Duration::from_secs(30)).await;
    let response = task.await.expect("join task");

    assert!(response.is_ok(), "expected retry to succeed, got {response:?}");
}
```

## 3. Bounding Hung Futures with `timeout`

Wrap channel waits in `tokio::time::timeout` so a broken test fails with a clear message instead of hanging:

```rust
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn channel_delivers_messages_in_fifo_order() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    tx.send("first").await.expect("send first");
    tx.send("second").await.expect("send second");
    drop(tx);

    let first = timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("recv timed out");
    let second = timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("recv timed out");

    assert_eq!(first, Some("first"));
    assert_eq!(second, Some("second"));
    assert_eq!(rx.recv().await, None);
}
```

## Completion

Completion: async tests declare required `tokio` dev-dependencies, use `start_paused = true` instead of real sleeps, and bound channel receives with timeouts.
