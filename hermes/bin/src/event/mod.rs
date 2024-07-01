//! Hermes event's primitives.

pub(crate) mod queue;

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use crossbeam_channel::{unbounded, Receiver, Sender};

use crate::{
    app::HermesAppName,
    wasm::module::{ModuleId, ModuleInstance},
};

/// A trait for defining the behavior of a Hermes event.
pub(crate) trait HermesEventPayload: Send + Sync + 'static {
    /// Returns the name of the event associated with the payload.
    fn event_name(&self) -> &str;

    /// Executes the behavior associated with the payload, using the provided executor.
    ///
    /// # Arguments
    ///
    /// * `executor` - The executor to use for executing the payload's behavior.
    ///
    /// # Returns
    ///
    /// An `anyhow::Result` indicating the success or failure of the payload execution.
    fn execute(&self, module: &mut ModuleInstance) -> anyhow::Result<()>;
}

/// Target Hermes app to execute the event
#[derive(Clone)]
pub(crate) enum TargetApp {
    /// Execute for all available apps
    #[allow(dead_code)]
    All,
    /// Execute for a specific list of apps
    List(Vec<HermesAppName>),
}

/// Target WASM module to execute the event
pub(crate) enum TargetModule {
    /// Execute for all available modules
    All,
    /// Execute for a specific list of modules
    List(Vec<ModuleId>),
}

/// Hermes event
pub(crate) struct HermesEvent {
    /// The payload carried by the `HermesEvent`.
    payload: Box<dyn HermesEventPayload>,

    /// Target app
    target_app: TargetApp,

    /// Target module
    target_module: TargetModule,

    /// Signalling queue which tracks event deps
    completion_queue: Option<Sender<Box<dyn HermesEventPayload>>>,

    /// Event lifecycle tracker
    event_lifetimes: Arc<AtomicU64>,
}

impl HermesEvent {
    /// Create a new Hermes event
    pub(crate) fn new(
        payload: impl HermesEventPayload, target_app: TargetApp, target_module: TargetModule,
    ) -> Self {
        Self {
            payload: Box::new(payload),
            target_app,
            target_module,
            completion_queue: None,
            event_lifetimes: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Create event completion queue
    pub(crate) fn make_waiter(&mut self) -> Receiver<Box<dyn HermesEventPayload>> {
        let (tx, rx) = unbounded();
        self.completion_queue = Some(tx);
        rx
    }

    /// Event invocation on module
    pub(crate) fn add_processor(&self) {
        self.event_lifetimes.fetch_add(1, Ordering::SeqCst);
    }

    /// Event invocation on module complete
    pub(crate) fn subtract_processor(&self) {
        self.event_lifetimes.fetch_sub(1, Ordering::SeqCst);
    }

    /// Event lifecycle finished. Return self back to event completion queue.
    pub(crate) fn finished(self) {
        // Subtracts from the current value, returning the previous value.
        if self.event_lifetimes.load(Ordering::SeqCst) == 0 {
            if let Some(q) = self.completion_queue {
                match q.try_send(self.payload) {
                    Ok(_) => (),
                    Err(_) => (),
                };
            }
        };
    }

    /// Get event's payload
    pub(crate) fn payload(&self) -> &dyn HermesEventPayload {
        self.payload.as_ref()
    }

    /// Get event's target app
    pub(crate) fn target_app(&self) -> &TargetApp {
        &self.target_app
    }

    /// Get event's target module
    pub(crate) fn target_module(&self) -> &TargetModule {
        &self.target_module
    }
}
