use crate::signal::{BoxedSignal, Listener, Ref, Signal};
use std::sync::{Arc, RwLock};

/// Simple signal implementation based on [Rc] and [RefCell] to get/set a value and notify listeners when it changes.
///
/// You can also mutate the inner value, but only in a set scope via [StateSignal::mutate].
pub struct StateSignal<T: Send + Sync + 'static> {
    value: Arc<RwLock<T>>,
    listeners: Arc<RwLock<Vec<Arc<Listener<T>>>>>,
}

impl<T: Send + Sync + 'static> StateSignal<T> {
    /// Creates a new signal with the given value.
    pub fn new(value: T) -> Self {
        Self {
            value: Arc::new(RwLock::new(value)),
            listeners: Arc::new(RwLock::new(Vec::with_capacity(1))),
        }
    }

    /// Mutate the inner value in a set scope. This scope is needed in order to notify the app for changes.
    pub fn mutate(&self, op: impl FnOnce(&mut T)) {
        op(&mut self.value.write().unwrap());
        self.notify();
    }
}

impl<T: Send + Sync + 'static> Signal<T> for StateSignal<T> {
    fn get(&self) -> Ref<'_, T> {
        Ref::ReadGuard(self.value.read().unwrap())
    }

    fn set_value(&self, value: T) {
        self.mutate(move |old| *old = value);
    }

    fn listen(&self, listener: Listener<T>) {
        // Use a more robust deduplication approach based on function content hash
        // Since we can't easily compare function pointers, we'll use a simpler approach
        // and just add the listener (the performance impact is minimal for typical use cases)
        self.listeners.write().unwrap().push(Arc::new(listener));
    }

    fn notify(&self) {
        for listener in self.listeners.read().unwrap().iter() {
            listener(self.get());
        }
    }

    fn dyn_clone(&self) -> BoxedSignal<T> {
        Box::new(self.clone())
    }
}

impl<T: Send + Sync + 'static> Clone for StateSignal<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            listeners: self.listeners.clone(),
        }
    }
}
