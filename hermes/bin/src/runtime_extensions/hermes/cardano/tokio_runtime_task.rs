//! The Tokio Runtime task is responsible for executing a Tokio runtime inside
//! a OS thread so that Futures can be executed by the Cardano Runtime Extension.

use tracing::{error, instrument, trace};

use super::{Result, STATE};
use crate::{
    app::ApplicationName, runtime_extensions::bindings::hermes::cardano::api::CardanoBlockchainId,
    wasm::module::ModuleId,
};

/// Command data that can be send to the Tokio runtime background thread.
enum Command {
    /// Instructs the Tokio runtime background thread to spawn a new chain follower.
    SpawnFollower {
        /// Name of the app that the follower will be tied to.
        app_name: ApplicationName,
        /// ID of the module that the follower will be tied to.
        module_id: ModuleId,
        /// Cardano blockchain that the follower will connect to.
        chain_id: CardanoBlockchainId,
        /// Follower's starting point.
        follow_from: cardano_chain_follower::PointOrTip,
        /// Response channel sender.
        response_tx: tokio::sync::oneshot::Sender<
            Result<(
                super::chain_follower_task::Handle,
                cardano_chain_follower::Point,
            )>,
        >,
    },
    /// Instructs the Tokio runtime background thread to read a block using some follower.
    ReadBlock {
        /// Cardano blockchain from which the block will be fetched.
        chain_id: CardanoBlockchainId,
        /// Chain point at which the block is to be fetched.
        at: cardano_chain_follower::PointOrTip,
        /// Response channel sender.
        response_tx:
            tokio::sync::oneshot::Sender<Result<cardano_chain_follower::MultiEraBlockData>>,
    },
}

/// Tokio runtime handle command channel sender type.
type CommandSender = tokio::sync::mpsc::Sender<Command>;

/// Tokio runtime handle command channel receiver type.
type CommandReceiver = tokio::sync::mpsc::Receiver<Command>;

/// Handle used for communicating with the Tokio runtime background thread.
pub struct Handle {
    /// Commands channel sender.
    cmd_tx: CommandSender,
}

impl Handle {
    /// Spawns a new chain follower in the background Tokio runtime.
    ///
    /// # Errors
    ///
    /// Returns Err if the chain follower executor task could not be spawned.
    pub fn spawn_follower_sync(
        &self, app_name: ApplicationName, module_id: ModuleId, chain_id: CardanoBlockchainId,
        follow_from: cardano_chain_follower::PointOrTip,
    ) -> Result<(
        super::chain_follower_task::Handle,
        cardano_chain_follower::Point,
    )> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        let cmd = Command::SpawnFollower {
            app_name,
            module_id,
            chain_id,
            follow_from,
            response_tx,
        };

        self.cmd_tx.blocking_send(cmd)?;

        response_rx.blocking_recv()?
    }

    /// Reads a block from a Cardano network.
    ///
    /// # Errors
    ///
    /// Return Err if there were any errors while fetching the block.
    pub fn read_block(
        &self, chain_id: CardanoBlockchainId, at: cardano_chain_follower::PointOrTip,
    ) -> Result<cardano_chain_follower::MultiEraBlockData> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        let cmd = Command::ReadBlock {
            chain_id,
            at,
            response_tx,
        };

        self.cmd_tx.blocking_send(cmd)?;

        response_rx.blocking_recv()?
    }
}

/// Spawns a OS thread running the Tokio runtime task.
pub fn spawn() -> Handle {
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel(1);
    std::thread::spawn(move || {
        executor(cmd_rx);
    });

    Handle { cmd_tx }
}

/// Runs the Cardano Runtime Extension Tokio runtime.
#[instrument(skip(cmd_rx))]
fn executor(mut cmd_rx: CommandReceiver) {
    let res = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build();

    let rt = match res {
        Ok(rt) => rt,
        Err(err) => {
            error!(error = ?err, "Failed to start Cardano Runtime Extension background thread");
            return;
        },
    };

    trace!("Created Tokio runtime");

    rt.block_on(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                Command::SpawnFollower {
                    app_name,
                    module_id,
                    chain_id,
                    follow_from,
                    response_tx,
                } => {
                    let res = spawn_follower(app_name, module_id, chain_id, follow_from).await;
                    drop(response_tx.send(res));
                },
                Command::ReadBlock {
                    chain_id,
                    at,
                    response_tx,
                } => {
                    let res = read_block(chain_id, at).await;
                    drop(response_tx.send(res));
                },
            }
        }
    });
}

/// Spawns a follower which will follow the given chain and sets its starting point to the
/// given point.
async fn spawn_follower(
    app_name: ApplicationName, module_id: ModuleId, chain_id: CardanoBlockchainId,
    follow_from: cardano_chain_follower::PointOrTip,
) -> Result<(
    super::chain_follower_task::Handle,
    cardano_chain_follower::Point,
)> {
    trace!("Spawning chain follower executor");

    let config = cardano_chain_follower::FollowerConfigBuilder::default().build();
    let network = chain_id.into();

    let follower = cardano_chain_follower::Follower::connect(
        follower_connect_address(network),
        network,
        config,
    )
    .await?;

    trace!("Started chain follower");

    let point = follower
        .set_read_pointer(follow_from)
        .await?
        .ok_or(anyhow::anyhow!("Failed to locate follower starting point"))?;

    trace!("Set chain follower starting point");

    let handle = super::chain_follower_task::spawn(follower, app_name, module_id, chain_id);

    Ok((handle, point))
}

/// Reads a block from the given chain at the given point.
async fn read_block(
    chain_id: CardanoBlockchainId, at: cardano_chain_follower::PointOrTip,
) -> Result<cardano_chain_follower::MultiEraBlockData> {
    trace!("Reading block");

    let network = chain_id.into();

    if let Some(reader) = STATE.readers.get(&network) {
        let block_data = reader.read_block(at).await?;

        Ok(block_data)
    } else {
        // Limit the follower's buffer size. This does not really matter
        // since we'll not poll the
        // follower's future so the following process will
        // not be executed.
        let cfg = cardano_chain_follower::FollowerConfigBuilder::default()
            .chain_update_buffer_size(1)
            .build();

        let reader = cardano_chain_follower::Follower::connect(
            follower_connect_address(network),
            network,
            cfg,
        )
        .await?;

        let block_data = reader.read_block(at).await?;

        Ok(block_data)
    }
}

/// Returns the peer address used to connect to each Cardano network.
const fn follower_connect_address(network: cardano_chain_follower::Network) -> &'static str {
    match network {
        cardano_chain_follower::Network::Mainnet => "backbone.cardano-mainnet.iohk.io:3001",
        cardano_chain_follower::Network::Preprod => "preprod-node.play.dev.cardano.org:3001",
        cardano_chain_follower::Network::Preview => "preview-node.play.dev.cardano.org:3001",
        cardano_chain_follower::Network::Testnet => todo!(),
    }
}
