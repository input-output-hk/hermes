//! Hermes IPFS runtime extension event handler implementation.
use crate::{
    event::HermesEventPayload,
    runtime_extensions::bindings::hermes::ipfs::api::{PubsubMessage, PubsubTopic},
};

/// Event handler for the `on-topic` event.
#[allow(dead_code)]
pub(crate) struct OnTopicEvent {
    ///  Topic
    pub(crate) topic: PubsubTopic,
    ///  Message
    pub(crate) message: PubsubMessage,
}

impl HermesEventPayload for OnTopicEvent {
    fn event_name(&self) -> &str {
        "on-topic"
    }

    fn execute(&self, module: &mut crate::wasm::module::ModuleInstance) -> anyhow::Result<()> {
        let _res: bool = module
            .instance
            .hermes_ipfs_event()
            .call_on_topic(&mut module.store, &self.message)?;
        // TODO(@saibatizoku):  WIP: add message handling
        Ok(())
    }
}
