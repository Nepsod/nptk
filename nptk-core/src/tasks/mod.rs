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

/// Spawns the given future.
pub fn spawn<F>(fut: F) -> impl Future<Output = F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let runner = RUNNER.load().clone();
    async move { runner.spawn(fut).await }
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
