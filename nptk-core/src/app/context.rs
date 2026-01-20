use crate::app::action::ActionCallbackManager;
use crate::app::diagnostics::Diagnostics;
use crate::app::focus::{FocusId, SharedFocusManager};
use crate::app::popup::PopupManager;
use crate::app::status_bar::StatusBarManager;
use crate::app::tooltip::TooltipRequestManager;
use crate::app::update::{Update, UpdateManager};
use crate::menu::ContextMenuState;
use crate::shortcut::ShortcutRegistry;
use crate::signal::eval::EvalSignal;
use crate::signal::fixed::FixedSignal;
use crate::signal::future::FutureSignal;
use crate::signal::memoized::MemoizedSignal;
use crate::signal::state::StateSignal;
use crate::signal::{MaybeSignal, Signal};
use crate::theme::Palette;
use crate::vgi::GpuContext;
use nptk_theme::id::WidgetId;
use std::sync::Arc;

use nptk_services::settings::SettingsRegistry;

/// Performance constants for async operations
const DEFAULT_DEBOUNCE_DELAY_MS: u64 = 300;
const DEFAULT_TIMEOUT_MS: u64 = 5000;

/// The application context for managing the application lifecycle.
#[derive(Clone)]
pub struct AppContext {
    update: UpdateManager,
    diagnostics: Diagnostics,
    gpu_context: Arc<GpuContext>,
    focus_manager: SharedFocusManager,
    pub menu_manager: ContextMenuState,
    pub shortcut_registry: ShortcutRegistry,
    pub action_callbacks: ActionCallbackManager,
    pub popup_manager: PopupManager,
    pub tooltip_manager: TooltipRequestManager,
    pub status_bar: StatusBarManager,
    pub settings: Arc<SettingsRegistry>,
    pub palette: Arc<Palette>,
}

impl AppContext {
    /// Create a new application context using the given [UpdateManager].
    pub fn new(
        update: UpdateManager,
        diagnostics: Diagnostics,
        gpu_context: Arc<GpuContext>,
        focus_manager: SharedFocusManager,
        menu_manager: ContextMenuState,
        shortcut_registry: ShortcutRegistry,
        action_callbacks: ActionCallbackManager,
        popup_manager: PopupManager,
        tooltip_manager: TooltipRequestManager,
        status_bar: StatusBarManager,
        settings: Arc<SettingsRegistry>,
        palette: Arc<Palette>,
    ) -> Self {
        Self {
            update,
            diagnostics,
            gpu_context,
            focus_manager,
            menu_manager,
            shortcut_registry,
            action_callbacks,
            popup_manager,
            tooltip_manager,
            status_bar,
            settings,
            palette,
        }
    }

    /// Get the palette for accessing theme colors and properties.
    pub fn palette(&self) -> &Palette {
        &self.palette
    }

    /// Get the [Diagnostics] of the application.
    pub fn diagnostics(&self) -> Diagnostics {
        self.diagnostics
    }

    /// Get the [GpuContext] of the application.
    pub fn gpu_context(&self) -> Arc<GpuContext> {
        self.gpu_context.clone()
    }

    /// Get the [GpuContext] of the application (backward compatibility alias).
    #[deprecated(note = "Use gpu_context() instead")]
    pub fn render_ctx(&self) -> Arc<GpuContext> {
        self.gpu_context.clone()
    }

    /// Get the [UpdateManager] of the application.
    pub fn update(&self) -> UpdateManager {
        self.update.clone()
    }

    /// Make the application exit by setting [Update::EXIT].

    pub fn exit(&self) {
        self.update.insert(Update::EXIT);
    }

    /// Hook the given [Signal] to the [UpdateManager] of this application.
    ///
    /// This makes the signal reactive, so it will notify the renderer when the inner value changes.
    pub fn hook_signal<T: Send + Sync + 'static, S: Signal<T>>(&self, signal: &S) {
        let update = self.update();

