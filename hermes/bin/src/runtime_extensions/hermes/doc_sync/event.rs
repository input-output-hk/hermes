//! Doc Sync events module.

use crate::{
    app::ApplicationName,
    event::{HermesEvent, HermesEventPayload},
    runtime_extensions::bindings::{
        hermes::doc_sync::api::{ChannelName, DocData},
        unchecked_exports,
    },
    wasm::module::ModuleId,
};

unchecked_exports::define! {
    /// Extends [`wasmtime::component::Instance`] with guest functions for Doc Sync.
    trait ComponentInstanceExt {
         #[wit("hermes:doc-sync/event-on-new-doc", "on-new-doc")]
        fn hermes_doc_sync_event_on_new_doc(channel: &str, doc: &[u8]);
    }
}

unchecked_exports::define! {
    /// Extends [`wasmtime::component::Instance`] with guest functions for Doc Sync.
    pub(crate) trait ReadComponentInstanceExt {
         #[wit("hermes:doc-sync/event-document-provider", "return-cids")]
        fn hermes_event_document_provider_return_cids(channel: &str) -> Vec<Vec<u8>>;

         #[wit("hermes:doc-sync/event-document-provider", "retrieve-doc")]
        fn hermes_event_document_provider_retrieve_doc(channel: &str, cid: &[u8]) -> Option<Vec<u8>>;

         #[wit("hermes:doc-sync/event-document-provider", "return-channels")]
        fn hermes_event_document_provider_return_channels() -> Vec<String>;
    }
}

/// Event payload for the `on-http-response` event.
#[derive(Clone)]
pub(crate) struct OnNewDocEvent {
    /// Channel name associated.
    pub(super) channel: ChannelName,
    /// Bytes representing the document.
    pub(super) doc: DocData,
}

impl OnNewDocEvent {
    /// Create the `OnNewDocEvent` from IPFS channel and document data.
    pub fn new(
        channel: &str,
        doc: &[u8],
    ) -> Self {
        Self {
            channel: channel.to_owned(),
            doc: doc.to_vec(),
        }
    }

    /// Build and send on-new-doc event.
    pub fn build_and_send(
        &self,
        app_names: Vec<ApplicationName>,
        module_ids: Option<&Vec<ModuleId>>,
    ) -> anyhow::Result<()> {
        let target_module = match module_ids {
            Some(module_ids) => crate::event::TargetModule::List(module_ids.clone()),
            None => crate::event::TargetModule::All,
        };

        let event = HermesEvent::new(
            self.clone(),
            crate::event::TargetApp::List(app_names),
            target_module,
        );
        crate::event::queue::send(event).map_err(|err| {
            tracing::error!(
                error = %err,
                channel = %self.channel,
                "Failed to send on-new-doc event"
            );
            err
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
