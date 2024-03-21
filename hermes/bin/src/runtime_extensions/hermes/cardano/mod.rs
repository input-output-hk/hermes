//! Cardano Blockchain runtime extension implementation.

use dashmap::DashMap;
use tracing::{error, instrument, trace, warn};

use crate::{
    app::HermesAppName,
    event::{HermesEvent, TargetApp, TargetModule},
    runtime_extensions::bindings::hermes::cardano::api::{BlockSrc, CardanoBlockchainId},
    wasm::module::ModuleId,
};

mod event;
mod host;

/// Cardano Runtime Extension internal error type.
#[derive(Debug, thiserror::Error)]
pub(super) enum Error {
    /// General internal error.
    #[error("Internal Cardano Runtime Extension Error")]
    InternalError,
}

/// Cardano Runtime Extension internal result type.
pub(super) type Result<T> = std::result::Result<T, Error>;

/// Command data that can be send to the Tokio runtime background thread.
enum TokioRuntimeCommand {
    /// Instructs the Tokio runtime background thread to spawn a new chain follower.
    SpawnFollower {
        /// Name of the app that the follower will be tied to.
        app_name: HermesAppName,
        /// Id of the module that the follower will be tied to.
        module_id: ModuleId,
        /// Cardano blockchain that the follower will connect to.
        chain_id: CardanoBlockchainId,
        /// Follower's starting point.
        follow_from: cardano_chain_follower::PointOrTip,
        /// Response channel sender.
        response_tx: tokio::sync::oneshot::Sender<ChainFollowerHandle>,
    },
    /// Instructs the Tokio runtime background thread to read a block using some follower.
    ReadBlock {
        chain_id: CardanoBlockchainId,
        at: cardano_chain_follower::PointOrTip,
        response_tx:
            tokio::sync::oneshot::Sender<Result<cardano_chain_follower::MultiEraBlockData>>,
    },
}

/// Tokio runtime handle command channel sender type.
type TokioRuntimeHandleCommandSender = tokio::sync::mpsc::Sender<TokioRuntimeCommand>;

/// Tokio runtime handle command channel receiver type.
type TokioRuntimeHandleCommandReceiver = tokio::sync::mpsc::Receiver<TokioRuntimeCommand>;

/// Handle used for communicating with the Tokio runtime background thread.
struct TokioRuntimeHandle {
    /// Commands channel sender.
    cmd_tx: TokioRuntimeHandleCommandSender,
}

impl TokioRuntimeHandle {
    /// Spawns a new chain follower in the background Tokio runtime.
    ///
    /// # Errors
    ///
    /// Returns Err if the chain follower executor Tokio task could not be spawned.
    fn spawn_follower_sync(
        &self, app_name: HermesAppName, module_id: ModuleId, chain_id: CardanoBlockchainId,
        follow_from: cardano_chain_follower::PointOrTip,
    ) -> Result<ChainFollowerHandle> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        let cmd = TokioRuntimeCommand::SpawnFollower {
            app_name,
            module_id,
            chain_id,
            follow_from,
            response_tx,
        };

        self.cmd_tx
            .blocking_send(cmd)
            .map_err(|_| Error::InternalError)?;

        response_rx
            .blocking_recv()
            .map_err(|_| Error::InternalError)
    }
}

/// Chain follower executor commands.
enum ChainFollowerCommand {
    /// Instructs the chain follower executor to set the read pointer to the specified
    /// position.
    SetReadPointer(
        cardano_chain_follower::PointOrTip,
        tokio::sync::oneshot::Sender<
            cardano_chain_follower::Result<Option<cardano_chain_follower::Point>>,
        >,
    ),
    /// Instructs the chain follower to stop generating events.
    Stop(tokio::sync::oneshot::Sender<()>),
    /// Instructs the chain follower to resume generating events.
    Continue(tokio::sync::oneshot::Sender<()>),
}

/// Chain follower handle command channel sender.
type ChainFollowerHandleCommandSender = tokio::sync::mpsc::Sender<ChainFollowerCommand>;
/// Chain follower handle command channel receiver.
type ChainFollowerHandleCommandReceiver = tokio::sync::mpsc::Receiver<ChainFollowerCommand>;

/// Handle used to communicate with a chain follower executor Tokio task.
struct ChainFollowerHandle {
    /// Commands channel sender.
    cmd_tx: ChainFollowerHandleCommandSender,
}