        signal.listen(Box::new(move |_| {
            update.insert(Update::EVAL);
        }));
    }

    /// Hook the given [Signal] to the [UpdateManager] of this application and return it.
    ///
    /// See [AppContext::hook_signal] for more.
    pub fn use_signal<T: Send + Sync + 'static, S: Signal<T>>(&self, signal: S) -> S {
        self.hook_signal(&signal);

        signal
    }

    /// Shortcut for creating and hooking a [StateSignal] into the application lifecycle.
    pub fn use_state<T: Send + Sync + 'static>(&self, value: T) -> StateSignal<T> {
        self.use_signal(StateSignal::new(value))
    }

    /// Shortcut for creating and hooking a [MemoizedSignal] into the application lifecycle.
    pub fn use_memoized<T: Send + Sync + 'static>(&self, value: impl Fn() -> T + Send + Sync + 'static) -> MemoizedSignal<T> {
        self.use_signal(MemoizedSignal::new(value))
    }

    /// Shortcut for creating and hooking a [FixedSignal] into the application lifecycle.
    pub fn use_fixed<T: Send + Sync + 'static>(&self, value: T) -> FixedSignal<T> {
        self.use_signal(FixedSignal::new(value))
    }

    /// Shortcut for creating and hooking an [EvalSignal] into the application lifecycle.
    pub fn use_eval<T: Send + Sync + 'static + Clone>(&self, eval: impl Fn() -> T + Send + Sync + 'static) -> EvalSignal<T> {
        self.use_signal(EvalSignal::new(eval))
    }

    /// Creates a callback signal that returns [Update] and is already hooked into the app lifecycle.
    ///
    /// This is a convenience method for creating button callbacks and other event handlers
    /// that need to return `MaybeSignal<Update>`. It eliminates the need for `.hook(&context).maybe()`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use nptk_core::app::context::AppContext;
    /// use nptk_core::app::update::Update;
    /// use nptk_core::signal::state::StateSignal;
    ///
    /// // Before:
    /// // EvalSignal::new(move || { ... }).hook(&context).maybe()
    ///
    /// // After:
    /// context.callback(move || {
    ///     // callback logic
    ///     Update::DRAW
    /// })
    /// ```
    pub fn callback(&self, f: impl Fn() -> Update + Send + Sync + 'static) -> MaybeSignal<Update> {
        let signal = EvalSignal::new(f);
        MaybeSignal::signal(Box::new(self.use_signal(signal)))
    }

    /// Shortcut for creating and hooking a [FutureSignal] into the application lifecycle.
    pub fn use_future<T, F>(&self, future: F) -> FutureSignal<T>
    where
        T: Send + Sync + Clone + 'static,
        F: std::future::Future<Output = T> + Send + 'static,
    {
        let signal = FutureSignal::new(future);
        let update = self.update.clone();
        
        // Notify the update manager when the future completes
        signal.on_complete(move || {
            update.insert(Update::EVAL | Update::DRAW);
        });

        self.use_signal(signal)
    }

    /// Spawns the given future on the background task runner.
    pub fn spawn<F>(&self, future: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        crate::tasks::spawn(future);
    }

    /// Spawns a future that triggers an app update when complete.
    pub fn spawn_with_update<F>(&self, future: F, update_type: Update)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let update_manager = self.update.clone();
        crate::tasks::spawn(async move {
            future.await;
            update_manager.insert(update_type);
        });
    }

    /// Spawns a future that triggers a redraw when complete.
    /// This is a convenience method for the common case of triggering a redraw after async work.
    pub fn spawn_with_redraw<F>(&self, future: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        self.spawn_with_update(future, Update::DRAW);
    }

    /// Spawns a future and calls a callback with the result, then triggers an update.
    /// This is useful for async operations that need to update UI state when complete.
    pub fn spawn_with_callback<F, T, C>(&self, future: F, callback: C, update_type: Update)
    where
        F: std::future::Future<Output = T> + Send + 'static,
        T: Send + 'static,
        C: FnOnce(T) + Send + 'static,
    {
        let update_manager = self.update.clone();
        crate::tasks::spawn(async move {
            let result = future.await;
            callback(result);
            update_manager.insert(update_type);
        });
    }

    /// Spawns an async data loading operation with automatic error handling.
    /// This is useful for loading data that might fail and needs UI feedback.
    pub fn spawn_data_load<F, T, S, E>(&self, future: F, on_success: S, on_error: E)
    where
        F: std::future::Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
        T: Send + 'static,
        S: FnOnce(T) + Send + 'static,
        E: FnOnce(Box<dyn std::error::Error + Send + Sync>) + Send + 'static,
    {
        let update_manager = self.update.clone();
        crate::tasks::spawn(async move {
            match future.await {
                Ok(data) => {
                    on_success(data);
                    update_manager.insert(Update::DRAW);
                },
                Err(error) => {
                    log::error!("Data loading failed: {}", error);
                    on_error(error);
                    update_manager.insert(Update::DRAW);
                },
            }
        });
    }

    /// Spawns a debounced async operation that only executes after a delay.
    /// If called again before the delay expires, the previous operation is cancelled.
    /// This is useful for operations like search-as-you-type or auto-save.
    pub fn spawn_debounced<F>(&self, delay_ms: u64, future: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let update_manager = self.update.clone();
        crate::tasks::spawn(async move {
            // Wait for the debounce delay
            smol::Timer::after(std::time::Duration::from_millis(delay_ms)).await;
            
            // Execute the operation
            future.await;
            
            // Trigger update
            update_manager.insert(Update::DRAW);
        });
    }

    /// Spawns a debounced async operation with default delay.
    pub fn spawn_debounced_default<F>(&self, future: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        self.spawn_debounced(DEFAULT_DEBOUNCE_DELAY_MS, future);
    }

    /// Spawns an async resource loading operation with caching.
    /// This is useful for loading resources like images, fonts, or other assets.
    pub fn spawn_resource_load<K, R, F, S, E>(&self, _cache_key: K, loader: F, on_success: S, on_error: E)
    where
        K: std::hash::Hash + Eq + Clone + Send + 'static,
        R: Send + 'static,
        F: std::future::Future<Output = Result<R, Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
        S: FnOnce(R) + Send + 'static,
        E: FnOnce(Box<dyn std::error::Error + Send + Sync>) + Send + 'static,
    {
        let update_manager = self.update.clone();
        crate::tasks::spawn(async move {
            match loader.await {
                Ok(resource) => {
                    on_success(resource);
                    update_manager.insert(Update::DRAW);
                },
                Err(error) => {
                    log::error!("Resource loading failed for key {:?}: {}", std::any::type_name::<K>(), error);
                    on_error(error);
                    update_manager.insert(Update::DRAW);
                },
            }
        });
    }

    /// Spawns multiple async operations concurrently and waits for all to complete.
    /// This is useful for loading multiple resources in parallel.
    pub fn spawn_concurrent_batch<F, T, C>(&self, futures: Vec<F>, on_complete: C)
    where
        F: std::future::Future<Output = T> + Send + 'static,
        T: Send + 'static,
        C: FnOnce(Vec<T>) + Send + 'static,
    {
        let update_manager = self.update.clone();
        crate::tasks::spawn(async move {
            use futures::future::join_all;
            let results = join_all(futures).await;
            on_complete(results);
            update_manager.insert(Update::DRAW);
        });
    }

    /// Spawns a batch of async operations with individual error handling.
    /// Each operation can succeed or fail independently.
    pub fn spawn_batch_with_error_handling<F, T, S, E>(&self, operations: Vec<F>, on_success: S, on_error: E)
    where
        F: std::future::Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
        T: Send + 'static,
        S: Fn(usize, T) + Send + Sync + 'static,
        E: Fn(usize, Box<dyn std::error::Error + Send + Sync>) + Send + Sync + 'static,
    {
        let update_manager = self.update.clone();
        let on_success = std::sync::Arc::new(on_success);
        let on_error = std::sync::Arc::new(on_error);
        
        crate::tasks::spawn(async move {
            use futures::future::join_all;
            let results = join_all(operations).await;
            
            for (index, result) in results.into_iter().enumerate() {
                match result {
                    Ok(data) => on_success(index, data),
                    Err(error) => {
                        log::error!("Batch operation {} failed: {}", index, error);
                        on_error(index, error);
                    },
                }
            }
            
            update_manager.insert(Update::DRAW);
        });
    }

    /// Spawns async operations with a timeout.
    /// If the operation doesn't complete within the timeout, it's cancelled.
    pub fn spawn_with_timeout<F, T, S, E>(&self, timeout_ms: u64, future: F, on_success: S, on_timeout: E)
    where
        F: std::future::Future<Output = T> + Send + 'static,
        T: Send + 'static,
        S: FnOnce(T) + Send + 'static,
        E: FnOnce() + Send + 'static,
    {
        let update_manager = self.update.clone();
        crate::tasks::spawn(async move {
            let timeout = smol::Timer::after(std::time::Duration::from_millis(timeout_ms));
            let future = Box::pin(future);
            
            match futures::future::select(future, timeout).await {
                futures::future::Either::Left((result, _)) => {
                    // Operation completed before timeout
                    on_success(result);
                },
                futures::future::Either::Right((_, _)) => {
                    // Timeout occurred
                    log::warn!("Operation timed out after {}ms", timeout_ms);
                    on_timeout();
                },
            }
            
            update_manager.insert(Update::DRAW);
        });
    }

    /// Spawns async operations with default timeout.
    pub fn spawn_with_default_timeout<F, T, S, E>(&self, future: F, on_success: S, on_timeout: E)
    where
        F: std::future::Future<Output = T> + Send + 'static,
        T: Send + 'static,
        S: FnOnce(T) + Send + 'static,
        E: FnOnce() + Send + 'static,
    {
        self.spawn_with_timeout(DEFAULT_TIMEOUT_MS, future, on_success, on_timeout);
    }

    /// Get the shared focus manager.
    pub fn focus_manager(&self) -> SharedFocusManager {
        self.focus_manager.clone()
    }

    /// Set focus to a specific widget.
    pub fn set_focus(&self, focus_id: Option<FocusId>) {
        if let Ok(mut manager) = self.focus_manager.lock() {
            manager.set_focus(focus_id);
            self.update.insert(Update::FOCUS | Update::DRAW);
        }
    }

    /// Get the currently focused widget ID.
    pub fn get_focused_widget(&self) -> Option<FocusId> {
        self.focus_manager
            .lock()
            .ok()
            .and_then(|manager| manager.get_focused_widget())
    }

    /// Move focus to the next widget in tab order.
    pub fn focus_next(&self) {
        if let Ok(mut manager) = self.focus_manager.lock() {
            manager.focus_next();
            self.update.insert(Update::FOCUS | Update::DRAW);
        }
    }

    /// Move focus to the previous widget in tab order.
    pub fn focus_previous(&self) {
        if let Ok(mut manager) = self.focus_manager.lock() {
            manager.focus_previous();
            self.update.insert(Update::FOCUS | Update::DRAW);
        }
    }

    /// Clear focus from all widgets.
    pub fn clear_focus(&self) {
        if let Ok(mut manager) = self.focus_manager.lock() {
            manager.clear_focus();
            self.update.insert(Update::FOCUS | Update::DRAW);
        }
    }

    /// Request to show a tooltip.
    ///
    /// The tooltip will be shown after a delay (700ms by default).
    pub fn request_tooltip_show(
        &self,
        text: String,
        source_widget_id: WidgetId,
        cursor_pos: (f64, f64),
    ) {
        self.tooltip_manager.request_show(text, source_widget_id, cursor_pos);
        // Request redraw to process tooltip requests
        self.update.insert(Update::DRAW);
    }

    /// Request to hide the current tooltip.
    pub fn request_tooltip_hide(&self) {
        self.tooltip_manager.request_hide();
        // Request redraw to process tooltip requests
        self.update.insert(Update::DRAW);
    }

    /// Set the status bar text.
    ///
    /// This is typically called by widgets (like buttons) when hovered to show
    /// a status tip in the status bar.
    pub fn set_status_bar_text(&self, text: String) {
        self.status_bar.set_text(text);
    }

    /// Clear the status bar text.
    pub fn clear_status_bar_text(&self) {
        self.status_bar.clear();
    }
}
