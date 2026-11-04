//! Caches render-scoped resources in [`minijinja::State`]'s typed extension
//! storage.
//!
//! [`cached`] backs `query.rs`'s
//! [`WorkspaceIndex`](crate::index::WorkspaceIndex) cache. It stashes a load
//! result in a typed extension. A render that calls into a namespace several
//! times then pays for one load instead of hand-rolling its own get-or-load
//! body per resource. `schema.rs` seeds its render's
//! [`SchemaService`](crate::schema::SchemaService) the same way, with a typed
//! newtype of its own and no load step. [`Schema`](crate::schema::Schema)'s
//! own minijinja `Object` impl can then reach it without holding a reference
//! itself.
//!
//! minijinja 3's extensions replace the string-keyed temp pattern for typed
//! state. Each extension uses a `TypeId` key instead of a `&'static str`. It
//! needs no [`Value`](minijinja::value::Value)
//! [`Object`](minijinja::value::Object) wrapper and no interior mutability
//! because it needs `&mut State`. Like temps, extensions are render-local.
//! They persist across includes, blocks, and macros. They drop with the
//! render's state.

use minijinja::State;

/// Wraps a render-scoped cacheable value in its own extension type, keeping
/// the cache slot isolated from any raw `T` extension elsewhere. `T` alone
/// selects the slot. A second `cached()` call site that instantiates the same
/// `T` in one render reuses the first call's value. Templates never see this
/// type.
#[derive(Debug)]
struct Cached<T>(T);

/// Returns the render-scoped value cached in `state`'s extensions.
///
/// The function runs `load` when this render has no cached value. It caches
/// the load result before it returns the value.
///
/// `T` should be cheap to clone (an `Arc<_>` or similarly shared type).
/// Every call clones the cached value out of the extension rather than
/// handing back a reference tied to its lifetime.
///
/// A failed load caches nothing, so the next call retries `load`.
///
/// # Errors
///
/// - `E` if `load` returns an error.
pub(super) fn cached<T, E>(
    state: &mut State,
    load: impl FnOnce() -> Result<T, E>,
) -> Result<T, E>
where
    T: Clone + Send + 'static,
{
    if let Some(cached) = state.get_extension::<Cached<T>>() {
        return Ok(cached.0.clone());
    }
    let value = load()?;
    state.get_or_insert_extension(Cached(value.clone()));
    Ok(value)
}

#[cfg(test)]
mod tests {
    mod cached {
        use std::cell::Cell;

        use minijinja::Environment;
        use pretty_assertions::assert_eq;

        use super::super::*;

        #[test]
        fn second_call_reuses_the_cached_value_without_reloading() {
            let env = Environment::new();
            let mut state = env.empty_state();
            let calls = Cell::new(0);
            let load = || {
                calls.set(calls.get() + 1);
                Ok::<_, ()>(42_i32)
            };

            let first = cached(&mut state, load);
            let second = cached(&mut state, load);

            assert_eq!(first, Ok(42));
            assert_eq!(second, Ok(42));
            assert_eq!(calls.get(), 1, "load must run once, not per call");
        }

        #[test]
        fn a_failed_load_is_not_cached_and_retries_next_call() {
            let env = Environment::new();
            let mut state = env.empty_state();
            let calls = Cell::new(0);
            let load = || {
                calls.set(calls.get() + 1);
                if calls.get() == 1 {
                    Err::<i32, _>("boom")
                } else {
                    Ok(7)
                }
            };

            let first = cached(&mut state, load);
            let second = cached(&mut state, load);

            assert_eq!(first, Err("boom"));
            assert_eq!(second, Ok(7));
            assert_eq!(calls.get(), 2, "a failed load must not be cached");
        }
    }
}
