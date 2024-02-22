//! Cardano Blockchain runtime extension event handler implementation.

use crate::{
    event_queue::event::HermesEventPayload,
    runtime_extensions::bindings::hermes::cardano::api::{
        BlockSrc, CardanoBlock, CardanoBlockchainId, CardanoTxn,
    },
};

/// On Cardano block event
struct OnCardanoBlockEvent {
    /// The blockchain id the block originated from.
    blockchain: CardanoBlockchainId,
    /// This raw CBOR block data.
    block: CardanoBlock,
    /// Source information about where the block came from, and if we are at tip or not.
    source: BlockSrc,
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
struct OnCardanoTxnEvent {
    /// The blockchain id the block originated from.
    blockchain: CardanoBlockchainId,
    /// The slot the transaction is in.
    slot: u64,
    /// The offset in the block this transaction is at.
    txn_index: u32,
    /// The raw transaction data itself.
    txn: CardanoTxn,
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
struct OnCardanoRollback {
    /// The blockchain id the block originated from.
    blockchain: CardanoBlockchainId,
    /// The slot the transaction is in.
    slot: u64,
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
