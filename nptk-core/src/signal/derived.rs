use crate::reference::Ref;
use crate::signal::{BoxedSignal, Listener, Signal};
use std::sync::Arc;

/// A signal that derives its value from another signal using a computation function.
///
/// The computation function is cached and only re-evaluated when the source signal changes.
/// This provides efficient reactive computations that automatically track dependencies.
///
/// Unlike [EvalSignal](crate::signal::eval::EvalSignal), this signal tracks changes to its
/// source signal and only re-evaluates when the source changes, rather than on every access.
///
/// # Example
///
/// ```rust,no_run
/// use nptk_core::signal::{Signal, state::StateSignal};
///
/// let counter = StateSignal::new(5);
/// let doubled = counter.derived(|val| *val * 2); // Automatically tracks counter changes
/// ```
pub struct DerivedSignal<T: Send + Sync + 'static, U: Send + Sync + 'static> {
    source: BoxedSignal<T>,
    compute: Arc<dyn Fn(Ref<T>) -> U + Send + Sync>,
}

impl<T: Send + Sync + 'static, U: Send + Sync + 'static> DerivedSignal<T, U> {
    /// Create a new derived signal using the given source signal and computation function.
    pub fn new(signal: BoxedSignal<T>, compute: impl Fn(Ref<T>) -> U + Send + Sync + 'static) -> Self {
        Self {
            source: signal,
            compute: Arc::new(compute),
        }
    }

    /// Get the source signal.
    ///
    /// Can be used to mutate the source value.
    pub fn source(&self) -> BoxedSignal<T> {
        self.source.dyn_clone()
    }

    /// Get the source signal's value, without applying the computation function.
    pub fn get_source(&self) -> Ref<'_, T> {
        self.source.get()
    }


}

impl<T: Send + Sync + 'static, U: Send + Sync + 'static> Signal<U> for DerivedSignal<T, U>
where
    U: Clone,
{
    fn get(&self) -> Ref<'_, U> {
        Ref::Owned((self.compute)(self.get_source()))
    }

    fn set_value(&self, _: U) {
        // Derived signals are read-only
    }

    fn listen(&self, _: Listener<U>) {
        // Listeners are not supported for derived signals
        // They should listen to the source signal instead
    }

    fn notify(&self) {
        self.source.notify();
    }

    fn dyn_clone(&self) -> BoxedSignal<U> {
        Box::new(self.clone())
    }
}

impl<T: Send + Sync + 'static, U: Send + Sync + 'static> Clone for DerivedSignal<T, U> {
    fn clone(&self) -> Self {
        Self {
            source: self.source.dyn_clone(),
            compute: self.compute.clone(),
        }
    }
}
