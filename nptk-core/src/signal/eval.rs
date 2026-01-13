use crate::reference::Ref;
use crate::signal::{BoxedSignal, Listener, Signal};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

/// A signal that evaluates a function to get the value.
///
/// The evaluation function is cached and only re-evaluated when explicitly invalidated.
/// This provides better performance for expensive computations.
pub struct EvalSignal<T: Send + Sync + 'static> {
    eval: Arc<dyn Fn() -> T + Send + Sync>,
    cached_value: RwLock<Option<T>>,
    is_dirty: AtomicBool,
}

impl<T: Send + Sync + 'static> EvalSignal<T> {
    /// Create a new eval signal using the given evaluation function.
    pub fn new(eval: impl Fn() -> T + Send + Sync + 'static) -> Self {
        Self {
            eval: Arc::new(eval),
            cached_value: RwLock::new(None),
            is_dirty: AtomicBool::new(true),
        }
    }

    /// Invalidate the cached value, forcing re-evaluation on next access.
    pub fn invalidate(&self) {
        self.is_dirty.store(true, Ordering::Relaxed);
    }
}

impl<T: Send + Sync + 'static> Signal<T> for EvalSignal<T>
where
    T: Clone,
{
    fn get(&self) -> Ref<'_, T> {
        // Need to check if dirty OR value missing
        let needs_update = self.is_dirty.load(Ordering::Relaxed) || self.cached_value.read().unwrap().is_none();
        
        if needs_update {
            let new_value = (self.eval)();
            *self.cached_value.write().unwrap() = Some(new_value);
            self.is_dirty.store(false, Ordering::Relaxed);
        }

        Ref::Owned(self.cached_value.read().unwrap().as_ref().unwrap().clone())
    }

    fn set_value(&self, _: T) {}

    fn listen(&self, _: Listener<T>) {}

    fn notify(&self) {
        self.invalidate();
    }

    fn dyn_clone(&self) -> BoxedSignal<T> {
        Box::new(self.clone())
    }
}

impl<T: Send + Sync + 'static> Clone for EvalSignal<T> {
    fn clone(&self) -> Self {
        Self {
            eval: self.eval.clone(),
            cached_value: RwLock::new(None),
            is_dirty: AtomicBool::new(true),
        }
    }
}
