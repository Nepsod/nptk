use crate::config::TasksConfig;
use std::future::Future;
use tokio::runtime::{Builder, Runtime};

/// A task runner using [tokio] as runtime.
#[derive(Debug)]
pub struct TokioRunner {
    rt: Runtime,
}

impl TokioRunner {
    /// Initializes the tokio task runner with the given config.
    pub(crate) fn new(config: TasksConfig) -> Self {
        let mut builder = if config.workers.get() == 1 {
            Builder::new_current_thread()
        } else {
            Builder::new_multi_thread()
        };

        let rt = builder
            .enable_all()
            .worker_threads(config.workers.get())
            .thread_stack_size(config.stack_size)
            .build()
            .expect("Failed to create tokio runtime");

        Self { rt }
    }

    /// Blocks on the given future.
    pub(crate) fn block_on<F>(&self, fut: F) -> F::Output
    where
        F: Future,
    {
        self.rt.block_on(fut)
    }

    /// Spawns the given future (fire-and-forget).
    /// Uses smol to avoid keeping the tokio runtime alive indefinitely.
    pub(crate) fn spawn_detached<F>(&self, fut: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        // Use smol::spawn to avoid keeping tokio runtime alive
        // Tokio runtimes can keep threads alive even after tasks complete,
        // causing the process to hang. smol tasks complete naturally.
        smol::spawn(fut).detach();
    }

    /// Spawns the given future and waits for its result.
    pub(crate) async fn spawn<F>(&self, fut: F) -> F::Output
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.rt.spawn(fut).await.expect("Failed to spawn task")
    }

    /// Spawns the given blocking function.
    pub(crate) async fn spawn_blocking<F, R>(&self, fut: F) -> R
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        self.rt
            .spawn_blocking(fut)
            .await
            .expect("Failed to spawn task")
    }
}
