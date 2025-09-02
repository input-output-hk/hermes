//! Hermes IPFS runtime extension event handler implementation.
use std::fmt::Display;

use crate::{
    event::HermesEventPayload,
    runtime_extensions::bindings::{
        hermes::ipfs::api::PubsubMessage, unchecked_exports::ComponentInstanceExt as _,
    },
};

/// Event handler for the `on-topic` event.
#[derive(Clone)]
pub(crate) struct OnTopicEvent {
    ///  Topic message received.
    pub(crate) message: PubsubMessage,
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
