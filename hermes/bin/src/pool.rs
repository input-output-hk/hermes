//! Global thread pool for parallel WASM module execution.
//!
//! This module initializes a global Rayon thread pool
//! for running WASM modules in parallel. The pool uses
//! work-stealing for efficient load balancing across
//! available threads. To avoid saturating the system,
//! the pool leaves a small margin of CPU capacity
//! unused — typically 2 threads are reserved for the
//! main application flow and event loop.
//!
//! Each event may execute multiple WASM modules
//! concurrently within this pool.

use std::{
    sync::{Condvar, Mutex, atomic::AtomicUsize},
    thread::available_parallelism,
};

use anyhow::{Context, Result};
use rayon::ThreadPoolBuilder;

/// Global counter of currently running tasks.
static TASK_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Synchronization primitives for waiting until all tasks finish.
static TASK_WAIT: (Mutex<()>, Condvar) = (Mutex::new(()), Condvar::new());

/// Get a reference to the global `(Mutex, Condvar)` tuple.
fn get_task_wait() -> &'static (Mutex<()>, Condvar) {
    &TASK_WAIT
}

/// Initialize the global Rayon thread pool
///
/// The number of threads is set to (CPU cores - 2).
/// Reserving two cores helps the main thread and event loop
/// remain responsive, reducing CPU contention with compute-heavy
/// WASM tasks. At least one worker thread is always created.
pub(crate) fn init() -> Result<()> {
    let available_threads = available_parallelism()
        .map(|num_threads| num_threads.get().saturating_sub(2).max(1))
        .unwrap_or(1);

    ThreadPoolBuilder::new()
        .num_threads(available_threads)
        .build_global()
        .context("Failed to build global Rayon thread pool")?;

    Ok(())
}

/// Execute a task in the global thread pool
///
/// This function increments a global task counter, runs the
/// given task in Rayon’s pool, and decrements the counter
/// when the task finishes. If it was the last task, it
/// notifies any threads waiting in `terminate()`.
///
/// # Arguments
///
/// * `task` - The closure to run concurrently
pub(crate) fn execute<F>(task: F)
where
    F: FnOnce() + Send + 'static,
{
    TASK_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    rayon::spawn(move || {
        task();

        let prev = TASK_COUNTER.fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
        if prev == 1 {
            // Last task finished — notify waiting threads
            let (lock, no_active_tasks_cv) = get_task_wait();
            if let Ok(_guard) = lock.lock() {
                no_active_tasks_cv.notify_all();
            }
        }
    });
}

/// Wait for all currently submitted tasks to finish
///
/// This function blocks until `TASK_COUNTER` reaches zero,
/// i.e., until all tasks submitted via `execute()` have
/// completed.
pub(crate) fn terminate() {
    let (lock, no_active_tasks_cv) = get_task_wait();
    if let Ok(mut guard) = lock.lock() {
        while TASK_COUNTER.load(std::sync::atomic::Ordering::Acquire) != 0 {
            if let Ok(g) = no_active_tasks_cv.wait(guard) {
                guard = g;
            } else {
                // If waiting on Condvar fails, exit loop
                break;
            }
        }
    }
}
