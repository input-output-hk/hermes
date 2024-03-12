//! Cardano Blockchain runtime extension implementation.
#![allow(unused)]

use std::{
    error::Error,
    sync::{
        atomic::{AtomicBool, AtomicU32},
        Arc,
    },
};

use dashmap::DashMap;
use tracing::{instrument, trace, warn, Instrument};

use crate::{
    app::HermesAppName,
    event::{HermesEvent, TargetApp, TargetModule},
    runtime_extensions::bindings::hermes::cardano::api::{BlockSrc, CardanoBlockchainId, Slot},
    wasm::module::ModuleId,
};

mod event;
mod host;

pub(super) type Result<T> = std::result::Result<T, Box<dyn Error>>;

struct TokioRuntimeSpawnFollowerCommand {
    follower_id: FollowerId,
    network: cardano_chain_follower::Network,
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
        &self, follower_id: FollowerId, network: cardano_chain_follower::Network,
        follow_from: cardano_chain_follower::PointOrTip,
    ) -> Result<ChainFollowerHandle> {
        let (res_tx, res_rx) = tokio::sync::oneshot::channel();
        let cmd = TokioRuntimeSpawnFollowerCommand {
            follower_id,
            follow_from,
            network,
        };

        self.cmd_tx.blocking_send((cmd, res_tx)).map_err(Box::new)?;

        // TODO(FelipeRosa): Handle errors
        let handle = res_rx.blocking_recv().expect("Tokio runtime not running");
        Ok(handle)
    }
}

type ChainFollowerHandleCommandSender = tokio::sync::mpsc::Sender<(
    cardano_chain_follower::PointOrTip,
    tokio::sync::oneshot::Sender<
        cardano_chain_follower::Result<Option<cardano_chain_follower::Point>>,
    >,
)>;
type ChainFollowerHandleCommandReceiver = tokio::sync::mpsc::Receiver<(
    cardano_chain_follower::PointOrTip,
    tokio::sync::oneshot::Sender<
        cardano_chain_follower::Result<Option<cardano_chain_follower::Point>>,
    >,
)>;

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
            .blocking_send((at, res_tx))
            .expect("Follower executor is not running");

        // TODO(FelipeRosa): Same as above.
        let result = res_rx
            .blocking_recv()
            .expect("Follower executor is not running");

        result
    }
}

struct ActiveFollower {
    handle: ChainFollowerHandle,
    at: Option<u64>,
}

type FollowerId = u32;
type ChainUpdateSender = tokio::sync::mpsc::Sender<(
    FollowerId,
    cardano_chain_follower::Result<cardano_chain_follower::ChainUpdate>,
)>;
type ChainUpdateReceiver = tokio::sync::mpsc::Receiver<(
    FollowerId,
    cardano_chain_follower::Result<cardano_chain_follower::ChainUpdate>,
)>;

struct SubscriptionState {
    stopped: AtomicBool,
    // NOTE(FelipeRosa): For now, each subscription has its own follower.
    // The intention is that, in the future, this is changed in order to maximize
    // the utilization of the active followers.
    follower_id: AtomicU32,
}

struct State {
    tokio_rt_handle: TokioRuntimeHandle,
    follower_id_counter: AtomicU32,
    active_followers: DashMap<FollowerId, ActiveFollower>,
    subscriptions: DashMap<
        HermesAppName,
        DashMap<ModuleId, DashMap<cardano_chain_follower::Network, Arc<SubscriptionState>>>,
    >,
}

impl State {
    fn next_follower_id(&self) -> FollowerId {
        let prev = self
            .follower_id_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        prev + 1
    }

    fn insert_module_subscription_state(
        &self, app_name: HermesAppName, module_id: ModuleId,
        network: cardano_chain_follower::Network, sub_state: SubscriptionState,
    ) {
        let modules_index = self.subscriptions.entry(app_name).or_default();
        let network_index = modules_index.entry(module_id).or_default();
        network_index.insert(network, Arc::new(sub_state));
    }

