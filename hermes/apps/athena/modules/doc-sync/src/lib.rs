#![allow(missing_docs)]
//! Doc Sync Module

shared::bindings_generate!({
    world: "hermes:app/hermes",
    path: "../../../../../wasm/wasi/wit",
    inline: "
        package hermes:app;

        world hermes {
            include wasi:cli/imports@0.2.6;
            import hermes:doc-sync/api;
            import hermes:logging/api;

            export hermes:init/event;
            export hermes:doc-sync/event;
        }
    ",
    share: ["hermes:logging", "hermes:doc-sync"],
});

export!(Component);

use shared::{
    bindings::hermes::doc_sync::api::{ChannelName, DocData, SyncChannel},
    utils::log::{self, info, warn},
};

/// Doc Sync component.
struct Component;

impl exports::hermes::init::event::Guest for Component {
    fn init() -> bool {
        log::init(log::LevelFilter::Trace);
        info!(target: "doc_sync::init", "💫 Opening channel...");
        let _chan = SyncChannel::new("documents");
        info!(target: "doc_sync::init", "💫 Channel opened");
        true
    }
}

impl exports::hermes::doc_sync::event::Guest for Component {
    fn on_new_doc(
        channel: ChannelName,
        doc: DocData,
    ) {
        log::init(log::LevelFilter::Trace);
        warn!(target: "doc_sync::on_new_doc", channel:%, doc_byte_length = doc.len(); "Unimplemented!");
    }
}
