use crate::reference::Ref;
use crate::signal::async_state::AsyncState;
use crate::signal::{BoxedSignal, Listener, Signal};
use crate::tasks;
use std::sync::{Arc, RwLock};

/// A signal that wraps a Future and updates its value when the future completes.
pub struct FutureSignal<T: Send + Sync + 'static> {
    state: Arc<RwLock<AsyncState<T>>>,
    listeners: Arc<RwLock<Vec<Listener<AsyncState<T>>>>>,
    /// Callback to notify when the future completes.
    notify_callback: Arc<RwLock<Option<Box<dyn Fn() + Send + Sync>>>>,
}

impl<T: Send + Sync + 'static> FutureSignal<T> {
    /// Creates a new future signal.
    pub fn new<F>(future: F) -> Self
    where
        F: std::future::Future<Output = T> + Send + 'static,
    {
        let state = Arc::new(RwLock::new(AsyncState::Loading));
        let listeners = Arc::new(RwLock::new(Vec::new()));
        let notify_callback: Arc<RwLock<Option<Box<dyn Fn() + Send + Sync>>>> =
            Arc::new(RwLock::new(None));

        let state_clone = state.clone();
        let callback_clone = notify_callback.clone();

        tasks::spawn(async move {
            let result = future.await;

            {
                let mut write = state_clone.write().unwrap();
                *write = AsyncState::Ready(result);
            }

            if let Some(callback) = callback_clone.read().unwrap().as_ref() {
                callback();
            }
        });

        Self {
            state,
            listeners,
            notify_callback,
        }
    }

    /// Sets a callback to be invoked when the future completes.
    pub fn on_complete<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        let mut lock = self.notify_callback.write().unwrap();
        *lock = Some(Box::new(callback));
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
            notify_callback: self.notify_callback.clone(),
        }
    }
}
