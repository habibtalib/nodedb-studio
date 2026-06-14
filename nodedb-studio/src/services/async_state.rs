//! The reusable loading/empty/error UI-state primitive.
//!
//! Kept in plain Rust (no renderer) so the state-mapping logic is unit-
//! testable. Every later wiring phase maps a `use_resource` result into an
//! `AsyncState<T>` via `from_value` and hands it to the `AsyncView` component.

use crate::services::error::StudioError;

/// Anything that can report emptiness, so `from_value` can distinguish a
/// loaded-but-empty result (-> Empty) from a loaded-with-data result.
pub trait IsEmpty {
    fn is_empty(&self) -> bool;
}

impl<T> IsEmpty for Vec<T> {
    fn is_empty(&self) -> bool {
        Vec::is_empty(self)
    }
}

// Consumed by plan 01-04 (wires `use_resource` results into the AsyncView
// popover). This is a binary crate, so `pub` does not suppress dead-code until
// a non-test caller references it; the scoped allow mirrors StudioError (01-01).
#[allow(dead_code)]
pub enum AsyncState<T> {
    Loading,
    Empty,
    Loaded(T),
    Error(StudioError),
}

impl<T: IsEmpty> AsyncState<T> {
    /// Pure mapping from a `use_resource` read (`Option<Result<T, StudioError>>`):
    ///   None              -> Loading  (first run not finished)
    ///   Some(Err(e))      -> Error(e)
    ///   Some(Ok(empty))   -> Empty
    ///   Some(Ok(data))    -> Loaded(data)
    // Consumed by plan 01-04 (see note on `AsyncState` above).
    #[allow(dead_code)]
    pub fn from_value(v: Option<Result<T, StudioError>>) -> Self {
        match v {
            None => AsyncState::Loading,
            Some(Err(e)) => AsyncState::Error(e),
            Some(Ok(t)) if t.is_empty() => AsyncState::Empty,
            Some(Ok(t)) => AsyncState::Loaded(t),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn async_state_none_is_loading() {
        let s = AsyncState::<Vec<u8>>::from_value(None);
        assert!(matches!(s, AsyncState::Loading));
    }

    #[test]
    fn async_state_empty_vec_is_empty() {
        let s = AsyncState::from_value(Some(Ok(Vec::<u8>::new())));
        assert!(matches!(s, AsyncState::Empty));
    }

    #[test]
    fn async_state_nonempty_is_loaded() {
        let s = AsyncState::from_value(Some(Ok(vec![1u8])));
        assert!(matches!(s, AsyncState::Loaded(_)));
    }

    #[test]
    fn async_state_err_is_error() {
        let s = AsyncState::<Vec<u8>>::from_value(Some(Err(StudioError::NotConnected)));
        assert!(matches!(s, AsyncState::Error(_)));
    }
}
