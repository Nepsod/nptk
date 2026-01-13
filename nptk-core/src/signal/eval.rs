use crate::reference::Ref;
use crate::signal::{BoxedSignal, Listener, Signal};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A signal that evaluates a function to get the value.
///
/// The evaluation function is cached and only re-evaluated when explicitly invalidated.
/// This provides better performance for expensive computations.
pub struct EvalSignal<T: 'static> {
    eval: Rc<dyn Fn() -> T>,
    cached_value: RefCell<Option<T>>,
    is_dirty: Cell<bool>,
}

impl<T: 'static> EvalSignal<T> {
    /// Create a new eval signal using the given evaluation function.
    pub fn new(eval: impl Fn() -> T + 'static) -> Self {
        Self {
            eval: Rc::new(eval),
            cached_value: RefCell::new(None),
            is_dirty: Cell::new(true),
        }
    }

    /// Invalidate the cached value, forcing re-evaluation on next access.
    pub fn invalidate(&self) {
        self.is_dirty.set(true);
    }
}

impl<T: 'static> Signal<T> for EvalSignal<T>
where
    T: Clone,
{
    fn get(&self) -> Ref<'_, T> {
        if self.is_dirty.get() || self.cached_value.borrow().is_none() {
            let new_value = (self.eval)();
            *self.cached_value.borrow_mut() = Some(new_value);
            self.is_dirty.set(false);
        }

        Ref::Owned(self.cached_value.borrow().as_ref().unwrap().clone())
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

impl<T: 'static> Clone for EvalSignal<T> {
    fn clone(&self) -> Self {
        Self {
            eval: self.eval.clone(),
            cached_value: RefCell::new(None),
            is_dirty: Cell::new(true),
        }
    }
}
