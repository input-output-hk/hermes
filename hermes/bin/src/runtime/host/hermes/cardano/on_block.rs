//! Host implementation of the on-block event
use wasmtime::Store;

use crate::runtime::extensions::{
    bindings::{
        exports::hermes::cardano::event_on_block::BlockSrc,
        hermes::cardano::api::CardanoBlockchainId, Hermes,
    },
    event::HermesEventPayload,
    HermesState,
};

pub(crate) struct CardanoOnBlockEventPayload {
    // These are not correct, just used for example purposes.
    // re-define to correct values required.
    pub mainnet: bool,
    pub block_data: Vec<u8>,
    pub at_tip: bool,
    pub mithril: bool,
}

impl HermesEventPayload for CardanoOnBlockEventPayload {
    fn event_name(&self) -> &str {
        "on-block"
    }

    fn execute(&self, bindings: &Hermes, store: &mut Store<HermesState>) -> anyhow::Result<()> {
        // Example of calling on_block.

        // Get all the parameters right to call the wasm event
        let arg0 = if self.mainnet {
            CardanoBlockchainId::Mainnet
        } else {
            CardanoBlockchainId::Preprod
        };
        let arg1 = &self.block_data;
        let mut arg2 = if self.mithril {
            BlockSrc::MITHRIL
        } else {
            BlockSrc::NODE
        };
        if self.at_tip {
            arg2 |= BlockSrc::TIP;
        }

        let result = bindings
            .hermes_cardano_event_on_block()
            .call_on_cardano_block(store, arg0, arg1, arg2);

        // Handle the response here if required...

        result
    }
}
