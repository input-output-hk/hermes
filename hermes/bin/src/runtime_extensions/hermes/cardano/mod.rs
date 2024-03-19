//! Cardano Blockchain runtime extension implementation.

use dashmap::DashMap;
use tracing::{instrument, trace, warn};

use crate::{
    app::HermesAppName,
    event::{HermesEvent, TargetApp, TargetModule},
    runtime_extensions::bindings::hermes::cardano::api::{BlockSrc, CardanoBlockchainId},
    wasm::module::ModuleId,
};

mod event;
mod host;

#[derive(Debug, thiserror::Error)]
pub(super) enum Error {
    #[error("Internal Cardano Runtime Extension Error")]
    InternalError,
}

pub(super) type Result<T> = std::result::Result<T, Error>;

/// Command data that can be send to the Tokio runtime background thread.
///
/// For now it just spawns new chain followers.
struct TokioRuntimeSpawnFollowerCommand {
    app_name: HermesAppName,
    module_id: ModuleId,
    chain_id: CardanoBlockchainId,
    follow_from: cardano_chain_follower::PointOrTip,
}

type TokioRuntimeHandleCommandSender = tokio::sync::mpsc::Sender<(
    TokioRuntimeSpawnFollowerCommand,
    tokio::sync::oneshot::Sender<ChainFollowerHandle>,
)>;
type TokioRuntimeHandleCommandReceiver = tokio::sync::mpsc::Receiver<(
    TokioRuntimeSpawnFollowerCommand,
    tokio::sync::oneshot::Sender<ChainFollowerHandle>,
)>;

struct TokioRuntimeHandle {
    cmd_tx: TokioRuntimeHandleCommandSender,
}

impl TokioRuntimeHandle {
    fn spawn_follower_sync(
        &self, app_name: HermesAppName, module_id: ModuleId, chain_id: CardanoBlockchainId,
        follow_from: cardano_chain_follower::PointOrTip,
    ) -> Result<ChainFollowerHandle> {
        let (res_tx, res_rx) = tokio::sync::oneshot::channel();
        let cmd = TokioRuntimeSpawnFollowerCommand {
            app_name,
            module_id,
            chain_id,
            follow_from,
        };

        self.cmd_tx
            .blocking_send((cmd, res_tx))
            .map_err(|_| Error::InternalError)?;

        // TODO(FelipeRosa): Handle errors
        let handle = res_rx.blocking_recv().expect("Tokio runtime not running");
        Ok(handle)
    }
}

enum ChainFollowerCommand {
    SetReadPointer(
        cardano_chain_follower::PointOrTip,
        tokio::sync::oneshot::Sender<
            cardano_chain_follower::Result<Option<cardano_chain_follower::Point>>,
        >,
    ),
    Stop(tokio::sync::oneshot::Sender<()>),
    Continue(tokio::sync::oneshot::Sender<()>),
}

type ChainFollowerHandleCommandSender = tokio::sync::mpsc::Sender<ChainFollowerCommand>;
type ChainFollowerHandleCommandReceiver = tokio::sync::mpsc::Receiver<ChainFollowerCommand>;

struct ChainFollowerHandle {
    cmd_tx: ChainFollowerHandleCommandSender,
}

impl ChainFollowerHandle {
    fn set_read_pointer_sync(
        &self, at: cardano_chain_follower::PointOrTip,
    ) -> cardano_chain_follower::Result<Option<cardano_chain_follower::Point>> {
        let (res_tx, res_rx) = tokio::sync::oneshot::channel();

        // TODO(FelipeRosa): This should be mapped into an error. It's a serious bug
        // if the follower's executor was stopped and the handle was not dropped.
        self.cmd_tx
            .blocking_send(ChainFollowerCommand::SetReadPointer(at, res_tx))
            .expect("Follower executor is not running");

        // TODO(FelipeRosa): Same as above.
        let result = res_rx
            .blocking_recv()
            .expect("Follower executor is not running");

        result
    }
}

#[derive(Default)]
struct SubscriptionState {
    subscribed_to_blocks: bool,
    subscribed_to_txns: bool,
    subscribed_to_rollbacks: bool,
    follower_handle: Option<ChainFollowerHandle>,
}

struct State {
    tokio_rt_handle: TokioRuntimeHandle,
    subscriptions:
        DashMap<(HermesAppName, ModuleId, cardano_chain_follower::Network), SubscriptionState>,
}

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

pub(super) fn subscribe(
    chain_id: CardanoBlockchainId, app_name: HermesAppName, module_id: ModuleId,
    sub_type: SubscriptionType,
) -> Result<()> {
    let network = network_from_chain_id(chain_id);

    let mut sub_state = STATE
        .subscriptions
        .entry((app_name.clone(), module_id.clone(), network))
        .or_default();

    match sub_type {
        SubscriptionType::Blocks(follow_from) => {
            if let Some(handle) = sub_state.follower_handle.as_ref() {
                handle.set_read_pointer_sync(follow_from).unwrap();
            } else {
                // TODO(FelipeRosa): Handle error
                let follower_handle = STATE
                    .tokio_rt_handle
                    .spawn_follower_sync(app_name, module_id, chain_id, follow_from)
                    .unwrap();

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

                // TODO(FelipeRosa): Handle
                handle
                    .cmd_tx
                    .blocking_send(ChainFollowerCommand::Continue(res_tx))
                    .unwrap();

                drop(res_rx.blocking_recv());
            }
        },
    }
    Ok(())
}

