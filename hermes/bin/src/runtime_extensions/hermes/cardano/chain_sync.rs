//! Chain sync task

use cardano_chain_follower::{turbo_downloader::DlConfig, ChainSyncConfig};

use crate::runtime_extensions::hermes::cardano::{STATE, TOKIO_RUNTIME};

pub(crate) fn spawn_chain_sync_task(chain: cardano_blockchain_types::Network) {
    tracing::info!(chain = %chain, "Spawning chain sync task");
    let dl_config = DlConfig::default();
    let mut sync_cfg = ChainSyncConfig::default_for(chain);
    sync_cfg.mithril_cfg = sync_cfg.mithril_cfg.with_dl_config(dl_config);

    if !STATE.sync_state.contains_key(&chain) {
        let handle = TOKIO_RUNTIME.handle();
        let join_handle = handle.spawn(async move {
            if let Err(error) = sync_cfg.run().await {
                tracing::error!(chain = %chain, error = %error, "Chain sync failed");
            }
        });
        STATE.sync_state.insert(chain, join_handle);
    }
}
