//! An executor for running tasks in the background.
use crate::config::TasksConfig;
use arc_swap::ArcSwap;
use runner::TaskRunner;
use std::future::Future;
use std::sync::{Arc, LazyLock};

/// An abstraction over a task runner.
pub mod runner;

static RUNNER: LazyLock<ArcSwap<TaskRunner>> =
    LazyLock::new(|| ArcSwap::new(Arc::new(TaskRunner::None)));

/// Initializes the task runner.
pub fn init(config: TasksConfig) {
    #[cfg(feature = "tokio-runner")]
    let runner = TaskRunner::Tokio(runner::tokio_runner::TokioRunner::new(config));
    #[cfg(any(not(feature = "tokio-runner")))]
    let runner = TaskRunner::None;
    RUNNER.store(Arc::new(runner));
}

/// Spawns the given future (fire-and-forget).
pub fn spawn<F>(fut: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    let runner = RUNNER.load().clone();
    runner.spawn_detached(fut);
}

/// Blocks on the given future.
pub fn block_on<F>(fut: F) -> F::Output
where
    F: Future,
{
    RUNNER.load().block_on(fut)
}

/// Spawns the given blocking function.
pub fn spawn_blocking<F, R>(fut: F) -> impl Future<Output = R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let runner = RUNNER.load().clone();
    async move { runner.spawn_blocking(fut).await }
}

/// Shuts down the task runner gracefully.
/// This should be called during application shutdown to prevent hanging.
pub fn shutdown() {
    log::debug!("Shutting down task runner...");
    
    // Take the current runner and replace with None
    let current_runner = RUNNER.swap(Arc::new(TaskRunner::None));
    
    // Shutdown the runner if it's not None
    match Arc::try_unwrap(current_runner) {
        Ok(runner) => runner.shutdown(),
        Err(_) => {
            log::warn!("Could not shutdown task runner - still has active references");
        }
    }
}
