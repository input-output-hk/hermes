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

/// Chain follower can deliver updates at network speed
/// (hundreds per second), but WASM execution cannot match this rate. Without
/// throttling, resources accumulate faster than consumed, causing memory
/// exhaustion and system freezes on slower machines.
const BLOCK_RATE_LIMIT_MS: u64 = 10;

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
    let mut follower = ChainFollower::new(network, start, Point::TIP).await;

    // Chain updates: Chain follower delivers updates at network speed (hundreds per second),
    // which WASM execution cannot match. Without rate limiting, resources accumulate faster
    // than they're consumed, leading to memory exhaustion and system freezes.
    //
    // TODO: Replace rate limiting with lazy resource creation for better architecture.
    // Currently resources are created before events are queued, so backpressure can't prevent
    // accumulation. Ideal fix: send lightweight events with Arc<BlockData>, create resources
    // on-demand only when WASM actually accesses them. This eliminates the ordering problem
    // and allows natural backpressure without artificial throttling. Requires API redesign.
    let mut rate_limiter =
        tokio::time::interval(std::time::Duration::from_millis(BLOCK_RATE_LIMIT_MS));
    // Don't build up missed ticks if we fall behind
    rate_limiter.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

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

            // Wait for rate limiter before processing next block
            _ = rate_limiter.tick() => {
                // Only process a block if rate limit allows
                let update = follower.next().await;
                match update {
                    Some(chain_update) => {
                        // Clone block data BEFORE acquiring any locks to avoid holding locks during expensive clone
                        let block_data = chain_update.block_data().clone();

                        let Ok(block_app_state) = STATE.block.get_app_state_readonly(&app) else {
                            // This should not failed
                            error!(error="Failed to get block app state for app: {app}");
                            return
                        };
                        // Critical ordering: Resource creation happens BEFORE event queue send().
                        // Even with a bounded channel providing backpressure, the resource is already
                        // allocated in the DashMap before send() can block. Without rate limiting at
                        // this point, we can accumulate thousands of resources waiting for queue space,
                        // causing memory exhaustion. The rate limiter above ensures resources are created
                        // no faster than WASM can consume them, preventing unbounded growth.
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
) -> Point {
    match slot {
        SyncSlot::Genesis => Point::ORIGIN,
        SyncSlot::Tip => {
            match get_tips(network) {
                Ok((_, live_tip)) => Point::fuzzy(live_tip),
                Err(e) => {
                    tracing::error!(error=?e, "Failed to get tips for network {network}");
                    Point::TIP
                },
            }
        },
        SyncSlot::ImmutableTip => {
            match get_tips(network) {
                Ok((immutable_tip, _)) => Point::fuzzy(immutable_tip),
                Err(e) => {
                    tracing::error!(error=?e, "Failed to get tips for network {network}");
                    Point::ORIGIN
                },
            }
        },
        SyncSlot::Specific(slot) => Point::fuzzy(slot.into()),
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
