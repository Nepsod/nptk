use crate::app::context::AppContext;
use crate::config::MayConfig;
use crate::plugin::PluginManager;
use crate::widget::Widget;
use async_trait::async_trait;

/// The main application interface for async-first applications.
#[async_trait]
pub trait AsyncApplication: Sized {
    /// The global state of the application.
    type State: Send + Sync + 'static;

    /// Renders/builds the application's widgets.
    fn build(context: AppContext, state: Self::State) -> impl Widget;

    /// Returns the [MayConfig] for the application.
    fn config(&self) -> MayConfig {
        MayConfig::default()
    }

    /// Builds and returns the [PluginManager] for the application.
    fn plugins(&self) -> PluginManager {
        PluginManager::new()
    }

    /// Asynchronously initializes the application state.
    async fn initialize() -> Self::State;

    /// Runs the application using the [MayRunner].
    /// 
    /// This will block the current thread while initializing the state asynchronously,
    /// then start the application event loop.
    fn run(self) {
        let state = crate::tasks::block_on(Self::initialize());
        crate::app::runner::MayRunner::new(self.config())
            .run(state, Self::build, self.plugins());
    }
}
