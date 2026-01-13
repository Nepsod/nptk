use crate::app::diagnostics::Diagnostics;
use crate::app::focus::{FocusId, SharedFocusManager};
use crate::app::popup::PopupManager;
use crate::app::tooltip::TooltipRequestManager;
use crate::app::update::{Update, UpdateManager};
use crate::menu::ContextMenuState;
use crate::signal::eval::EvalSignal;
use crate::signal::fixed::FixedSignal;
use crate::signal::future::FutureSignal;
use crate::signal::memoized::MemoizedSignal;
use crate::signal::state::StateSignal;
use crate::signal::Signal;
use crate::vgi::GpuContext;
use nptk_theme::id::WidgetId;
use std::sync::Arc;

use nptk_services::settings::SettingsRegistry;

/// The application context for managing the application lifecycle.
#[derive(Clone)]
pub struct AppContext {
    update: UpdateManager,
    diagnostics: Diagnostics,
    gpu_context: Arc<GpuContext>,
    focus_manager: SharedFocusManager,
    pub menu_manager: ContextMenuState,
    pub popup_manager: PopupManager,
    pub tooltip_manager: TooltipRequestManager,
    pub settings: Arc<SettingsRegistry>,
}

impl AppContext {
    /// Create a new application context using the given [UpdateManager].
    pub fn new(
        update: UpdateManager,
        diagnostics: Diagnostics,
        gpu_context: Arc<GpuContext>,
        focus_manager: SharedFocusManager,
        menu_manager: ContextMenuState,
        popup_manager: PopupManager,
        tooltip_manager: TooltipRequestManager,
        settings: Arc<SettingsRegistry>,
    ) -> Self {
        Self {
            update,
            diagnostics,
            gpu_context,
            focus_manager,
            menu_manager,
            popup_manager,
            tooltip_manager,
            settings,
        }
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
    pub fn hook_signal<T: 'static, S: Signal<T>>(&self, signal: &mut S) {
        let update = self.update();

        signal.listen(Box::new(move |_| {
            update.insert(Update::EVAL);
        }));
    }

    /// Hook the given [Signal] to the [UpdateManager] of this application and return it.
    ///
    /// See [AppContext::hook_signal] for more.
    pub fn use_signal<T: 'static, S: Signal<T>>(&self, mut signal: S) -> S {
        self.hook_signal(&mut signal);

        signal
    }

    /// Shortcut for creating and hooking a [StateSignal] into the application lifecycle.
    pub fn use_state<T: 'static>(&self, value: T) -> StateSignal<T> {
        self.use_signal(StateSignal::new(value))
    }

    /// Shortcut for creating and hooking a [MemoizedSignal] into the application lifecycle.
    pub fn use_memoized<T: 'static>(&self, value: impl Fn() -> T + 'static) -> MemoizedSignal<T> {
        self.use_signal(MemoizedSignal::new(value))
    }

    /// Shortcut for creating and hooking a [FixedSignal] into the application lifecycle.
    pub fn use_fixed<T: 'static>(&self, value: T) -> FixedSignal<T> {
        self.use_signal(FixedSignal::new(value))
    }

    /// Shortcut for creating and hooking an [EvalSignal] into the application lifecycle.
    pub fn use_eval<T: 'static>(&self, eval: impl Fn() -> T + 'static) -> EvalSignal<T> {
        self.use_signal(EvalSignal::new(eval))
    }

    /// Shortcut for creating and hooking a [FutureSignal] into the application lifecycle.
    pub fn use_future<T, F>(&self, future: F) -> FutureSignal<T>
    where
        T: Send + Sync + 'static,
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
}
