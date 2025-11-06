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
            import hermes:init/api;

            export hermes:init/event;
            export hermes:doc-sync/event;
        }
    ",
    share: ["hermes:logging"],
});

export!(Component);

use shared::utils::log::{self, warn};

use crate::exports::hermes::doc_sync::event::{ChannelName, DocData};

/// Doc Sync component.
struct Component;

impl exports::hermes::init::event::Guest for Component {
    fn init() -> bool {
        log::init(log::LevelFilter::Trace);
        warn!(target: "doc_sync::init", "Unimplemented");
        true
    }
}

impl exports::hermes::doc_sync::event::Guest for Component {
    fn on_new_doc(
        channel: ChannelName,
        doc: DocData,
    ) {
        log::init(log::LevelFilter::Trace);
        warn!(target: "doc_sync::on_new_doc", channel:%, doc_byte_length = doc.len(); "Unimplemented");
    }
}
