use crate::signal::{BoxedSignal, Listener, Ref, Signal};
use std::sync::Arc;

/// A signal wrapping another signal and applying a mapping function, when the inner value is requested.
/// The mapping function will be cached and only re-evaluated when the inner signal changes.
/// This signal cannot be directly mutated. Use [MapSignal::signal] to get the inner signal.
///
/// Calling [Signal::set], [Signal::set_value], [Signal::listen] or [Signal::notify] has no effect.
pub struct MapSignal<T: Send + Sync + 'static, U: Send + Sync + 'static> {
    signal: BoxedSignal<T>,
    map: Arc<dyn Fn(Ref<T>) -> Ref<U> + Send + Sync>,
}

impl<T: Send + Sync + 'static, U: Send + Sync + 'static> MapSignal<T, U> {
    pub fn new(signal: BoxedSignal<T>, map: impl Fn(Ref<T>) -> Ref<U> + Send + Sync + 'static) -> Self {
        Self {
            signal,
            map: Arc::new(map),
        }
    }

    /// Get the inner signal.
    ///
    /// Can be used to mutate the inner value.
    pub fn signal(&self) -> BoxedSignal<T> {
        self.signal.dyn_clone()
    }

    /// Get the inner signal's value, without applying the mapping function.
    pub fn get_unmapped(&self) -> Ref<'_, T> {
        self.signal.get()
    }


}

impl<T: Send + Sync + 'static, U: Send + Sync + 'static> Signal<U> for MapSignal<T, U>
where
    U: Clone,
{
    fn get(&self) -> Ref<'_, U> {
        (self.map)(self.get_unmapped())
    }

    fn set_value(&self, _: U) {}

    fn listen(&self, _: Listener<U>) {}

    fn notify(&self) {
        self.signal.notify();
    }

    fn dyn_clone(&self) -> BoxedSignal<U> {
        Box::new(self.clone())
    }
}

impl<T: Send + Sync + 'static, U: Send + Sync + 'static> Clone for MapSignal<T, U> {
    fn clone(&self) -> Self {
        Self {
            signal: self.signal.dyn_clone(),
            map: self.map.clone(),
        }
    }
}
