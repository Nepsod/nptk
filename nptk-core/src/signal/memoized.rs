use crate::reference::Ref;
use crate::signal::{BoxedSignal, Listener, Signal};
use std::sync::{Arc, OnceLock};

/// A signal for creating a value once, when requested.
/// The value is immutable after creation.
/// Calling [Signal::set], [Signal::set_value], [Signal::listen] or [Signal::notify] has no effect.
///
/// **NOTE:** The inner factory function will only be called **once**, when the value is requested via [Signal::get].
pub struct MemoizedSignal<T: Send + Sync + 'static> {
    inner: Arc<OnceLock<T>>,
    factory: Arc<dyn Fn() -> T + Send + Sync>,
}

impl<T: Send + Sync + 'static> MemoizedSignal<T> {
    /// Create a new memoized signal using the given factory function.
    pub fn new(factory: impl Fn() -> T + Send + Sync + 'static) -> Self {
        Self {
            inner: Arc::new(OnceLock::new()),
            factory: Arc::new(factory),
        }
    }
}

impl<T: Send + Sync + 'static> Signal<T> for MemoizedSignal<T> {
    fn get(&self) -> Ref<'_, T> {
        Ref::Borrow(self.inner.get_or_init(&*self.factory))
    }

    fn set_value(&self, _: T) {}

    fn listen(&self, _: Listener<T>) {}

    fn notify(&self) {}

    fn dyn_clone(&self) -> BoxedSignal<T> {
        Box::new(self.clone())
    }
}

impl<T: Send + Sync + 'static> Clone for MemoizedSignal<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            factory: self.factory.clone(),
        }
    }
}