impl ChainFollowerHandle {
    /// Sends a command to the chain follower executor Tokio task to set its
    /// read pointer to the given point.
    fn set_read_pointer_sync(
        &self, at: cardano_chain_follower::PointOrTip,
    ) -> Result<Option<cardano_chain_follower::Point>> {
        let (res_tx, res_rx) = tokio::sync::oneshot::channel();

        self.cmd_tx
            .blocking_send(ChainFollowerCommand::SetReadPointer(at, res_tx))
            .map_err(|_| Error::InternalError)?;

        res_rx
            .blocking_recv()
            .map_err(|_| Error::InternalError)?
            .map_err(|_| Error::InternalError)
    }
}

/// Hermes application module subscription state.
#[derive(Default)]
struct SubscriptionState {
    /// Whether the module is subscribed to receive block events.
    subscribed_to_blocks: bool,
    /// Whether the module is subscribed to receive transaction events.
    subscribed_to_txns: bool,
    /// Whether the module is subscribed to receive rollback events.
    subscribed_to_rollbacks: bool,
    /// Handle to the cardano chain follower from which the module is receiving
    /// events.
    follower_handle: Option<ChainFollowerHandle>,
}

/// Cardano Runtime Extension state.
struct State {
    /// Handle to the Tokio runtime background thread.
    tokio_rt_handle: TokioRuntimeHandle,
    /// Mapping of application module subscription states.
    subscriptions:
        DashMap<(HermesAppName, ModuleId, cardano_chain_follower::Network), SubscriptionState>,
    /// Chain followers configured only for reading blocks.
    readers: DashMap<cardano_chain_follower::Network, cardano_chain_follower::Follower>,
}

/// Cardano Runtime Extension internal state.
static STATE: once_cell::sync::Lazy<State> = once_cell::sync::Lazy::new(|| {
    // Spawn a thread for running a Tokio runtime if we haven't yet.
    // This is done so we can run async functions.
    let (tokio_cmd_tx, tokio_cmd_rx) = tokio::sync::mpsc::channel(1);
    std::thread::spawn(move || {
        tokio_runtime_executor(tokio_cmd_rx);
    });

    State {
        tokio_rt_handle: TokioRuntimeHandle {
            cmd_tx: tokio_cmd_tx,
        },
        subscriptions: DashMap::new(),
        readers: DashMap::new(),
    }
});

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(_ctx: &crate::runtime_context::HermesRuntimeContext) {}

pub(super) enum SubscriptionType {
    Blocks(cardano_chain_follower::PointOrTip),
    Rollbacks,
    Transactions,
    Continue,
}

/// Subscribes a module or resumes the generation of subscribed events for a module.
pub(super) fn subscribe(
    chain_id: CardanoBlockchainId, app_name: HermesAppName, module_id: ModuleId,
    sub_type: SubscriptionType,
) -> Result<()> {
    let network = chain_id.into();

    let mut sub_state = STATE
        .subscriptions
        .entry((app_name.clone(), module_id.clone(), network))
        .or_default();

    match sub_type {
        SubscriptionType::Blocks(follow_from) => {
            if let Some(handle) = sub_state.follower_handle.as_ref() {
                handle.set_read_pointer_sync(follow_from)?;
            } else {
                let follower_handle = STATE
                    .tokio_rt_handle
                    .spawn_follower_sync(app_name, module_id, chain_id, follow_from)
                    .map_err(|_| Error::InternalError)?;

                sub_state.follower_handle = Some(follower_handle);
            }

            sub_state.subscribed_to_blocks = true;
        },
        SubscriptionType::Rollbacks => {
            sub_state.subscribed_to_rollbacks = true;
        },
        SubscriptionType::Transactions => {
            sub_state.subscribed_to_txns = true;
        },
        SubscriptionType::Continue => {
            if let Some(handle) = sub_state.follower_handle.as_ref() {
                let (res_tx, res_rx) = tokio::sync::oneshot::channel();

                handle
                    .cmd_tx
                    .blocking_send(ChainFollowerCommand::Continue(res_tx))
                    .map_err(|_| Error::InternalError)?;

                drop(res_rx.blocking_recv());
            }
        },
    }

    Ok(())
}

