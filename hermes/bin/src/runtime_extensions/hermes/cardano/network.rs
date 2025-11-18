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
            CardanoError, STATE, SubscriptionType, TOKIO_RUNTIME,
            block::get_tips,
            event::{build_and_send_block_event, build_and_send_roll_forward_event},
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
    /// Uses non-blocking send and doesn't wait for confirmation to prevent shutdown
    /// hangs.
    pub fn stop(&self) -> anyhow::Result<()> {
        // Use try_send to avoid blocking if the channel is full
        // Don't wait for response - just send the signal and move on
        match self
            .cmd_tx
            .try_send(Command::Stop(tokio::sync::oneshot::channel().0))
        {
            Ok(_) => {
                tracing::debug!("Stop command sent to chain follower");
                Ok(())
            },
            Err(e) => {
                tracing::warn!(
                    "Failed to send stop command (task may already be stopped): {}",
                    e
                );
                // Not a fatal error - task might already be stopped
                Ok(())
            },
        }
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
    let handle = TOKIO_RUNTIME.handle();
    handle.spawn(subscribe(
        cmd_rx,
        app,
        module_id,
        start,
        network,
        subscription_type,
        subscription_id,
    ));

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
    let mut follower = ChainFollower::new(&network, start, Point::TIP).await;

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
                        // Clone block data BEFORE acquiring any locks to avoid holding locks during expensive clone
                        let block_data = chain_update.block_data().clone();

                        let Ok(block_app_state) = STATE.block.get_app_state_readonly(&app) else {
                            // This should not failed
                            error!(error="Failed to get block app state for app: {app}");
                            return
                        };
                        let block_resource = block_app_state.create_resource(block_data);
                        // Drop the app state reference immediately to release the lock
                        drop(block_app_state);

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
    match slot {
        SyncSlot::Genesis => Ok(Point::ORIGIN),
        SyncSlot::Tip => {
            let (_, live_tip) = get_tips(network)?;
            Ok(Point::fuzzy(live_tip))
        },
        SyncSlot::ImmutableTip => {
            let (immutable_tip, _) = get_tips(network)?;
            Ok(Point::fuzzy(immutable_tip))
        },
        SyncSlot::Specific(slot) => Ok(Point::fuzzy(slot.into())),
    }
}

impl TryFrom<cardano_blockchain_types::Network> for CardanoNetwork {
    type Error = CardanoError;

    fn try_from(network: cardano_blockchain_types::Network) -> Result<Self, Self::Error> {
        Ok(match network {
            cardano_blockchain_types::Network::Mainnet => CardanoNetwork::Mainnet,
            cardano_blockchain_types::Network::Preprod => CardanoNetwork::Preprod,
            cardano_blockchain_types::Network::Preview => CardanoNetwork::Preview,
            cardano_blockchain_types::Network::Devnet { magic, .. } => {
                CardanoNetwork::TestnetMagic(magic)
            },
            _ => return Err(CardanoError::UnknownNetwork),
        })
    }
}
