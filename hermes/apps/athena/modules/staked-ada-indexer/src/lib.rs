//! Staked ADA Indexing Module

shared::bindings_generate!({
    world: "hermes:app/hermes",
    path: "../../../../../wasm/wasi/wit",
    inline: "
        package hermes:app;

        world hermes {
            include wasi:cli/imports@0.2.6;
            import hermes:cardano/api;
            import hermes:logging/api;
            import hermes:init/api;
            import hermes:sqlite/api;
            
            export hermes:init/event;
            export hermes:cardano/event-on-block;
            export hermes:cardano/event-on-immutable-roll-forward;
        }
    ",
    share: ["hermes:cardano", "hermes:sqlite", "hermes:logging"],
});

export!(Component);

mod events;
mod index;

use shared::{
    bindings::hermes::cardano::api::{Block, SubscriptionId},
    utils::log::{self, error},
};

mod config {
    use shared::bindings::hermes::cardano::api::SyncSlot;

    /// Slot to subscribe from during initialization.
    pub const SUBSCRIBE_FROM: SyncSlot = match option_env!("SUBSCRIBE_FROM") {
        None => SyncSlot::Genesis,
        Some(s) if matches!(s.as_bytes(), b"GENESIS") => SyncSlot::Genesis,
        Some(s) if matches!(s.as_bytes(), b"TIP") => SyncSlot::Tip,
        Some(s) if matches!(s.as_bytes(), b"IMMUTABLE_TIP") => SyncSlot::ImmutableTip,
        Some(s) => match u64::from_str_radix(s, 10) {
            Ok(i) => SyncSlot::Specific(i),
            Err(_) => panic!("non integer specific sync slot"),
        },
    };
}

struct Component;

impl exports::hermes::init::event::Guest for Component {
    fn init() -> bool {
        log::init(log::LevelFilter::Trace);
        match events::init(config::SUBSCRIBE_FROM) {
            Ok(()) => true,
            Err(error) => {
                error!(target: "staked_ada_indexer::init", error:?; "Not handled");
                false
            },
        }
    }
}

impl exports::hermes::cardano::event_on_block::Guest for Component {
    fn on_cardano_block(
        subscription_id: &SubscriptionId,
        block: &Block,
    ) {
        log::init(log::LevelFilter::Trace);
        if let Err(error) = events::on_cardano_block(subscription_id, block) {
            error!(target: "staked_ada_indexer::on_cardano_block", error:?; "Not handled");
        }
    }
}

impl exports::hermes::cardano::event_on_immutable_roll_forward::Guest for Component {
    fn on_cardano_immutable_roll_forward(
        subscription_id: &SubscriptionId,
        block: &Block,
    ) {
        log::init(log::LevelFilter::Trace);
        if let Err(error) = events::on_cardano_immutable_roll_forward(subscription_id, block) {
            error!(target: "staked_ada_indexer::on_cardano_immutable_roll_forward", error:?; "Not handled");
        }
    }
}
