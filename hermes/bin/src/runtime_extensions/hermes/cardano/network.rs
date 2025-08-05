//! Cardano Blockchain network implementation for WASM runtime.

use std::sync::Arc;

use cardano_blockchain_types::{Network, Point};
use cardano_chain_follower::{ChainFollower, Kind};
use tracing::error;

use crate::{
    app::ApplicationName,
    runtime_extensions::{
        bindings::hermes::cardano::api::{CardanoNetwork, SubscriptionId, SyncSlot},
        hermes::cardano::{
            block::get_tips,
            event::{build_and_send_block_event, build_and_send_roll_forward_event},
            CardanoError, SubscriptionType, STATE,
        },
    },
    wasm::module::ModuleId,
};

/// Chain follower subscribe command
enum Command {
    /// Instructs the chain follower to stop.
    Stop(tokio::sync::oneshot::Sender<()>),
}

/// Chain follower handle command channel sender.
type CommandSender = tokio::sync::mpsc::Sender<Command>;
/// Chain follower handle command channel receiver.
type CommandReceiver = tokio::sync::mpsc::Receiver<Command>;

/// Handle used to communicate with a chain follower executor task.
pub struct Handle {
    /// Commands channel sender.
    cmd_tx: Arc<CommandSender>,
}

impl Handle {
    /// Sends a command to the chain follower executor task to stop following.
    pub fn stop(&self) -> anyhow::Result<()> {
        let (res_tx, res_rx) = tokio::sync::oneshot::channel();
        self.cmd_tx.blocking_send(Command::Stop(res_tx))?;
        drop(res_rx.blocking_recv());
        Ok(())
    }
}

/// Spawn a new thread that runs a Tokio runtime, which is used to handle
/// a subscription to a specific network.
pub(crate) fn spawn_subscribe(
    app: ApplicationName,
    module_id: ModuleId,
    start: Point,
    network: Network,
    subscription_type: SubscriptionType,
    subscription_id: wasmtime::component::Resource<SubscriptionId>,
) -> Handle {
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel(1);
    let arc_cmd_tx = Arc::new(cmd_tx);

    std::thread::spawn(move || {
        let Ok(rt) = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .enable_io()
            .build()
        else {
            error!(
                error = "Failed to create Tokio runtime",
                "Failed to spawn chain follower"
            );
            return;
        }; // // Hold onto the clone inside the thread to keep Arc alive

        rt.block_on(subscribe(
            cmd_rx,
            app,
            module_id,
            start,
            network,
            subscription_type,
            subscription_id,
        ));
    });

    Handle { cmd_tx: arc_cmd_tx }
}

/// Subscribe to events from a Cardano network.
/// This function will enter a loop and await either a command to stop the
/// subscription or a new block update from the chain follower.
async fn subscribe(
    mut cmd_rx: CommandReceiver,
    app: ApplicationName,
    module_id: ModuleId,
    start: Point,
    network: Network,
    subscription_type: SubscriptionType,
    subscription_id: wasmtime::component::Resource<SubscriptionId>,
) {
    let mut follower = ChainFollower::new(network, start, Point::TIP).await;

    loop {
        tokio::select! {
            res = cmd_rx.recv() => {
                match res {
                    // Received a stop command
                    Some(Command::Stop(res_tx)) => {
                        let _ = res_tx.send(());
                        break;
                    }
                    None => {
                        // Channel close
                        break;
                    }
                }
            }

            // Handle new block update and send the block event
            update = follower.next() => {
                match update {
                    Some(chain_update) => {
                        let Ok(block_app_state) = STATE.block.get_app_state(&app) else {
                            // This should not failed
                            error!(error="Failed to get block app state for app: {app}");
                            return
                        };
                        let block_resource = block_app_state.create_resource(chain_update.block_data().clone());
                        match chain_update.kind {
                            Kind::Block if subscription_type == SubscriptionType::Block => {
                                 if let Err(e) = build_and_send_block_event(
                                    app.clone(),
                                    module_id.clone(),
                                    subscription_id.rep(),
                                    block_resource.rep()
                                ) {
                                    error!(error=?e, "Failed to send block event");
                                    break;
                                }
                            }
                            Kind::Rollback if subscription_type == SubscriptionType::Block => {
                                if let Err(e) = build_and_send_block_event(
                                    app.clone(),
                                    module_id.clone(),
                                    subscription_id.rep(),
                                    block_resource.rep()
                                ) {
                                    error!(error=?e, "Failed to send rollback block event");
                                    break;
                                }
                            }
                            Kind::ImmutableBlockRollForward if subscription_type == SubscriptionType::ImmutableRollForward => {
                                if let Err(e) = build_and_send_roll_forward_event(
                                    app.clone(),
                                    module_id.clone(),
                                    subscription_id.rep(),
                                    block_resource.rep()
                                ){
                                    error!(error=?e, "Failed to send immutable block roll-forward event");
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    None => {
                        break;
                    }
                }
            }
        }
    }
}

impl TryFrom<CardanoNetwork> for cardano_blockchain_types::Network {
    type Error = CardanoError;

    fn try_from(network: CardanoNetwork) -> Result<Self, Self::Error> {
        match network {
            CardanoNetwork::Mainnet => Ok(cardano_blockchain_types::Network::Mainnet),
            CardanoNetwork::Preprod => Ok(cardano_blockchain_types::Network::Preprod),
            CardanoNetwork::Preview => Ok(cardano_blockchain_types::Network::Preview),
            CardanoNetwork::TestnetMagic(n) => Err(CardanoError::NetworkNotSupported(n)),
        }
    }
}

/// Convert `SyncSlot` to a point.
pub(crate) fn sync_slot_to_point(
    slot: SyncSlot,
    network: Network,
) -> anyhow::Result<Point> {
    let (immutable_tip, live_tip) = get_tips(network)?;
    let immutable_tip = Point::fuzzy(immutable_tip);
    let live_tip = Point::fuzzy(live_tip);
    match slot {
        SyncSlot::Genesis => Ok(Point::ORIGIN),
        SyncSlot::Tip => Ok(live_tip),
        SyncSlot::ImmutableTip => Ok(immutable_tip),
        SyncSlot::Specific(slot) => Ok(Point::fuzzy(slot.into())),
    }
}

impl From<cardano_blockchain_types::Network> for CardanoNetwork {
    fn from(network: cardano_blockchain_types::Network) -> Self {
        match network {
            cardano_blockchain_types::Network::Mainnet => CardanoNetwork::Mainnet,
            cardano_blockchain_types::Network::Preprod => CardanoNetwork::Preprod,
            cardano_blockchain_types::Network::Preview => CardanoNetwork::Preview,
        }
    }
}
