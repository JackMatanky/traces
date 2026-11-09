# Fakes, Trait Seams, and Mocks

> Lead with hand-written fakes behind small traits. Use `mockall` only when call-order or call-count expectations are the contract.

## 1. Trait Seams (Native `async fn` in Traits)

Put external systems (databases, HTTP clients, clocks, filesystems) behind narrow domain traits so unit tests can inject deterministic in-memory fakes.

Starting in Rust 1.75+, native `async fn` in traits works without the `#[async_trait]` macro for static dispatch (`impl Trait` or generics):

```rust
pub trait UserStore {
    async fn fetch_user(&self, id: u64) -> Result<Option<User>, StoreError>;
    async fn save_user(&mut self, user: User) -> Result<(), StoreError>;
}

pub struct AccountService<S: UserStore> {
    store: S,
}
```

Do not mock pure functions or value types. Do not mock the boundary under test in an integration suite (`tests/`).

## 2. Hand-Written Fakes (Primary Pattern)

A small struct with an in-memory `HashMap` or `Vec` is clearer, faster to compile, and less brittle than macro-generated mocks:

```rust
use std::collections::HashMap;

#[derive(Default)]
struct FakeUserStore {
    users: HashMap<u64, User>,
    fail_next: bool,
}

impl UserStore for FakeUserStore {
    async fn fetch_user(&self, id: u64) -> Result<Option<User>, StoreError> {
        if self.fail_next {
            return Err(StoreError::Unavailable);
        }
        Ok(self.users.get(&id).cloned())
    }

    async fn save_user(&mut self, user: User) -> Result<(), StoreError> {
        if self.fail_next {
            return Err(StoreError::Unavailable);
        }
        self.users.insert(user.id, user);
        Ok(())
    }
}
```

## 3. `mockall` (Optional Secondary Pattern)

Use `mockall` only when the interaction protocol (call count, call order, or one method of a wide trait) is tested:

```rust
use mockall::automock;

#[automock]
trait Notifier {
    fn send_alert(&self, message: &str) -> Result<(), NotifyError>;
}

#[test]
fn sends_single_alert_when_threshold_is_exceeded() {
    let mut mock = MockNotifier::new();
    mock.expect_send_alert()
        .with(mockall::predicate::eq("CPU > 90%"))
        .times(1)
        .returning(|_| Ok(()));

    let status = monitor_cpu(95, &mock);

    assert!(status.is_ok());
}
```

## 4. Choosing Between Fakes and `mockall`

| Situation | Prefer |
|---|---|
| State-based verification or forced error return | Hand-written fake |
| Native `async fn` trait with static dispatch | Hand-written fake |
| Exact call count (`.times(1)`) or `Sequence` ordering | `mockall` |
| Wide trait where a test exercises only one method | `mockall` |

## Completion

Completion: external collaborators sit behind narrow traits, state-based tests use hand-written fakes without `#[async_trait]`, and `mockall` is reserved for interaction contracts.
