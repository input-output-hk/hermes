//! Thread pool implementation for parallel WASM module execution.
//!
//! This module provides a thread pool that can
//! execute WASM modules in parallel,
//! with work-stealing semantics and graceful
//! handling of multiple modules per event.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex, OnceLock,
    },
    thread::{available_parallelism, JoinHandle},
};

use anyhow::anyhow;
use crossbeam_channel::{unbounded, Receiver, Sender};

/// Singleton instance of the Hermes thread pool.
pub(crate) static THREAD_POOL_INSTANCE: OnceLock<Pool> = OnceLock::new();

/// Failed when thread pool already been initialized.
#[derive(thiserror::Error, Debug, Clone)]
#[error("Thread pool already been initialized.")]
pub(crate) struct AlreadyInitializedError;

/// This struct represents single thread worker, who
/// is responsible for processing new incoming tasks.
struct Worker {
    /// Handle to process termination.
    handle: JoinHandle<()>,
}

/// Wrapper for task to process termination.
enum Message {
    /// Worker task.
    Job(Box<dyn FnOnce() -> anyhow::Result<()> + Send + 'static>),
    /// Termination signal.
    Shutdown,
}

/// Spawns new pool thread which reads and executes tasks one by one.
fn spawn(receiver: Receiver<Message>) -> Worker {
    let handle = std::thread::spawn(move || {
        while let Ok(task) = receiver.recv() {
            match task {
                // Execute the task and log any errors
                Message::Job(task) => {
                    if let Err(err) = task() {
                        tracing::error!("Task execution failed: {err}");
                    }
                },
                Message::Shutdown => break,
            }
        }
    });

    Worker { handle }
}

pub(crate) struct Pool {
    /// Workers handles.
    workers: Mutex<Vec<Worker>>,
    /// Task queue sender handle.
    queue: Sender<Message>,
    /// Has `terminate()` been already called
    terminated: AtomicBool,
}

impl Pool {
    /// Execute a task on the thread pool
    pub(crate) fn execute(
        &self,
        task: Box<dyn FnOnce() -> anyhow::Result<()> + Send + 'static>,
    ) -> anyhow::Result<()> {
        if self.terminated.load(Ordering::Acquire) {
            return Err(anyhow::anyhow!("Thread pool is shut down"));
        }

        self.queue
            .send(Message::Job(task))
            .map_err(|_| anyhow::anyhow!("Thread pool is shut down"))?;
        Ok(())
    }

    /// Terminates the thread pool.]
    fn terminate(&self) -> anyhow::Result<()> {
        if self.terminated.swap(true, Ordering::AcqRel) {
            return Ok(());
        }

        let mut workers = self
            .workers
            .lock()
            .map_err(|_err| anyhow!("failed to lock mutex"))?;
        // Ask each worker to shut down.
        for _ in 0..workers.len() {
            // If send fails, receivers are already gone; that's fine.
            self.queue
                .send(Message::Shutdown)
                .map_err(|err| anyhow!("failed to send shutdown signal: {err}"))?;
        }

        let mut mutex_replacement = vec![];
        std::mem::swap(&mut mutex_replacement, &mut *workers);

        for worker in mutex_replacement {
            worker
                .handle
                .join()
                .map_err(|_err| anyhow!("failed to terminate worker"))?;
        }

        Ok(())
    }
}

impl Default for Pool {
    /// Creates pool with `available_parallelism() - 2` threads
    /// if `available_parallelism() > 2` or just `1`.
    ///
    /// Since one thread is used for main execution flow and one for event loop.
    fn default() -> Self {
        let (sender, receiver) = unbounded::<Message>();

        let available_threads = available_parallelism()
            .map(|num_threads| {
                num_threads
                    .get()
                    .checked_sub(2)
                    .filter(|num_threads| *num_threads > 0)
            })
            .unwrap_or(Some(1))
            .unwrap_or(1);

        let workers = Mutex::new(
            (0..available_threads)
                .map(|_| spawn(receiver.clone()))
                .collect(),
        );

        Self {
            workers,
            queue: sender,
            terminated: AtomicBool::default(),
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
        .ok_or_else(|| anyhow::anyhow!("Thread pool is not initialized"))?;

    pool.execute(task)
}

/// Terminates the global thread pool
pub(crate) fn terminate() -> anyhow::Result<()> {
    let pool = THREAD_POOL_INSTANCE
        .get()
        .ok_or_else(|| anyhow::anyhow!("Thread pool is not initialized"))?;

    pool.terminate()?;
    Ok(())
}
