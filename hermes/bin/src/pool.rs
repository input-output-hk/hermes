//! Thread pool implementation for parallel WASM module execution.
//!
//! This module provides a thread pool that can
//! execute WASM modules in parallel,
//! with work-stealing semantics and graceful
//! handling of multiple modules per event.

use std::{
    sync::OnceLock,
    thread::{available_parallelism, JoinHandle},
};

use crossbeam_channel::{unbounded, Receiver, Sender};

/// Singleton instance of the
/// Hermes thread pool.
pub(crate) static THREAD_POOL_INSTANCE: OnceLock<Pool> = OnceLock::new();

/// Failed when thread pool
/// already been initialized.
#[derive(thiserror::Error, Debug, Clone)]
#[error("Thread pool already been initialized.")]
pub(crate) struct AlreadyInitializedError;

/// This struct represents single thread worker, who
/// is responsible for processing new incoming tasks.
struct Worker {
    /// Handle to process termination.
    handle: JoinHandle<()>,
}

/// Spawns new pool thread which reads and executes tasks one by one.
fn spawn(receiver: Receiver<Box<dyn FnOnce() -> anyhow::Result<()> + Send + 'static>>) -> Worker {
    let handle = std::thread::spawn(move || {
        while let Ok(task) = receiver.recv() {
            // Execute
            // the task
            // and log
            // any errors
            if let Err(err) = task() {
                tracing::error!("Task execution failed: {err}");
            }
        }
    });

    Worker { handle }
}

pub(crate) struct Pool {
    /// Workers handles.
    #[allow(dead_code)]
    workers: Vec<Worker>,
    /// Task queue sender handle.
    queue: Sender<Box<dyn FnOnce() -> anyhow::Result<()> + Send + 'static>>,
}

impl Pool {
    /// Execute a task on the thread pool
    pub(crate) fn execute(
        &self,
        task: Box<dyn FnOnce() -> anyhow::Result<()> + Send + 'static>,
    ) -> anyhow::Result<()> {
        self.queue
            .send(task)
            .map_err(|_| anyhow::anyhow!("Thread pool is shut down"))?;
        Ok(())
    }

    // TODO: fix termination mechanism
    /// Terminates the thread pool.
    #[allow(dead_code)]
    fn terminate(self) {
        // Drop the sender to signal workers to exit
        drop(self.queue);

        // Wait for all workers to finish
        for worker in self.workers {
            if let Err(err) = worker.handle.join() {
                tracing::error!("Worker thread panicked: {err:?}");
            }
        }
    }
}

impl Default for Pool {
    /// Creates pool with `available_parallelism() - 2` threads
    /// if `available_parallelism() > 2` or just `1`.
    ///
    /// Since one thread is used for main execution flow and one for event loop.
    fn default() -> Self {
        let (sender, receiver) =
            unbounded::<Box<dyn FnOnce() -> anyhow::Result<()> + Send + 'static>>();

        let available_threads = available_parallelism()
            .map(|num_threads| {
                num_threads
                    .get()
                    .checked_sub(2)
                    .filter(|num_threads| *num_threads > 0)
            })
            .unwrap_or(Some(1))
            .unwrap_or(1);

        let workers = (0..available_threads)
            .map(|_| spawn(receiver.clone()))
            .collect();

        Self {
            workers,
            queue: sender,
        }
    }
}

/// Initialize the global thread pool
pub(crate) fn init() -> anyhow::Result<()> {
    THREAD_POOL_INSTANCE
        .set(Pool::default())
        .map_err(|_| AlreadyInitializedError)?;

    Ok(())
}

/// Execute a task on the global thread pool
pub(crate) fn execute(
    task: Box<dyn FnOnce() -> anyhow::Result<()> + Send + 'static>
) -> anyhow::Result<()> {
    let pool = THREAD_POOL_INSTANCE
        .get()
        .ok_or_else(|| anyhow::anyhow!("Thread pool not initialized"))?;

    pool.execute(task)
}

/// Terminates the global thread pool
pub(crate) fn terminate() -> anyhow::Result<()> {
    let _pool = THREAD_POOL_INSTANCE
        .get()
        .ok_or_else(|| anyhow::anyhow!("Thread pool not initialized"))?;

    // TODO: fix termination
    // pool.terminate();
    Ok(())
}
