use crate::app::context::AppContext;
use crate::app::runner::MayRunner;
use crate::config::MayConfig;
use crate::plugin::PluginManager;
use crate::widget::Widget;

/// Contains diagnostics data for the application.
pub mod diagnostics;

/// Contains the font context structure.
pub mod font_ctx;

/// Contains async image loading utilities.
pub mod image_loader;

/// Contains the application handler.
pub mod handler;

/// Contains the application information structure.
pub mod info;

/// Contains the update mode bitflag.
pub mod update;

/// Contains the [AppContext] structure for access to the application lifecycle.
pub mod context;

/// Contains the [MayRunner] structure to create and run an application using `winit`.
pub mod runner;

/// Contains the focus management system.
pub mod focus;

/// Contains the XKB keymap manager for Wayland keyboard handling.
#[cfg(all(target_os = "linux", feature = "wayland"))]
pub mod keymap;

/// Contains the overlay and popup management system.
pub mod popup;

/// Contains the tooltip management system.
pub mod tooltip;

/// Contains the action callback system for status tips and hover tracking.
pub mod action;

/// The main application interface.
///
/// Contains basic functions for the [MayRunner] to create and run an application.
pub trait Application: Sized {
    /// The global state of the application.
    type State;

    /// Renders/builds the application's widgets.
    ///
    /// This function will be passed to the [MayRunner] to create and run the application.
    fn build(context: AppContext, state: Self::State) -> impl Widget;

    /// Returns the [MayConfig] for the application.
    fn config(&self) -> MayConfig {
        MayConfig::default()
    }

    /// Builds and returns the [PluginManager] for the application.
    fn plugins(&self) -> PluginManager {
        PluginManager::new()
    }

    /// Runs the application using the [MayRunner].
    ///
    /// Override this method if you want to use a custom event loop.
    fn run(self, state: Self::State) {
        MayRunner::new(self.config()).run(state, Self::build, self.plugins());
    }
}
