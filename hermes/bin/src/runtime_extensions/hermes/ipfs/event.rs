//! Hermes IPFS runtime extension event handler implementation.
use std::fmt::Display;

use crate::{
    app::ApplicationName,
    event::{HermesEvent, HermesEventPayload},
    runtime_extensions::bindings::{hermes::ipfs::api::PubsubMessage, unchecked_exports},
    wasm::module::ModuleId,
};

unchecked_exports::define! {
    /// Extends [`wasmtime::component::Instance`] with guest functions for ipfs.
    trait ComponentInstanceExt {
       #[wit("hermes:ipfs/event", "on-topic")]
        fn hermes_ipfs_event_on_topic(message: &PubsubMessage) -> bool;
    }
}

/// Event handler for the `on-topic` event.
#[derive(Clone)]
pub(crate) struct OnTopicEvent {
    ///  Topic message received.
    pub(crate) message: PubsubMessage,
}

impl OnTopicEvent {
    /// Create a new `OnTopicEvent` event from a message.
    pub(crate) fn new(message: PubsubMessage) -> Self {
        Self { message }
    }

    /// Build and send on-topic event.
    pub(crate) fn build_and_send(
        &self,
        app_names: Vec<ApplicationName>,
        module_ids: Vec<ModuleId>,
    ) -> anyhow::Result<()> {
        let event = HermesEvent::new(
            self.clone(),
            crate::event::TargetApp::List(app_names),
            crate::event::TargetModule::List(module_ids),
        );

        crate::event::queue::send(event).map_err(|err| {
            tracing::error!(
                error = %err,
                channel = %self.message.topic,
                "Failed to send on-topic event"
            );
            err
        })
    }
}

impl Display for OnTopicEvent {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        #[allow(clippy::use_debug)]
        let msg = format!("{:?}", self.message);
        write!(f, "Message: {msg}")
    }
}

impl HermesEventPayload for OnTopicEvent {
    fn event_name(&self) -> &'static str {
        "on-topic"
    }

    fn execute(
        &self,
        module: &mut crate::wasm::module::ModuleInstance,
    ) -> anyhow::Result<()> {
        let _res = module
            .instance
            .hermes_ipfs_event_on_topic(&mut module.store, &self.message)?;
        // TODO(@saibatizoku):  WIP: add message handling
        Ok(())
    }
}
