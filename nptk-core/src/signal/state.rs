use crate::signal::{BoxedSignal, Listener, Ref, Signal};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

/// Simple signal implementation based on [Rc] and [RefCell] to get/set a value and notify listeners when it changes.
///
/// You can also mutate the inner value, but only in a set scope via [StateSignal::mutate].
pub struct StateSignal<T: 'static> {
    value: Rc<RefCell<T>>,
    listeners: Vec<Rc<Listener<T>>>,
    listener_ids: HashSet<usize>,
}

impl<T: 'static> StateSignal<T> {
    /// Creates a new signal with the given value.
    pub fn new(value: T) -> Self {
        Self {
            value: Rc::new(RefCell::new(value)),
            listeners: Vec::with_capacity(1),
            listener_ids: HashSet::new(),
        }
    }

    /// Mutate the inner value in a set scope. This scope is needed in order to notify the app for changes.
    pub fn mutate(&self, op: impl FnOnce(&mut T)) {
        op(&mut self.value.borrow_mut());
        self.notify();
    }
}

impl<T: 'static> Signal<T> for StateSignal<T> {
    fn get(&self) -> Ref<'_, T> {
        Ref::Ref(self.value.borrow())
    }

    fn set_value(&self, value: T) {
        self.mutate(move |old| *old = value);
    }

    fn listen(&mut self, listener: Listener<T>) {
        let listener_id = &listener as *const _ as usize;
        
        // Prevent duplicate listeners
        if !self.listener_ids.contains(&listener_id) {
            self.listener_ids.insert(listener_id);
            self.listeners.push(Rc::new(listener));
        }
    }

    fn notify(&self) {
        for listener in &self.listeners {
            listener(self.get());
        }
    }

    fn dyn_clone(&self) -> BoxedSignal<T> {
        Box::new(self.clone())
    }
}

impl<T: 'static> Clone for StateSignal<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            listeners: self.listeners.clone(),
            listener_ids: self.listener_ids.clone(),
        }
    }
}
