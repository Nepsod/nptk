use crate::signal::{BoxedSignal, Listener, Ref, Signal};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

/// A signal wrapping another signal and applying a mapping function, when the inner value is requested.
/// The mapping function will be cached and only re-evaluated when the inner signal changes.
/// This signal cannot be directly mutated. Use [MapSignal::signal] to get the inner signal.
///
/// Calling [Signal::set], [Signal::set_value], [Signal::listen] or [Signal::notify] has no effect.
pub struct MapSignal<T: Send + Sync + 'static, U: Send + Sync + 'static> {
    signal: BoxedSignal<T>,
    map: Arc<dyn Fn(Ref<T>) -> Ref<U> + Send + Sync>,
    cached_value: RwLock<Option<U>>,
    cache_generation: AtomicU64,
    signal_generation: AtomicU64,
}

impl<T: Send + Sync + 'static, U: Send + Sync + 'static> MapSignal<T, U> {
    /// Create a new map signal using the given inner signal and mapping function.
    pub fn new(signal: BoxedSignal<T>, map: impl Fn(Ref<T>) -> Ref<U> + Send + Sync + 'static) -> Self {
        Self {
            signal,
            map: Arc::new(map),
            cached_value: RwLock::new(None),
            cache_generation: AtomicU64::new(0),
            signal_generation: AtomicU64::new(0),
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

    /// Invalidate the cache when the inner signal changes.
    fn invalidate_cache(&self) {
        self.cache_generation.fetch_add(1, Ordering::Relaxed);
    }

    /// Check if the cache is valid by comparing generations.
    fn is_cache_valid(&self) -> bool {
        self.cache_generation.load(Ordering::Relaxed) == self.signal_generation.load(Ordering::Relaxed)
    }
}

impl<T: Send + Sync + 'static, U: Send + Sync + 'static> Signal<U> for MapSignal<T, U>
where
    U: Clone,
{
    fn get(&self) -> Ref<'_, U> {
        // Check if we need to update the cache
        // Note: checking is_none() requires read lock, so we do it carefully
        let needs_update = !self.is_cache_valid() || self.cached_value.read().unwrap().is_none();
        
        if needs_update {
            let mapped_value = (self.map)(self.get_unmapped());
            let owned_value = match mapped_value {
                Ref::Owned(val) => val,
                Ref::Ref(val) => val.clone(),
                Ref::Borrow(val) => val.clone(),
                Ref::ReadGuard(guard) => guard.clone(),
                Ref::Rc(rc) => (*rc).clone(),
                Ref::Arc(arc) => (*arc).clone(),
            };
            *self.cached_value.write().unwrap() = Some(owned_value);
            self.signal_generation.store(self.cache_generation.load(Ordering::Relaxed), Ordering::Relaxed);
        }

        // Return cached value
        Ref::Owned(self.cached_value.read().unwrap().as_ref().unwrap().clone())
    }

    fn set_value(&self, _: U) {}

    fn listen(&self, _: Listener<U>) {}

    fn notify(&self) {
        self.invalidate_cache();
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
            cached_value: RwLock::new(None),
            cache_generation: AtomicU64::new(0),
            signal_generation: AtomicU64::new(0),
        }
    }
}
