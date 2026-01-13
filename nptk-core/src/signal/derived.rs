use crate::reference::Ref;
use crate::signal::{BoxedSignal, Listener, Signal};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

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
pub struct DerivedSignal<T: 'static, U: 'static> {
    source: BoxedSignal<T>,
    compute: Rc<dyn Fn(Ref<T>) -> U>,
    cached_value: RefCell<Option<U>>,
    cache_generation: Cell<u64>,
    source_generation: Cell<u64>,
}

impl<T: 'static, U: 'static> DerivedSignal<T, U> {
    /// Create a new derived signal using the given source signal and computation function.
    pub fn new(signal: BoxedSignal<T>, compute: impl Fn(Ref<T>) -> U + 'static) -> Self {
        Self {
            source: signal,
            compute: Rc::new(compute),
            cached_value: RefCell::new(None),
            cache_generation: Cell::new(0),
            source_generation: Cell::new(0),
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

    /// Invalidate the cache when the source signal changes.
    fn invalidate_cache(&self) {
        self.cache_generation.set(self.cache_generation.get().wrapping_add(1));
    }

    /// Check if the cache is valid by comparing generations.
    fn is_cache_valid(&self) -> bool {
        self.cache_generation.get() == self.source_generation.get()
    }
}

impl<T: 'static, U: 'static> Signal<U> for DerivedSignal<T, U>
where
    U: Clone,
{
    fn get(&self) -> Ref<'_, U> {
        // Check if we need to update the cache
        if !self.is_cache_valid() || self.cached_value.borrow().is_none() {
            let computed_value = (self.compute)(self.get_source());
            *self.cached_value.borrow_mut() = Some(computed_value);
            self.source_generation.set(self.cache_generation.get());
        }

        // Return cached value as owned
        Ref::Owned(self.cached_value.borrow().as_ref().unwrap().clone())
    }

    fn set_value(&self, _: U) {
        // Derived signals are read-only
    }

    fn listen(&self, _: Listener<U>) {
        // Listeners are not supported for derived signals
        // They should listen to the source signal instead
    }

    fn notify(&self) {
        self.invalidate_cache();
        self.source.notify();
    }

    fn dyn_clone(&self) -> BoxedSignal<U> {
        Box::new(self.clone())
    }
}

impl<T: 'static, U: 'static> Clone for DerivedSignal<T, U> {
    fn clone(&self) -> Self {
        Self {
            source: self.source.dyn_clone(),
            compute: self.compute.clone(),
            cached_value: RefCell::new(None),
            cache_generation: Cell::new(0),
            source_generation: Cell::new(0),
        }
    }
}
