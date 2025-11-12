use crate::app::context::AppContext;
use crate::app::runner::MayRunner;
use crate::config::MayConfig;
use crate::plugin::PluginManager;
#[cfg(feature = "vello")]
use crate::widget::Widget;
use nptk_theme::theme::Theme;

/// Contains diagnostics data for the application.
pub mod diagnostics;

/// Contains the font context structure.
pub mod font_ctx;

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

/// Contains the overlay and popup management system.

/// The main application interface.
///
/// Contains basic functions for the [MayRunner] to create and run an application.
pub trait Application: Sized {
    /// The theme of the application and its widgets.
    ///
    /// See [nptk_theme::theme] for built-in themes.
    type Theme: Theme + Default;

    /// The global state of the application.
    type State;

    /// Renders/builds the application's widgets.
    ///
    /// This function will be passed to the [MayRunner] to create and run the application.
    #[cfg(feature = "vello")]
    fn build(context: AppContext, state: Self::State) -> impl Widget;

    /// Returns the [MayConfig] for the application.
    fn config(&self) -> MayConfig<Self::Theme> {
        MayConfig {
            theme: Self::Theme::default(),
            ..Default::default()
        }
    }

    /// Builds and returns the [PluginManager] for the application.
    fn plugins(&self) -> PluginManager<Self::Theme> {
        PluginManager::new()
    }

    /// Runs the application using the [MayRunner].
    ///
    /// Override this method if you want to use a custom event loop.
    #[cfg(feature = "vello")]
    fn run(self, state: Self::State) {
        MayRunner::<Self::Theme>::new(self.config()).run(state, Self::build, self.plugins());
    }
}
