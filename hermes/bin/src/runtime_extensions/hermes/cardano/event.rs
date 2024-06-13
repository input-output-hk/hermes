//! Cardano Blockchain runtime extension event handler implementation.

use crate::{
    event::HermesEventPayload,
    runtime_extensions::bindings::hermes::cardano::api::{
        BlockDetail, CardanoBlock, CardanoBlockchainId, CardanoTxn,
    },
};

/// On Cardano block event
pub(super) struct OnCardanoBlockEvent {
    /// The blockchain id the block originated from.
    pub(super) blockchain: CardanoBlockchainId,
    /// This raw CBOR block data.
    pub(super) block: CardanoBlock,
    /// Source information about where the block came from, and if we are at tip or not.
    pub(super) details: BlockDetail,
}

impl HermesEventPayload for OnCardanoBlockEvent {
    fn event_name(&self) -> &str {
        "on-cardano-block"
    }

    fn execute(&self, module: &mut crate::wasm::module::ModuleInstance) -> anyhow::Result<()> {
        module
            .instance
            .hermes_cardano_event_on_block()
            .call_on_cardano_block(
                &mut module.store,
                self.blockchain,
                &self.block,
                &self.details,
            )?;
        Ok(())
    }
}

/// On Cardano txn event
pub(super) struct OnCardanoTxnEvent {
    /// The blockchain id the block originated from.
    pub(super) blockchain: CardanoBlockchainId,
    /// The transaction index with the block the transaction is in.
    pub(super) txn_index: u32,
    /// The raw transaction data itself.
    pub(super) txn: CardanoTxn,
    /// Details about the block the transaction is in.
    pub(super) details: BlockDetail,
}

impl HermesEventPayload for OnCardanoTxnEvent {
    fn event_name(&self) -> &str {
        "on-cardano-txn"
    }

    fn execute(&self, module: &mut crate::wasm::module::ModuleInstance) -> anyhow::Result<()> {
        module
            .instance
            .hermes_cardano_event_on_txn()
            .call_on_cardano_txn(
                &mut module.store,
                self.blockchain,
                self.txn_index,
                &self.txn,
                &self.details,
            )?;

        Ok(())
    }
}
