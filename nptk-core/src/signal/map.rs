use crate::signal::{BoxedSignal, Listener, Ref, Signal};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A signal wrapping another signal and applying a mapping function, when the inner value is requested.
/// The mapping function will be cached and only re-evaluated when the inner signal changes.
/// This signal cannot be directly mutated. Use [MapSignal::signal] to get the inner signal.
///
/// Calling [Signal::set], [Signal::set_value], [Signal::listen] or [Signal::notify] has no effect.
pub struct MapSignal<T: 'static, U: 'static> {
    signal: BoxedSignal<T>,
    map: Rc<dyn Fn(Ref<T>) -> Ref<U>>,
    cached_value: RefCell<Option<U>>,
    cache_generation: Cell<u64>,
    signal_generation: Cell<u64>,
}

impl<T: 'static, U: 'static> MapSignal<T, U> {
    /// Create a new map signal using the given inner signal and mapping function.
    pub fn new(signal: BoxedSignal<T>, map: impl Fn(Ref<T>) -> Ref<U> + 'static) -> Self {
        Self {
            signal,
            map: Rc::new(map),
            cached_value: RefCell::new(None),
            cache_generation: Cell::new(0),
            signal_generation: Cell::new(0),
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
        self.cache_generation.set(self.cache_generation.get().wrapping_add(1));
    }

    /// Check if the cache is valid by comparing generations.
    fn is_cache_valid(&self) -> bool {
        self.cache_generation.get() == self.signal_generation.get()
    }
}

impl<T: 'static, U: 'static> Signal<U> for MapSignal<T, U>
where
    U: Clone,
{
    fn get(&self) -> Ref<'_, U> {
        // Check if we need to update the cache
        if !self.is_cache_valid() || self.cached_value.borrow().is_none() {
            let mapped_value = (self.map)(self.get_unmapped());
            let owned_value = match mapped_value {
                Ref::Owned(val) => val,
                Ref::Ref(val) => val.clone(),
                Ref::Borrow(val) => val.clone(),
                Ref::ReadGuard(guard) => guard.clone(),
                Ref::Rc(rc) => (*rc).clone(),
                Ref::Arc(arc) => (*arc).clone(),
            };
            *self.cached_value.borrow_mut() = Some(owned_value);
            self.signal_generation.set(self.cache_generation.get());
        }

        // Return cached value
        Ref::Owned(self.cached_value.borrow().as_ref().unwrap().clone())
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

impl<T: 'static, U: 'static> Clone for MapSignal<T, U> {
    fn clone(&self) -> Self {
        Self {
            signal: self.signal.dyn_clone(),
            map: self.map.clone(),
            cached_value: RefCell::new(None),
            cache_generation: Cell::new(0),
            signal_generation: Cell::new(0),
        }
    }
}
