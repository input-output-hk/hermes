//! Hermes general event definition

use std::sync::{Arc, Condvar, Mutex};

use wasmtime::Store;

use super::{Hermes, HermesState};

/// A trait for defining the behavior of payloads carried by HermesEvents.
///
/// # Examples
///
/// ```
/// // Define a type that implements the HermesEventPayload trait
/// struct YourPayload;
///
/// // Implement the required methods for the payload
/// impl HermesEventPayload for YourPayload {
///     fn event_name(&self) -> &str {
///         // Implement the method to return the event name
///     }
///
///     fn execute(&self, bindings: &Hermes, store: &mut Store<HermesState>) -> anyhow::Result<()> {
///         // Implement the method to execute the payload's behavior
///     }
/// }
/// ```
pub(crate) trait HermesEventPayload {
    /// Returns the name of the event associated with the payload.
    fn event_name(&self) -> &str;

    /// Executes the behavior associated with the payload, using the provided Hermes
    /// bindings and state store.
    ///
    /// # Arguments
    ///
    /// * `bindings` - The Hermes instance to use for executing the payload's behavior.
    /// * `store` - A mutable reference to the store containing HermesState.
    ///
    /// # Returns
    ///
    /// An `anyhow::Result` indicating the success or failure of the payload execution.
    fn execute(&self, bindings: &Hermes, store: &mut Store<HermesState>) -> anyhow::Result<()>;
}

/// A generic event type that can carry a payload and optionally provide synchronization
/// using a Condvar.
pub(crate) struct HermesEvent<T: HermesEventPayload + Send + Sync> {
    /// An optional Arc containing a Mutex and Condvar for synchronization.
    cv: Option<Arc<(Mutex<bool>, Condvar)>>,

    /// The payload carried by the HermesEvent.
    payload: T,
}

/// A generic event type that can carry a payload and optionally provide synchronization
/// using a Condvar.
///
/// # Examples
///
/// ```
/// use std::sync::{Arc, Condvar, Mutex};
///
/// // Define a payload type
/// struct YourPayload;
///
/// // Implement the HermesEventPayload trait for the payload type
/// impl HermesEventPayload for YourPayload {
///     // Implement the required methods and associated types
/// }
///
/// // Create a new HermesEvent with synchronization enabled
/// let event_with_sync = HermesEvent::new(YourPayload, true);
///
/// // Notify and wait on the event
/// // Typically from different threads
/// event_with_sync.notify();
/// event_with_sync.wait();
/// ```
impl<T: HermesEventPayload + Send + Sync> HermesEvent<T> {
    /// Creates a new HermesEvent with the specified payload and synchronization option.
    ///
    /// # Arguments
    ///
    /// * `payload` - The payload to be carried by the event.
    /// * `wait` - A boolean flag indicating whether synchronization is enabled.
    ///
    /// # Returns
    ///
    /// A new HermesEvent instance with the provided payload and synchronization settings.
    pub fn new(payload: T, wait: bool) -> Self {
        let cv: Option<Arc<(Mutex<bool>, Condvar)>> = if wait {
            Some(Arc::new((Mutex::new(false), Condvar::new())))
        } else {
            None
        };

        Self { cv, payload }
    }

    /// Returns a reference to the payload carried by the event.
    pub fn payload(&self) -> &T {
        &self.payload
    }

    /// Notifies any waiting threads that the event condition has been met.
    ///
    /// # Returns
    ///
    /// An `anyhow::Result` indicating the success or failure of the notification.
    /// This can only error IFF another thread holding the mutex panics.
    /// This is a very very unlikely scenario given the use case.
    ///
    /// IF this fails, then it should just be logged, and execution continue as if there
    /// was no error.
    pub fn notify(&self) -> anyhow::Result<()> {
        if let Some(cv) = &mut self.cv {
            let (lock, cv) = &**cv;
            let mut done = lock.lock()?;
            *done = true;
            cv.notify_all();
        }

        Ok(())
    }

    /// Waits for the event condition to be met, potentially blocking the current thread.
    ///
    /// # Returns
    ///
    /// An `anyhow::Result` indicating the success or failure of the wait operation.
    /// This can only error IFF another thread holding the mutex panics.
    /// This is a very very unlikely scenario given the use case.
    ///
    /// IF this fails, then it should just be logged, and execution continue as if there
    /// was no error.
    pub fn wait(&self) -> anyhow::Result<()> {
        if let Some(cv) = &self.cv {
            let (lock, cv) = &**cv;
            let mut done = lock.lock()?;
            while !*done {
                done = cv.wait(done)?;
            }
        }

        Ok(())
    }
}
