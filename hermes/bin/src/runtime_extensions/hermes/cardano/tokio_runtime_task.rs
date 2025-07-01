//! The Tokio Runtime task is responsible for executing a Tokio runtime inside
//! a OS thread so that Futures can be executed by the Cardano Runtime Extension.

use cardano_blockchain_types::{Network, Point};
use cardano_chain_follower::{turbo_downloader::DlConfig, ChainFollower, ChainSyncConfig};
use tracing::{error, info};

use crate::{app::ApplicationName, wasm::module::ModuleId};

/// Sync configuration.
struct SyncConfig {
    /// Cardano blockchain network.
    chain: Network,
    /// The maximum number of sync tasks.
    pub(crate) sync_tasks: u16,
    /// The maximum number of slots a sync task will process at once.
    pub(crate) sync_chunk_max_slots: u64,
    /// The Mithril Downloader Configuration.
    pub(crate) dl_config: DlConfig,
}

/// Start syncing a particular network
async fn start_sync_for(cfg: SyncConfig) -> anyhow::Result<()> {
    let chain = cfg.chain;
    let dl_config = cfg.dl_config.clone();

    let mut cfg = ChainSyncConfig::default_for(chain);
    cfg.mithril_cfg = cfg.mithril_cfg.with_dl_config(dl_config);

    info!(chain=%chain, "Starting Chain Sync Task");
    if let Err(error) = cfg.run().await {
        error!(chain=%chain, error=%error, "Failed to start Chain Sync Task");
        Err(error)?;
    }
    Ok(())
}

/// Command data that can be send to the Tokio runtime background thread.
enum Command {
    /// Instructs the Tokio runtime background thread to spawn a new chain follower.
    SpawnFollower {
        /// Name of the app that the follower will be tied to.
        app_name: ApplicationName,
        /// ID of the module that the follower will be tied to.
        module_id: ModuleId,
        /// Cardano blockchain that the follower will connect to.
        network: Network,
        /// Follower's starting point.
        follow_from: Point,
        /// Follower's ending point.
        follow_to: Point,
        /// Response channel sender.
        response_tx:
            tokio::sync::oneshot::Sender<anyhow::Result<super::chain_follower_task::Handle>>,
    },
}

/// Chain follower handle command channel sender.
type CommandSender = tokio::sync::mpsc::Sender<Command>;

/// Tokio runtime handle command channel receiver type.
type CommandReceiver = tokio::sync::mpsc::Receiver<Command>;

/// Handle used to communicate with a chain follower executor task.
pub struct Handle {
    /// Commands channel sender.
    cmd_tx: CommandSender,
}

impl Handle {
    // FIXME
    // /// Spawns a new chain follower in the background Tokio runtime.
    // ///
    // /// # Errors
    // ///
    // /// Returns Err if the chain follower executor task could not be spawned.
    // pub fn spawn_follower_sync(
    //     &self, app_name: ApplicationName, module_id: ModuleId, chain_id: CardanoBlockchainId,
    //     follow_from: cardano_chain_follower::PointOrTip,
    // ) -> Result<(
    //     super::chain_follower_task::Handle,
    //     cardano_chain_follower::Point,
    // )> {
    //     let (response_tx, response_rx) = tokio::sync::oneshot::channel();
    //     let cmd = Command::SpawnFollower {
    //         app_name,
    //         module_id,
    //         chain_id,
    //         follow_from,
    //         response_tx,
    //     };

    //     self.cmd_tx.blocking_send(cmd)?;

    //     response_rx.blocking_recv()?
    // }
}

pub(crate) async fn start_follower(cfg: SyncConfig) -> anyhow::Result<Handle> {
    // Syncing process need to be done prior to starting the follower
    start_sync_for(cfg).await?;
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel(1);
    // Spawning a new thread that will run the follower in background
    std::thread::spawn(move || {
        executor(cmd_rx);
    });
    Ok(Handle { cmd_tx })
}

/// Runs the Cardano Runtime Extension Tokio runtime.
fn executor(mut cmd_rx: CommandReceiver) {
    let res = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build();

    let runtime = match res {
        Ok(rt) => rt,
        Err(err) => {
            error!(error=?err, "Failed to start Cardano Runtime Extension background thread");
            return;
        },
    };

    runtime.block_on(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                Command::SpawnFollower {
                    app_name,
                    module_id,
                    network,
                    follow_from,
                    follow_to,
                    response_tx,
                } => {
                    info!(app=%app_name, module=%module_id, network=%network, follow_from=%follow_from, follow_to=%follow_to, "Spawning chain follower");
                    let res =
                        spawn_follower(app_name, module_id, network, follow_from, follow_to).await;
                    drop(response_tx.send(res));
                },
            }
        }
    })
}

async fn spawn_follower(
    app_name: ApplicationName, module_id: ModuleId, network: Network, follow_from: Point,
    follow_to: Point,
) -> anyhow::Result<super::chain_follower_task::Handle> {
    // Create a follower from the given parameters
    let mut follower = ChainFollower::new(network, follow_from, follow_to).await;
    let handle = super::chain_follower_task::spawn(follower, app_name, module_id, network);
    Ok(handle)
}
