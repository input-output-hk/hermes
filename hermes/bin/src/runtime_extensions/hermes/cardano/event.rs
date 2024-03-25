//! Cardano Blockchain runtime extension event handler implementation.

use crate::{
    event::HermesEventPayload,
    runtime_extensions::bindings::hermes::cardano::api::{
        BlockSrc, CardanoBlock, CardanoBlockchainId, CardanoTxn,
    },
};

/// On Cardano block event
pub(super) struct OnCardanoBlockEvent {
    /// The blockchain id the block originated from.
    pub(super) blockchain: CardanoBlockchainId,
    /// This raw CBOR block data.
    pub(super) block: CardanoBlock,
    /// Source information about where the block came from, and if we are at tip or not.
    pub(super) source: BlockSrc,
}

impl HermesEventPayload for OnCardanoBlockEvent {
    fn event_name(&self) -> &str {
        "on-cardano-block"
    }

    fn execute(&self, module: &mut crate::wasm::module::ModuleInstance) -> anyhow::Result<()> {
        module
            .instance
            .hermes_cardano_event_on_block()
            .call_on_cardano_block(&mut module.store, self.blockchain, &self.block, self.source)?;
        Ok(())
    }
}

/// On Cardano txn event
pub(super) struct OnCardanoTxnEvent {
    /// The blockchain id the block originated from.
    pub(super) blockchain: CardanoBlockchainId,
    /// The slot the transaction is in.
    pub(super) slot: u64,
    /// The offset in the block this transaction is at.
    pub(super) txn_index: u32,
    /// The raw transaction data itself.
    pub(super) txn: CardanoTxn,
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
                self.slot,
                self.txn_index,
                &self.txn,
            )?;

        Ok(())
    }
}

/// On Cardano rollback event
pub(super) struct OnCardanoRollback {
    /// The blockchain id the block originated from.
    pub(super) blockchain: CardanoBlockchainId,
    /// The slot the transaction is in.
    pub(super) slot: u64,
}

impl HermesEventPayload for OnCardanoRollback {
    fn event_name(&self) -> &str {
        "on-cardano-rollback"
    }

    fn execute(&self, module: &mut crate::wasm::module::ModuleInstance) -> anyhow::Result<()> {
        module
            .instance
            .hermes_cardano_event_on_rollback()
            .call_on_cardano_rollback(&mut module.store, self.blockchain, self.slot)?;
        Ok(())
    }
}