/// Unsubscribes a module or stops the generation of subscribed events for a module.
pub(super) fn unsubscribe(
    chain_id: CardanoBlockchainId, app_name: HermesAppName, module_id: ModuleId,
    opts: crate::runtime_extensions::bindings::hermes::cardano::api::UnsubscribeOptions,
) -> Result<()> {
    use crate::runtime_extensions::bindings::hermes::cardano::api::UnsubscribeOptions;

    let network = chain_id.into();
    let sub_state = STATE.subscriptions.get_mut(&(app_name, module_id, network));

    if let Some(mut sub_state) = sub_state {
        if opts & UnsubscribeOptions::BLOCK == UnsubscribeOptions::BLOCK {
            sub_state.subscribed_to_blocks = false;
        }

        if opts & UnsubscribeOptions::TRANSACTION == UnsubscribeOptions::TRANSACTION {
            sub_state.subscribed_to_txns = false;
        }

        if opts & UnsubscribeOptions::ROLLBACK == UnsubscribeOptions::ROLLBACK {
            sub_state.subscribed_to_rollbacks = false;
        }

        if opts & UnsubscribeOptions::STOP == UnsubscribeOptions::STOP {
            if let Some(handle) = sub_state.follower_handle.as_ref() {
                let (res_tx, res_rx) = tokio::sync::oneshot::channel();

                handle
                    .cmd_tx
                    .blocking_send(ChainFollowerCommand::Stop(res_tx))
                    .map_err(|_| Error::InternalError)?;

                drop(res_rx.blocking_recv());
            }
        }
    }

    Ok(())
}

/// Reads a block from a Cardano network.
pub(super) fn read_block(
    chain_id: CardanoBlockchainId, at: cardano_chain_follower::PointOrTip,
) -> Result<cardano_chain_follower::MultiEraBlockData> {
    let (response_tx, response_rx) = tokio::sync::oneshot::channel();

    let cmd = TokioRuntimeCommand::ReadBlock {
        chain_id,
        at,
        response_tx,
    };

    STATE
        .tokio_rt_handle
        .cmd_tx
        .blocking_send(cmd)
        .map_err(|_| Error::InternalError)?;

    response_rx
        .blocking_recv()
        .map_err(|_| Error::InternalError)?
}

/// Runs the Cardano Runtime Extension Tokio runtime.
#[instrument(skip(cmd_rx))]
fn tokio_runtime_executor(mut cmd_rx: TokioRuntimeHandleCommandReceiver) {
    let res = tokio::runtime::Builder::new_current_thread()
        .enable_io()
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
                TokioRuntimeCommand::SpawnFollower {
                    app_name,
                    module_id,
                    chain_id,
                    follow_from,
                    response_tx,
                } => {
                    trace!("Spawning chain follower executor");

                    let (follower_cmd_tx, follower_cmd_rx) = tokio::sync::mpsc::channel(1);

                    tokio::spawn(chain_follower_executor(
                        follower_cmd_rx,
                        app_name,
                        module_id,
                        chain_id,
                        follow_from,
                    ));

                    drop(response_tx.send(ChainFollowerHandle {
                        cmd_tx: follower_cmd_tx,
                    }));
                },
                TokioRuntimeCommand::ReadBlock {
                    chain_id,
                    at,
                    response_tx,
                } => {
                    trace!("Reading block");

                    let network = chain_id.into();

                    let block_data = match STATE.readers.get(&network) {
                        Some(reader) => reader.read_block(at).await.unwrap(),
                        None => {
                            // Limit the follower's buffer size. This does not really matter since
                            // we'll not poll the follower's future so the following process will
                            // not be executed.
                            let cfg = cardano_chain_follower::FollowerConfigBuilder::default()
                                .chain_update_buffer_size(1)
                                .build();

                            let reader = cardano_chain_follower::Follower::connect(
                                follower_connect_address(network),
                                network,
                                cfg,
                            )
                            .await
                            .unwrap();

                            reader.read_block(at).await.unwrap()
                        },
                    };

                    drop(response_tx.send(Ok(block_data)));
                },
            }
        }
    });
}

