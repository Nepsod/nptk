use crate::reference::Ref;
use crate::signal::{BoxedSignal, Listener, Signal};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

/// A signal that combines two signals into a tuple signal.
///
/// The tuple signal emits a new value whenever either of the source signals changes.
/// Values are cached and only recomputed when one of the source signals changes.
pub struct ZipSignal<A: Send + Sync + 'static, B: Send + Sync + 'static> {
    signal_a: BoxedSignal<A>,
    signal_b: BoxedSignal<B>,
    cached_value: RwLock<Option<(A, B)>>,
    cache_generation: AtomicU64,
    signal_a_generation: AtomicU64,
    signal_b_generation: AtomicU64,
}

impl<A: Send + Sync + 'static, B: Send + Sync + 'static> ZipSignal<A, B> {
    /// Create a new zip signal that combines two signals.
    pub fn new(signal_a: BoxedSignal<A>, signal_b: BoxedSignal<B>) -> Self {
        Self {
            signal_a,
            signal_b,
            cached_value: RwLock::new(None),
            cache_generation: AtomicU64::new(0),
            signal_a_generation: AtomicU64::new(0),
            signal_b_generation: AtomicU64::new(0),
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
        self.cache_generation.fetch_add(1, Ordering::Relaxed);
    }

    /// Check if the cache is valid by comparing generations.
    fn is_cache_valid(&self) -> bool {
        self.cache_generation.load(Ordering::Relaxed) == self.signal_a_generation.load(Ordering::Relaxed)
            && self.cache_generation.load(Ordering::Relaxed) == self.signal_b_generation.load(Ordering::Relaxed)
    }
}

impl<A: Send + Sync + 'static + Clone, B: Send + Sync + 'static + Clone> Signal<(A, B)> for ZipSignal<A, B> {
    fn get(&self) -> Ref<'_, (A, B)> {
        // Check if we need to update the cache
        let needs_update = !self.is_cache_valid() || self.cached_value.read().unwrap().is_none();
        
        if needs_update {
            let val_a = self.signal_a.get();
            let val_b = self.signal_b.get();
            let tuple = ((*val_a).clone(), (*val_b).clone());
            *self.cached_value.write().unwrap() = Some(tuple);
            let gen = self.cache_generation.load(Ordering::Relaxed);
            self.signal_a_generation.store(gen, Ordering::Relaxed);
            self.signal_b_generation.store(gen, Ordering::Relaxed);
        }

        // Return cached value as owned
        Ref::Owned(self.cached_value.read().unwrap().as_ref().unwrap().clone())
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

impl<A: Send + Sync + 'static, B: Send + Sync + 'static> Clone for ZipSignal<A, B> {
    fn clone(&self) -> Self {
        Self {
            signal_a: self.signal_a.dyn_clone(),
            signal_b: self.signal_b.dyn_clone(),
            cached_value: RwLock::new(None),
            cache_generation: AtomicU64::new(0),
            signal_a_generation: AtomicU64::new(0),
            signal_b_generation: AtomicU64::new(0),
        }
    }
}