    fn module_subscription_state(
        &self, app_name: &HermesAppName, module_id: &ModuleId,
        network: cardano_chain_follower::Network,
    ) -> Option<Arc<SubscriptionState>> {
        let modules_index = self.subscriptions.get(app_name)?;
        let network_index = modules_index.get(module_id)?;
        let st = network_index.get(&network)?;

        Some(st.clone())
    }
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
        follower_id_counter: AtomicU32::new(0),
        active_followers: DashMap::new(),
        subscriptions: DashMap::new(),
    }
});

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(_ctx: &crate::runtime_context::HermesRuntimeContext) {}

pub(super) fn subscribe(
    chain_id: CardanoBlockchainId, app_name: HermesAppName, module_id: ModuleId, opt: Slot,
) -> Result<()> {
    // TODO(FelipeRosa): This should handle each case for the subscription.
    // follow_from should be Some(PointOrTip) if we need to follow from some specified point
    // or None if we should continue following from the last point.
    let follow_from = match opt {
        Slot::Continue => None,
        Slot::Genesis => Some(cardano_chain_follower::PointOrTip::Point(
            cardano_chain_follower::Point::Origin,
        )),
        _ => Some(cardano_chain_follower::PointOrTip::Tip),
    };

    let network = network_from_chain_id(chain_id);

    let sub_state = STATE.module_subscription_state(&app_name, &module_id, network);

    let follow_from = match follow_from {
        Some(p) => p,
        None => {
            // TODO(FelipeRosa): Implement the logic for when this is a new
            // subscription but "continue" was requested.
            cardano_chain_follower::PointOrTip::Tip
        },
    };

    match sub_state {
        Some(sub_state) => {
            // NOTE(FelipeRosa): In the case the subscription is already active,
            // we simply ask the follower to set its read pointer to the correct
            // position on the chain.

            sub_state
                .stopped
                .store(false, std::sync::atomic::Ordering::SeqCst);

            let follower_id = sub_state
                .follower_id
                .load(std::sync::atomic::Ordering::SeqCst);

            if let Some(follower) = STATE.active_followers.get(&follower_id) {
                // TODO(FelipeRosa): Handle error.
                // Maybe this should cause subscribe to fail and the module could retry?
                follower.handle.set_read_pointer_sync(follow_from).unwrap();
            }
        },
        None => {
            let follower_id = STATE.next_follower_id();

            // TODO(FelipeRosa): Handle error
            let follower_handle = STATE
                .tokio_rt_handle
                .spawn_follower_sync(follower_id, network, follow_from)
                .unwrap();

            STATE.active_followers.insert(
                follower_id,
                ActiveFollower {
                    handle: follower_handle,
                    at: None,
                },
            );

            let initial_state = SubscriptionState {
                stopped: AtomicBool::new(false),
                follower_id: AtomicU32::new(follower_id),
            };

            STATE.insert_module_subscription_state(app_name, module_id, network, initial_state);
        },
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
        let (chain_update_tx, chain_update_rx) = tokio::sync::mpsc::channel(1);

        tokio::spawn(chain_update_processor(chain_update_rx));

        while let Some((cmd, res_tx)) = cmd_rx.recv().await {
            let (follower_cmd_tx, follower_cmd_rx) = tokio::sync::mpsc::channel(1);

            trace!("Spawning chain follower executor");
            tokio::spawn(chain_follower_executor(
                follower_cmd_rx,
                chain_update_tx.clone(),
                cmd.follower_id,
                cmd.network,
            ));

            // If the receiver side is closed, just drop the handle so the
            // chain follower executor is terminated.
            drop(res_tx.send(ChainFollowerHandle {
                cmd_tx: follower_cmd_tx,
            }));
        }
    });
}

