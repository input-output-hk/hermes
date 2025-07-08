use cardano_blockchain_types::{MultiEraBlock, Network, Point, Slot};
use cardano_chain_follower::{ChainFollower, Kind};

use crate::{
    app::ApplicationName,
    runtime_extensions::{
        bindings::hermes::cardano::api::{CardanoNetwork, CreateNetworkError, SyncSlot},
        hermes::cardano::{
            event::{build_and_send_block_event, build_and_send_roll_forward_event},
            host::SubscriptionType,
        },
    },
    wasm::module::ModuleId,
};

impl TryFrom<CardanoNetwork> for cardano_blockchain_types::Network {
    type Error = CreateNetworkError;

    fn try_from(network: CardanoNetwork) -> Result<Self, Self::Error> {
        match network {
            CardanoNetwork::Mainnet => Ok(cardano_blockchain_types::Network::Mainnet),
            CardanoNetwork::Preprod => Ok(cardano_blockchain_types::Network::Preprod),
            CardanoNetwork::Preview => Ok(cardano_blockchain_types::Network::Preview),
            CardanoNetwork::TestnetMagic(_) => Err(CreateNetworkError::NetworkNotSupport),
        }
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

impl From<SyncSlot> for Point {
    fn from(value: SyncSlot) -> Self {
        match value {
            SyncSlot::Genesis => Point::ORIGIN,
            SyncSlot::Tip => Point::TIP,
            // FIXME
            SyncSlot::ImmutableTip => Point::TIP,
            SyncSlot::Specific(slot) => Point::fuzzy(slot.into()),
        }
    }
}

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
    cmd_tx: CommandSender,
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

pub(crate) fn spawn_subscribe(
    app: ApplicationName, module_id: ModuleId, start: Point, network: Network,
    subscription_type: SubscriptionType, subscription_id: u32,
) -> Handle {
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel(1);

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to build Tokio runtime");

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

    Handle { cmd_tx }
}

async fn subscribe(
    mut cmd_rx: CommandReceiver, app: ApplicationName, module_id: ModuleId, start: Point,
    network: Network, subscription_type: SubscriptionType, subscription_id: u32,
) {
    let mut follower = ChainFollower::new(network, start, Point::TIP).await;

    loop {
        tokio::select! {
            // Handle stop command
            res = cmd_rx.recv() => {
                match res {
                    Some(Command::Stop(res_tx)) => {
                        let _ = res_tx.send(());
                        break;
                    }
                    None => {
                        break;
                    }
                }
            }

            // Handle new block update
            update = follower.next() => {
                match update {
                    Some(chain_update) => {
                        let block_data = chain_update.block_data();

                        match chain_update.kind {
                            Kind::Block if subscription_type == SubscriptionType::Block => {
                                build_and_send_block_event(
                                    app.clone(),
                                    module_id.clone(),
                                    subscription_id,
                                    network.into(),
                                    block_data.raw(),
                                    block_data.slot().into(),
                                    chain_update.immutable(),
                                    false,
                                );
                            }
                            Kind::Rollback if subscription_type == SubscriptionType::Block => {
                                build_and_send_block_event(
                                    app.clone(),
                                    module_id.clone(),
                                    subscription_id,
                                    network.into(),
                                    block_data.raw(),
                                    block_data.slot().into(),
                                    chain_update.immutable(),
                                    true,
                                );
                            }
                            Kind::ImmutableBlockRollForward if subscription_type == SubscriptionType::ImmutableRollForward => {
                                build_and_send_roll_forward_event(
                                    app.clone(),
                                    module_id.clone(),
                                    subscription_id,
                                    network.into(),
                                    block_data.slot().into(),
                                );
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

pub(crate) fn get_block_relative(
    chain: Network, start: Option<u64>, step: i64,
) -> anyhow::Result<MultiEraBlock> {
    let handle = std::thread::spawn(move || -> anyhow::Result<MultiEraBlock> {
        let point = if let Some(start_point) = start {
            let target = if step.is_negative() {
                start_point
                    .checked_sub(step.unsigned_abs())
                    .ok_or_else(|| anyhow::anyhow!("Step causes underflow"))?
            } else {
                start_point
                    .checked_add(step.unsigned_abs())
                    .ok_or_else(|| anyhow::anyhow!("Step causes overflow"))?
            };
            Point::fuzzy(target.into())
        } else {
            Point::TIP
        };

        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .enable_io()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to create Tokio runtime: {e}"));
            },
        };

        let block = rt
            .block_on(ChainFollower::get_block(chain, point))
            .ok_or_else(|| anyhow::anyhow!("Failed to fetch block at point"))?;

        Ok(block.data)
    });

    handle
        .join()
        .map_err(|e| anyhow::anyhow!("Thread panicked while getting block: {e:?}"))?
}

pub(crate) fn get_tips(chain: Network) -> anyhow::Result<(Slot, Slot)> {
    let handle = std::thread::spawn(move || -> anyhow::Result<(Slot, Slot)> {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .enable_io()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to create Tokio runtime: {e}"));
            },
        };

        let (immutable_tip, live_tip) = rt.block_on(ChainFollower::get_tips(chain));
        Ok((immutable_tip.slot_or_default(), live_tip.slot_or_default()))
    });

    handle
        .join()
        .map_err(|e| anyhow::anyhow!("Thread panicked while getting block: {e:?}"))?
}
