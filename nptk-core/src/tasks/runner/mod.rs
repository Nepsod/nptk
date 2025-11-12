//! Task runner implementations.
#[cfg(feature = "tokio-runner")]
use self::tokio_runner::TokioRunner;
use std::future::Future;

/// An abstraction over a task runner.
#[derive(Debug)]
pub enum TaskRunner {
    /// The tokio task runner.
    #[cfg(feature = "tokio-runner")]
    Tokio(TokioRunner),
    /// No task runner selected.
    None,
}

impl TaskRunner {
    /// Blocks on the given future.
    pub fn block_on<F>(&self, fut: F) -> F::Output
    where
        F: Future,
    {
        match self {
            #[cfg(feature = "tokio-runner")]
            TaskRunner::Tokio(runner) => runner.block_on(fut),
            TaskRunner::None => {
                // Since there is no runtime, we can just block on the future using pollster.
                pollster::block_on(fut)
            },
        }
    }

    /// Spawns the given future.
    pub async fn spawn<F>(&self, fut: F) -> F::Output
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        match self {
            #[cfg(feature = "tokio-runner")]
            TaskRunner::Tokio(runner) => runner.spawn(fut).await,
            TaskRunner::None => {
                panic!("No task runner initialized! Please specify a tasks configuration in `MayConfig`.")
            },
        }
    }

    /// Spawns the given blocking function.
    pub async fn spawn_blocking<F, R>(&self, fut: F) -> R
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        match self {
            #[cfg(feature = "tokio-runner")]
            TaskRunner::Tokio(runner) => runner.spawn_blocking(fut).await,
            TaskRunner::None => {
                panic!("No task runner initialized! Please specify a tasks configuration in `MayConfig`.")
            },
        }
    }
}
#[cfg(feature = "tokio-runner")]
pub mod tokio_runner;