pub(super) fn unsubscribe(
    chain_id: CardanoBlockchainId, app_name: HermesAppName, module_id: ModuleId,
    opts: crate::runtime_extensions::bindings::hermes::cardano::api::UnsubscribeOptions,
) -> Result<()> {
    use crate::runtime_extensions::bindings::hermes::cardano::api::UnsubscribeOptions;

    let network = network_from_chain_id(chain_id);
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

                // TODO(FelipeRosa): Handle
                handle
                    .cmd_tx
                    .blocking_send(ChainFollowerCommand::Stop(res_tx))
                    .unwrap();

                drop(res_rx.blocking_recv());
            }
        }
    }

    Ok(())
}

#[instrument(skip(cmd_rx))]
fn tokio_runtime_executor(mut cmd_rx: TokioRuntimeHandleCommandReceiver) {
    // TODO(FelipeRosa): Handle error
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()
        .unwrap();
    trace!("Created Tokio runtime");

    rt.block_on(async move {
        while let Some((cmd, res_tx)) = cmd_rx.recv().await {
            let (follower_cmd_tx, follower_cmd_rx) = tokio::sync::mpsc::channel(1);

            trace!("Spawning chain follower executor");
            tokio::spawn(chain_follower_executor(
                follower_cmd_rx,
                cmd.app_name,
                cmd.module_id,
                cmd.chain_id,
                cmd.follow_from,
            ));

            drop(res_tx.send(ChainFollowerHandle {
                cmd_tx: follower_cmd_tx,
            }));
        }
    });
}

#[instrument(skip(cmd_rx, follow_from), fields(app_name = %app_name, module_id = %module_id))]
async fn chain_follower_executor(
    mut cmd_rx: ChainFollowerHandleCommandReceiver, app_name: HermesAppName, module_id: ModuleId,
    chain_id: CardanoBlockchainId, follow_from: cardano_chain_follower::PointOrTip,
) {
    let config = cardano_chain_follower::FollowerConfigBuilder::default().build();
    let network = network_from_chain_id(chain_id);
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

                        // TODO(FelipeRosa):
                        // 1. Handle error
                        let decoded_block_data = block_data.decode().unwrap();
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
                                source: BlockSrc::NODE,
                            };
                            trace!(block_number, "Generated Cardano block event");

                            // TODO(FelipeRosa): Handle error?
                            drop(crate::event::queue::send(HermesEvent::new(
                                on_block_event,
                                TargetApp::List(vec![module_state_key.0.clone()]),
                                TargetModule::_List(vec![module_state_key.1.clone()]),
                            )));
                        }


                        slot
                    },
                    cardano_chain_follower::ChainUpdate::Rollback(block_data) => {
                        if !subscribed_to_rollbacks {
                            continue;
                        }

                        // TODO(FelipeRosa):
                        // 1. Handle error
                        let decoded_block_data = block_data.decode().unwrap();
                        let block_number = decoded_block_data.number();
                        let slot = decoded_block_data.slot();

                        let on_rollback_event = event::OnCardanoRollback {
                            blockchain: CardanoBlockchainId::Preprod,
                            slot: 0,
                        };
                        trace!(block_number, "Generated Cardano rollback event");

                        // TODO(FelipeRosa): Handle error?
                        drop(crate::event::queue::send(HermesEvent::new(
                            on_rollback_event,
                            TargetApp::List(Vec::new()),
                            TargetModule::All,
                        )));

                        slot
                    },
                };
            }
        }
    }

    // TODO(FelipeRosa): Stop waiting if this times out.
    drop(follower.close().await);
}

const fn follower_connect_address(network: cardano_chain_follower::Network) -> &'static str {
    match network {
        cardano_chain_follower::Network::Mainnet => "backbone.cardano-mainnet.iohk.io:3001",
        cardano_chain_follower::Network::Preprod => "preprod-node.play.dev.cardano.org:3001",
        cardano_chain_follower::Network::Preview => "preview-node.play.dev.cardano.org:3001",
        cardano_chain_follower::Network::Testnet => todo!(),
    }
}

const fn network_from_chain_id(chain_id: CardanoBlockchainId) -> cardano_chain_follower::Network {
    match chain_id {
        CardanoBlockchainId::Mainnet => cardano_chain_follower::Network::Mainnet,
        CardanoBlockchainId::Preprod => cardano_chain_follower::Network::Preprod,
        CardanoBlockchainId::Preview => cardano_chain_follower::Network::Preview,
        CardanoBlockchainId::LocalTestBlockchain => todo!(),
    }
}

#[cfg(test)]
mod test {
    use rusty_ulid::Ulid;

    use super::{subscribe, unsubscribe, SubscriptionType};
    use crate::{
        app::HermesAppName,
        runtime_extensions::bindings::hermes::cardano::api::{
            CardanoBlockchainId, UnsubscribeOptions,
        },
        wasm::module::ModuleId,
    };

    #[test]
    fn it_works() {
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
            SubscriptionType::Blocks(cardano_chain_follower::PointOrTip::Point(
                cardano_chain_follower::Point::Specific(
                    49_075_522,
                    hex::decode("b7639b523f320643236ab0fc04b7fd381dedd42c8d6b6433b5965a5062411396")
                        .unwrap(),
                ),
            )),
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
}
