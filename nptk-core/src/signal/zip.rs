use crate::reference::Ref;
use crate::signal::{BoxedSignal, Listener, Signal};
use std::cell::{Cell, RefCell};

/// A signal that combines two signals into a tuple signal.
///
/// The tuple signal emits a new value whenever either of the source signals changes.
/// Values are cached and only recomputed when one of the source signals changes.
pub struct ZipSignal<A: 'static, B: 'static> {
    signal_a: BoxedSignal<A>,
    signal_b: BoxedSignal<B>,
    cached_value: RefCell<Option<(A, B)>>,
    cache_generation: Cell<u64>,
    signal_a_generation: Cell<u64>,
    signal_b_generation: Cell<u64>,
}

impl<A: 'static, B: 'static> ZipSignal<A, B> {
    /// Create a new zip signal that combines two signals.
    pub fn new(signal_a: BoxedSignal<A>, signal_b: BoxedSignal<B>) -> Self {
        Self {
            signal_a,
            signal_b,
            cached_value: RefCell::new(None),
            cache_generation: Cell::new(0),
            signal_a_generation: Cell::new(0),
            signal_b_generation: Cell::new(0),
        }
    }

    /// Get the first source signal.
    pub fn signal_a(&self) -> BoxedSignal<A> {
        self.signal_a.dyn_clone()
    }

    /// Get the second source signal.
    pub fn signal_b(&self) -> BoxedSignal<B> {
        self.signal_b.dyn_clone()
    }

    /// Invalidate the cache when a source signal changes.
    fn invalidate_cache(&self) {
        self.cache_generation.set(self.cache_generation.get().wrapping_add(1));
    }

    /// Check if the cache is valid by comparing generations.
    fn is_cache_valid(&self) -> bool {
        self.cache_generation.get() == self.signal_a_generation.get()
            && self.cache_generation.get() == self.signal_b_generation.get()
    }
}

impl<A: 'static + Clone, B: 'static + Clone> Signal<(A, B)> for ZipSignal<A, B> {
    fn get(&self) -> Ref<'_, (A, B)> {
        // Check if we need to update the cache
        if !self.is_cache_valid() || self.cached_value.borrow().is_none() {
            let val_a = self.signal_a.get();
            let val_b = self.signal_b.get();
            let tuple = ((*val_a).clone(), (*val_b).clone());
            *self.cached_value.borrow_mut() = Some(tuple);
            self.signal_a_generation.set(self.cache_generation.get());
            self.signal_b_generation.set(self.cache_generation.get());
        }

        // Return cached value as owned
        Ref::Owned(self.cached_value.borrow().as_ref().unwrap().clone())
    }

    fn set_value(&self, _: (A, B)) {
        // Zip signals are read-only
    }

    fn listen(&self, _: Listener<(A, B)>) {
        // Listeners are not supported for zip signals
        // They should listen to the source signals instead
    }

    fn notify(&self) {
        self.invalidate_cache();
        self.signal_a.notify();
        self.signal_b.notify();
    }

    fn dyn_clone(&self) -> BoxedSignal<(A, B)> {
        Box::new(self.clone())
    }
}

impl<A: 'static, B: 'static> Clone for ZipSignal<A, B> {
    fn clone(&self) -> Self {
        Self {
            signal_a: self.signal_a.dyn_clone(),
            signal_b: self.signal_b.dyn_clone(),
            cached_value: RefCell::new(None),
            cache_generation: Cell::new(0),
            signal_a_generation: Cell::new(0),
            signal_b_generation: Cell::new(0),
        }
    }
}