/// Runs a Cardano chain follower that generates events for the given application module
/// and is connected to the given chain.
#[instrument(skip(cmd_rx, follow_from), fields(app_name = %app_name, module_id = %module_id))]
async fn chain_follower_executor(
    mut cmd_rx: ChainFollowerHandleCommandReceiver, app_name: HermesAppName, module_id: ModuleId,
    chain_id: CardanoBlockchainId, follow_from: cardano_chain_follower::PointOrTip,
) {
    let config = cardano_chain_follower::FollowerConfigBuilder::default().build();
    let network = chain_id.into();
    let module_state_key = (app_name, module_id, network);

    let mut follower = cardano_chain_follower::Follower::connect(
        follower_connect_address(network),
        network,
        config,
    )
    .await
    .unwrap();
    trace!("Started chain follower");

    follower.set_read_pointer(follow_from).await.unwrap();
    trace!("Set chain follower starting point");

    let mut stopped = false;

    'exec_loop: loop {
        tokio::select! {
            res = cmd_rx.recv() => {
                let Some(cmd) = res else {
                    break 'exec_loop;
                };

                match cmd {
                    ChainFollowerCommand::SetReadPointer(follow_from, res_tx) => {
                        // Set the follower as stopped in case we fail set the
                        // read pointer or the point can't be found.
                        stopped = true;

                        let result = follower.set_read_pointer(follow_from).await;

                        match &result {
                            Ok(Some(point)) => {
                                stopped = false;
                                trace!(slot = point.slot_or_default(), "Follower read pointer set");
                            }
                            // TODO(FelipeRosa): Decide what to do with these. For now we just
                            // will not resume the follower.
                            Ok(None) => {
                                warn!("Couldn't set follower read pointer: point not found");
                            }
                            Err(e) => {
                                warn!(error = ?e, "Failed to set read pointer");
                            }
                        }


                        // Ignore if the receiver is closed.
                        drop(res_tx.send(result));
                    }
                    ChainFollowerCommand::Stop(res_tx) => {
                        stopped = true;
                        let _ = res_tx.send(());
                    }
                    ChainFollowerCommand::Continue(res_tx) => {
                        stopped = false;
                        let _ = res_tx.send(());
                    }
                }
            }

            result = follower.next(), if !stopped => {
                let chain_update = match result {
                    Ok(chain_update) => chain_update,
                    Err(e) => {
                        // TODO(FelipeRosa): Decide what to do with this
                        warn!(error = ?e, "Failed to get chain update");
                        break 'exec_loop;
                    },
                };

                let (subscribed_to_blocks, subscribed_to_txns, subscribed_to_rollbacks) = {
                    let Some(sub_state) = STATE.subscriptions.get(&module_state_key) else {
                        break 'exec_loop;
                    };

                    (sub_state.subscribed_to_blocks, sub_state.subscribed_to_txns, sub_state.subscribed_to_rollbacks)
                };

                match chain_update {
                    cardano_chain_follower::ChainUpdate::Block(block_data) => {
                        if !subscribed_to_blocks && !subscribed_to_txns {
                            continue;
                        }

                        let decoded_block_data = match block_data.decode() {
                            Ok(b) => b,
                            Err(err) => {
                                error!(error = ?err, "Failed to decode block");
                                continue;
                            }
                        };

                        let block_number = decoded_block_data.number();
                        let slot = decoded_block_data.slot();

                        if subscribed_to_txns {
                            let txs = decoded_block_data.txs();

                            for (index, tx) in txs.iter().enumerate() {
                                let on_txn_event = event::OnCardanoTxnEvent {
                                    blockchain: chain_id,
                                    slot,
                                    txn_index: index as u32,
                                    txn: tx.encode(),
                                };

                                drop(crate::event::queue::send(HermesEvent::new(
                                    on_txn_event,
                                    TargetApp::List(vec![module_state_key.0.clone()]),
                                    TargetModule::_List(vec![module_state_key.1.clone()]),
                                )));
                            }

                            trace!(block_number, tx_count = txs.len(), "Generated Cardano block transactions events");
                        }

                        if subscribed_to_blocks {
                            let on_block_event = event::OnCardanoBlockEvent {
                                blockchain: chain_id,
                                block: block_data.into_raw_data(),
                                // TODO(FelipeRosa): In order to implement this we need the
                                // cardano-chain-follower crate to give this information along
                                // with the chain update.
                                source: BlockSrc::NODE,
                            };
                            trace!(block_number, "Generated Cardano block event");

                            let res = crate::event::queue::send(HermesEvent::new(
                                on_block_event,
                                TargetApp::List(vec![module_state_key.0.clone()]),
                                TargetModule::_List(vec![module_state_key.1.clone()]),
                            ));

                            if let Err(err) = res {
                                error!(error = ?err, "Failed to send Cardano block event to the Event queue");
                            }
                        }
                    },
                    cardano_chain_follower::ChainUpdate::Rollback(block_data) => {
                        if !subscribed_to_rollbacks {
                            continue;
                        }

                        let decoded_block_data = match block_data.decode() {
                            Ok(b) => b,
                            Err(err) => {
                                error!(error = ?err, "Failed to decode block");
                                continue;
                            }
                        };

                        let block_number = decoded_block_data.number();
                        let slot = decoded_block_data.slot();

                        let on_rollback_event = event::OnCardanoRollback {
                            blockchain: CardanoBlockchainId::Preprod,
                            slot,
                        };
                        trace!(block_number, "Generated Cardano rollback event");

                        let res = crate::event::queue::send(HermesEvent::new(
                            on_rollback_event,
                            TargetApp::List(vec![module_state_key.0.clone()]),
                            TargetModule::_List(vec![module_state_key.1.clone()]),
                        ));

                        if let Err(err) = res {
                            error!(error = ?err, "Failed to send Cardano block event to the Event queue");
                        }
                    },
                };
            }
        }
    }

    // TODO(FelipeRosa): Stop waiting if this times out.
    drop(follower.close().await);
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