#[instrument(skip(chain_update_rx))]
async fn chain_update_processor(mut chain_update_rx: ChainUpdateReceiver) {
    while let Some((follower_id, result)) = chain_update_rx.recv().await {
        trace!("Processing chain update");

        let chain_update = match result {
            Ok(chain_update) => chain_update,
            Err(e) => {
                // TODO(FelipeRosa): Handle this
                warn!(error = ?e, "Failed to get chain update");
                continue;
            },
        };

        let current_follower_slot = match chain_update {
            cardano_chain_follower::ChainUpdate::Block(block_data) => {
                // TODO(FelipeRosa):
                // 1. Handle error
                // 2. Generate transaction events
                let decoded_block_data = block_data.decode().unwrap();
                let slot = decoded_block_data.slot();

                let on_block_event = event::OnCardanoBlockEvent {
                    blockchain: CardanoBlockchainId::Preprod,
                    block: Vec::new(),
                    source: BlockSrc::TIP,
                };
                trace!("Generated Cardano block event");

                // TODO(FelipeRosa): Handle error?
                let res = crate::event::queue::send(HermesEvent::new(
                    on_block_event,
                    TargetApp::List(Vec::new()),
                    TargetModule::All,
                ));

                slot
            },
            cardano_chain_follower::ChainUpdate::Rollback(block_data) => {
                // TODO(FelipeRosa):
                // 1. Handle error
                let decoded_block_data = block_data.decode().unwrap();
                let slot = decoded_block_data.slot();

                let on_rollback_event = event::OnCardanoRollback {
                    blockchain: CardanoBlockchainId::Preprod,
                    slot: 0,
                };
                trace!("Generated Cardano rollback event");

                // TODO(FelipeRosa): Handle error?
                let res = crate::event::queue::send(HermesEvent::new(
                    on_rollback_event,
                    TargetApp::List(Vec::new()),
                    TargetModule::All,
                ));

                slot
            },
        };

        // TODO(FelipeRosa): If there's a follower at the same point on the
        // chain as this one, merge their subscribers and keep only one of them
        // alive.
        if let Some(mut follower) = STATE.active_followers.get_mut(&follower_id) {
            follower.at = Some(current_follower_slot);
        }
    }
}

#[instrument(skip(cmd_rx, chain_update_tx))]
async fn chain_follower_executor(
    mut cmd_rx: ChainFollowerHandleCommandReceiver, chain_update_tx: ChainUpdateSender,
    follower_id: FollowerId, network: cardano_chain_follower::Network,
) {
    let config = cardano_chain_follower::FollowerConfigBuilder::default().build();

    let mut follower = cardano_chain_follower::Follower::connect(
        follower_connect_address(network),
        network,
        config,
    )
    .await
    .unwrap();
    trace!("Started chain follower");

    'exec_loop: loop {
        tokio::select! {
            res = cmd_rx.recv() => {
                match res {
                    Some((follow_from, res_tx)) => {
                        let result = follower.set_read_pointer(follow_from).await;

                        // Ignore if the receiver is closed.
                        drop(res_tx.send(result));
                    }
                    None => break 'exec_loop,
                }
            }

            result = follower.next() => {
                // Failing to send means that the runtime extension receiver side is
                // closed, in that case, we stop the chain follower.
                if chain_update_tx.send((follower_id, result)).await.is_err() {
                    break 'exec_loop;
                }
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

    use crate::{
        app::HermesAppName,
        runtime_extensions::bindings::hermes::cardano::api::{CardanoBlockchainId, Slot},
        wasm::module::ModuleId,
    };

    use super::subscribe;

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
            Slot::Tip,
        )
        .expect("subscribed");

        std::thread::sleep(std::time::Duration::from_secs(3));

        subscribe(
            CardanoBlockchainId::Preprod,
            app_name,
            module_id,
            Slot::Genesis,
        )
        .expect("subscribed");

        std::thread::sleep(std::time::Duration::from_secs(10));
    }
}
