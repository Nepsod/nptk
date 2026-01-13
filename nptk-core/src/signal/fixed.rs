use crate::signal::{BoxedSignal, Listener, Ref, Signal};
use std::sync::Arc;

/// A signal with a fixed value. The inner value cannot be mutated and listeners do not exist.
pub struct FixedSignal<T: Send + Sync + 'static> {
    value: Arc<T>,
}

impl<T: Send + Sync + 'static> FixedSignal<T> {
    /// Creates a new fixed signal.
    pub fn new(value: T) -> Self {
        Self {
            value: Arc::new(value),
        }
    }
}

impl<T: Send + Sync + 'static> From<Arc<T>> for FixedSignal<T> {
    fn from(value: Arc<T>) -> Self {
        Self { value }
    }
}

impl<T: Send + Sync + 'static> Signal<T> for FixedSignal<T> {
    fn get(&self) -> Ref<'_, T> {
        Ref::Arc(self.value.clone())
    }

    fn set_value(&self, _: T) {}

    fn listen(&self, _: Listener<T>) {}

    fn notify(&self) {}

    fn dyn_clone(&self) -> BoxedSignal<T> {
        Box::new(self.clone())
    }
}

impl<T: Send + Sync + 'static> Clone for FixedSignal<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
        }
    }
}
