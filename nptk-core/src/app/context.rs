use crate::app::diagnostics::Diagnostics;
use crate::app::focus::{FocusId, SharedFocusManager};
use crate::app::update::{Update, UpdateManager};
use crate::signal::actor::ActorSignal;
use crate::signal::eval::EvalSignal;
use crate::signal::fixed::FixedSignal;
use crate::signal::memoized::MemoizedSignal;
use crate::signal::rw::RwSignal;
use crate::signal::state::StateSignal;
use crate::signal::Signal;
use crate::vgi::GpuContext;
use std::sync::Arc;

/// The application context for managing the application lifecycle.
#[derive(Clone)]
pub struct AppContext {
    update: UpdateManager,
    diagnostics: Diagnostics,
    gpu_context: Arc<GpuContext>,
    focus_manager: SharedFocusManager,
}

impl AppContext {
    /// Create a new application context using the given [UpdateManager].
    pub fn new(
        update: UpdateManager,
        diagnostics: Diagnostics,
        gpu_context: Arc<GpuContext>,
        focus_manager: SharedFocusManager,
    ) -> Self {
        Self {
            update,
            diagnostics,
            gpu_context,
            focus_manager,
        }
    }

    /// Get the [Diagnostics] of the application.
    pub fn diagnostics(&self) -> Diagnostics {
        self.diagnostics.clone()
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

    /// Hook the given [Signal] to the [UpdateManager] of this application and return it inside an [Arc].
    ///
    /// See [AppContext::hook_signal] for more.
    pub fn use_signal<T: 'static, S: Signal<T>>(&self, mut signal: S) -> Arc<S> {
        self.hook_signal(&mut signal);

        Arc::new(signal)
    }

    /// Shortcut for creating and hooking a [StateSignal] into the application lifecycle.
    pub fn use_state<T: 'static>(&self, value: T) -> Arc<StateSignal<T>> {
        self.use_signal(StateSignal::new(value))
    }

    /// Shortcut for creating and hooking a [MemoizedSignal] into the application lifecycle.
    pub fn use_memoized<T: 'static>(
        &self,
        value: impl Fn() -> T + 'static,
    ) -> Arc<MemoizedSignal<T>> {
        self.use_signal(MemoizedSignal::new(value))
    }

    /// Shortcut for creating and hooking a [FixedSignal] into the application lifecycle.
    pub fn use_fixed<T: 'static>(&self, value: T) -> Arc<FixedSignal<T>> {
        self.use_signal(FixedSignal::new(value))
    }

    /// Shortcut for creating and hooking an [EvalSignal] into the application lifecycle.
    pub fn use_eval<T: 'static>(&self, eval: impl Fn() -> T + 'static) -> Arc<EvalSignal<T>> {
        self.use_signal(EvalSignal::new(eval))
    }

    /// Shortcut for creating and hooking a [RwSignal] into the application lifecycle.
    pub fn use_rw<T: 'static>(&self, value: T) -> Arc<RwSignal<T>> {
        self.use_signal(RwSignal::new(value))
    }

    /// Shortcut for creating and hooking an [ActorSignal] into the application lifecycle.
    pub fn use_actor<T: 'static>(&self, value: T) -> Arc<ActorSignal<T>> {
        self.use_signal(ActorSignal::new(value))
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
}
