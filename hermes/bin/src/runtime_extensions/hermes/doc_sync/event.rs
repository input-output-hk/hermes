//! Doc Sync events module.

use crate::{
    event::HermesEventPayload,
    runtime_extensions::bindings::{
        hermes::doc_sync::api::{ChannelName, DocData},
        unchecked_exports,
    },
};

unchecked_exports::define! {
    /// Extends [`wasmtime::component::Instance`] with guest functions for Doc Sync.
    trait ComponentInstanceExt {
         #[wit("hermes:doc-sync/event", "on-new-doc")]
        fn hermes_doc_sync_event_on_new_doc(channel: &str, doc: &[u8]);
    }
}

/// Event payload for the `on-http-response` event.
#[allow(dead_code, reason = "sending an event is unimplemented")]
pub(crate) struct OnNewDocEvent {
    /// Channel name associated.
    pub(super) channel: ChannelName,
    /// Bytes representing the document.
    pub(super) doc: DocData,
}

impl OnNewDocEvent {
    /// Create the event from IPFS topic and message.
    pub fn from_ipfs(
        topic: &str,
        message: &[u8],
    ) -> anyhow::Result<Self> {
        super::map_ipfs_topic_to_channel_name(topic).map(|channel| {
            Self {
                channel: channel.into_owned(),
                doc: message.to_vec(),
            }
        })
    }
}

impl HermesEventPayload for OnNewDocEvent {
    fn event_name(&self) -> &'static str {
        "on-new-doc"
    }

    fn execute(
        &self,
        module: &mut crate::wasm::module::ModuleInstance,
    ) -> anyhow::Result<()> {
        module.instance.hermes_doc_sync_event_on_new_doc(
            &mut module.store,
            &self.channel,
            &self.doc,
        )?;
        Ok(())
    }
}
