use crate::app::update::{Update, UpdateManager};
use crate::reference::Ref;
use crate::signal::async_state::AsyncState;
use crate::signal::{BoxedSignal, Listener, Signal};
use crate::tasks;
use std::sync::{Arc, RwLock};

/// A signal that wraps a Future and updates its value when the future completes.
pub struct FutureSignal<T: Send + Sync + 'static> {
    state: Arc<RwLock<AsyncState<T>>>,
    listeners: Arc<RwLock<Vec<Listener<AsyncState<T>>>>>,
    update_manager: Arc<RwLock<Option<UpdateManager>>>,
}

impl<T: Send + Sync + 'static> FutureSignal<T> {
    /// Creates a new future signal.
    pub fn new<F>(future: F) -> Self
    where
        F: std::future::Future<Output = T> + Send + 'static,
    {
        let state = Arc::new(RwLock::new(AsyncState::Loading));
        let listeners = Arc::new(RwLock::new(Vec::new()));
        let update_manager: Arc<RwLock<Option<UpdateManager>>> = Arc::new(RwLock::new(None));

        let state_clone = state.clone();
        let update_clone = update_manager.clone();

        tasks::spawn(async move {
            let result = future.await;

            {
                let mut write = state_clone.write().unwrap();
                *write = AsyncState::Ready(result);
            }

            if let Some(manager) = update_clone.read().unwrap().as_ref() {
                manager.insert(Update::EVAL | Update::DRAW);
            }
        });

        Self {
            state,
            listeners,
            update_manager,
        }
    }

    /// Sets the update manager to trigger when the future completes.
    pub fn set_update_manager(&self, manager: UpdateManager) {
        let mut lock = self.update_manager.write().unwrap();
        *lock = Some(manager);
    }
}

impl<T: Send + Sync + 'static> Signal<AsyncState<T>> for FutureSignal<T> {
    fn get(&self) -> Ref<'_, AsyncState<T>> {
        let lock = self.state.read().unwrap();
        Ref::ReadGuard(lock)
    }

    fn set_value(&self, _value: AsyncState<T>) {
        // FutureSignal is read-only from the outside perspective
    }

    fn listen(&mut self, listener: Listener<AsyncState<T>>) {
        self.listeners.write().unwrap().push(listener);
    }

    fn notify(&self) {
        let listeners = self.listeners.read().unwrap();
        for listener in listeners.iter() {
            listener(self.get());
        }
    }

    fn dyn_clone(&self) -> BoxedSignal<AsyncState<T>> {
        Box::new(self.clone())
    }
}

impl<T: Send + Sync + 'static> Clone for FutureSignal<T> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            listeners: self.listeners.clone(),
            update_manager: self.update_manager.clone(),
        }
    }
}
