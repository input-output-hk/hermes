//! Chain sync task

use cardano_chain_follower::{ChainSyncConfig, turbo_downloader::DlConfig};

use crate::runtime_extensions::hermes::cardano::{STATE, TOKIO_RUNTIME};

/// Spawns a chain sync task for a given network
pub(crate) fn spawn_chain_sync_task(chain: cardano_blockchain_types::Network) {
    tracing::info!(chain = %chain, "Spawning chain sync task");

    // Check if chain sync already exists
    if STATE.sync_state.contains_key(&chain) {
        return;
    }

    let dl_config = DlConfig::default();
    let mut sync_cfg = ChainSyncConfig::default_for(chain.clone());
    sync_cfg.mithril_cfg = sync_cfg.mithril_cfg.with_dl_config(dl_config);

    let handle = TOKIO_RUNTIME.handle();
    let join_handle = handle.spawn(async move {
        // Make the task cancellable - note that ctrl_c can only be awaited once globally,
        // so we use a spawned task that can be aborted instead
        if let Err(error) = sync_cfg.clone().run().await {
            tracing::error!(chain = %sync_cfg.chain, error = %error, "Chain sync failed");
        }
    });

    STATE.sync_state.insert(chain, join_handle);
}