impl From<CardanoBlockchainId> for cardano_chain_follower::Network {
    fn from(chain_id: CardanoBlockchainId) -> Self {
        match chain_id {
            CardanoBlockchainId::Mainnet => cardano_chain_follower::Network::Mainnet,
            CardanoBlockchainId::Preprod => cardano_chain_follower::Network::Preprod,
            CardanoBlockchainId::Preview => cardano_chain_follower::Network::Preview,
            CardanoBlockchainId::LocalTestBlockchain => todo!(),
        }
    }
}

#[cfg(test)]
mod test {
    use rusty_ulid::Ulid;

    use super::{read_block, subscribe, unsubscribe, SubscriptionType};
    use crate::{
        app::HermesAppName,
        runtime_extensions::bindings::hermes::cardano::api::{
            CardanoBlockchainId, UnsubscribeOptions,
        },
        wasm::module::ModuleId,
    };

    #[test]
    #[ignore = "Just for testing locally"]
    fn subscription_works() {
        tracing_subscriber::fmt()
            .with_thread_ids(true)
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();

        let app_name = HermesAppName("test_app_it_works".to_string());
        let module_id = ModuleId(Ulid::generate());

        subscribe(
            CardanoBlockchainId::Preprod,
            app_name.clone(),
            module_id.clone(),
            SubscriptionType::Rollbacks,
        )
        .expect("subscribed");

        subscribe(
            CardanoBlockchainId::Preprod,
            app_name.clone(),
            module_id.clone(),
            SubscriptionType::Blocks(cardano_chain_follower::PointOrTip::Tip),
        )
        .expect("subscribed");

        subscribe(
            CardanoBlockchainId::Preprod,
            app_name.clone(),
            module_id.clone(),
            SubscriptionType::Transactions,
        )
        .expect("subscribed");

        std::thread::sleep(std::time::Duration::from_secs(5));

        subscribe(
            CardanoBlockchainId::Preprod,
            app_name.clone(),
            module_id.clone(),
            SubscriptionType::Blocks(
                cardano_chain_follower::Point::Specific(
                    49_075_522,
                    hex::decode("b7639b523f320643236ab0fc04b7fd381dedd42c8d6b6433b5965a5062411396")
                        .unwrap(),
                )
                .into(),
            ),
        )
        .expect("subscribed");

        std::thread::sleep(std::time::Duration::from_secs(5));

        unsubscribe(
            CardanoBlockchainId::Preprod,
            app_name.clone(),
            module_id.clone(),
            UnsubscribeOptions::BLOCK,
        )
        .expect("subscribed");

        std::thread::sleep(std::time::Duration::from_secs(5));

        unsubscribe(
            CardanoBlockchainId::Preprod,
            app_name.clone(),
            module_id.clone(),
            UnsubscribeOptions::STOP,
        )
        .expect("subscribed");

        std::thread::sleep(std::time::Duration::from_secs(5));

        subscribe(
            CardanoBlockchainId::Preprod,
            app_name,
            module_id,
            SubscriptionType::Continue,
        )
        .expect("subscribed");

        std::thread::sleep(std::time::Duration::from_secs(100));
    }

    #[test]
    #[ignore = "Just for local testing"]
    fn reading_works() {
        tracing_subscriber::fmt()
            .with_thread_ids(true)
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();

        let block_data = read_block(
            CardanoBlockchainId::Preprod,
            cardano_chain_follower::Point::Specific(
                49_075_522,
                hex::decode("b7639b523f320643236ab0fc04b7fd381dedd42c8d6b6433b5965a5062411396")
                    .unwrap(),
            )
            .into(),
        )
        .expect("read");

        assert_eq!(block_data.decode().expect("valid block").slot(), 49_075_522);
    }
}
