//! Caches render-scoped resources in [`minijinja::State`]'s typed extension
//! storage.
//!
//! [`cached`] backs the [`WorkspaceIndex`](crate::index::WorkspaceIndex) cache
//! in `query.rs`. It stores a load result in a typed extension, so repeated
//! namespace calls in one render pay for one load.
//!
//! The `schema.rs` module seeds its
//! [`SchemaService`](crate::schema::SchemaService) the same way, using a typed
//! newtype without a load step. The `MiniJinja`
//! [`Object`](minijinja::value::Object) impl for
//! [`Schema`](crate::schema::Schema) can then reach the service without holding
//! a reference.
//!
//! `MiniJinja` 3 extensions replace the string-keyed temp pattern for typed
//! state by keying on `TypeId` instead of `&'static str`. They need no
//! [`Value`](minijinja::value::Value) or [`Object`](minijinja::value::Object)
//! wrapper, and no interior mutability because access needs `&mut State`.
//!
//! Like temps, extensions are render-local and persist across includes, blocks,
//! and macros. They drop when the render's state drops.

use minijinja::State;

/// Wraps a render-scoped cacheable value in its own extension type, keeping the
/// cache slot isolated from any raw `T` extension elsewhere.
///
/// `T` alone selects the slot. A second `cached()` call site that instantiates
/// the same `T` in one render reuses the first call's value. Templates never
/// see this type.
#[derive(Debug)]
struct Cached<T>(T);

/// Returns the render-scoped value cached in `state`'s extensions.
///
/// The function runs `load` when this render has no cached value. It caches the
/// load result before it returns the value.
///
/// `T` needs to be cheap to clone (an `Arc<_>` or a similarly shared type).
///
/// Every call clones the cached value out of the extension rather than handing
/// back a reference tied to its lifetime.
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
