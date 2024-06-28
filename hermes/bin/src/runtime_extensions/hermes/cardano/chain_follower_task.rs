//! A Chain Follower task is responsible for managing a Cardano Chain Follower
//! that is controlled by the Cardano Runtime Extension.

use std::time::Duration;

use anyhow::Context;
use cardano_chain_follower::ChainUpdate;
use pallas::ledger::traverse::{wellknown::GenesisValues, MultiEraBlock, MultiEraTx};
use tracing::{error, instrument, trace, warn};

use super::{ModuleStateKey, Result, STATE};
use crate::{
    app::HermesAppName,
    event::{HermesEvent, TargetApp, TargetModule},
    runtime_extensions::bindings::{
        hermes::cardano::api::{BlockDetail, BlockSrc, CardanoBlockchainId},
        wasi::clocks::wall_clock::Datetime,
    },
    wasm::module::ModuleId,
};

/// Holds flags specifying which event subscriptions are active.
struct EventSubscriptions {
    /// Whether the module is subscribed to block events.
    blocks: bool,
    /// Whether the module is subscribed to transaction events.
    txns: bool,
}

/// Chain follower executor commands.
enum Command {
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
type CommandSender = tokio::sync::mpsc::Sender<Command>;
/// Chain follower handle command channel receiver.
type CommandReceiver = tokio::sync::mpsc::Receiver<Command>;

/// Handle used to communicate with a chain follower executor task.
pub struct Handle {
    /// Commands channel sender.
    cmd_tx: CommandSender,
}

impl Handle {
    /// Sends a command to the chain follower executor task to set its
    /// read pointer to the given point.
    pub fn set_read_pointer_sync(
        &self, at: cardano_chain_follower::PointOrTip,
    ) -> Result<Option<cardano_chain_follower::Point>> {
        let (res_tx, res_rx) = tokio::sync::oneshot::channel();

        self.cmd_tx
            .blocking_send(Command::SetReadPointer(at, res_tx))?;

        let maybe_point = res_rx.blocking_recv()??;

        Ok(maybe_point)
    }

    /// Sends a command to the chain follower executor task to stop following.
    /// The follower continues active and following can be resumed by calling
    /// [`Self::resume`].
    pub fn stop(&self) -> Result<()> {
        let (res_tx, res_rx) = tokio::sync::oneshot::channel();

        self.cmd_tx.blocking_send(Command::Stop(res_tx))?;

        drop(res_rx.blocking_recv());

        Ok(())
    }

    /// Sends a command to the chain follower executor task to resume following
    /// from the point it was previously stopped.
    ///
    /// This has no effect if the follower is not stopped.
    pub fn resume(&self) -> Result<()> {
        let (res_tx, res_rx) = tokio::sync::oneshot::channel();

        self.cmd_tx.blocking_send(Command::Continue(res_tx))?;

        drop(res_rx.blocking_recv());

        Ok(())
    }
}

/// Spawns a new Chain Follower task in the current Tokio runtime.
pub fn spawn(
    follower: cardano_chain_follower::Follower, app_name: HermesAppName, module_id: ModuleId,
    chain_id: CardanoBlockchainId,
) -> Handle {
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel(1);

    tokio::spawn(super::chain_follower_task::executor(
        cmd_rx, follower, app_name, module_id, chain_id,
    ));

    Handle { cmd_tx }
}

/// Runs a Cardano chain follower that generates events for the given application module
/// and is connected to the given chain.
#[instrument(skip(cmd_rx, follower), fields(app_name = %app_name, module_id = %module_id))]
async fn executor(
    mut cmd_rx: CommandReceiver, mut follower: cardano_chain_follower::Follower,
    app_name: HermesAppName, module_id: ModuleId, chain_id: CardanoBlockchainId,
) {
    let network = chain_id.into();
    let module_state_key = (app_name, module_id, network);

    let mut stopped = false;

    'exec_loop: loop {
        tokio::select! {
            res = cmd_rx.recv() => {
                let Some(cmd) = res else {
                    break 'exec_loop;
                };

                stopped = process_command(cmd, &follower).await;
            }

            result = follower.next(), if !stopped => {
                match result {
                    Ok(chain_update) => {
                        let Ok(event_subscriptions) = get_event_subscriptions(&module_state_key) else {
                            break 'exec_loop;
                        };

                        match process_chain_update(chain_update, &module_state_key, chain_id, &event_subscriptions) {
                            Ok(current_slot) => {
                                if update_current_slot(&module_state_key, current_slot).is_err() {
                                    break 'exec_loop;
                                }
                            }
                            Err(e) => {
                                error!(error = ?e, "Failed to process chain update");
                                break 'exec_loop;
                            }
                        }

                    }
                    Err(e) => {
                        // TODO(FelipeRosa): Decide what to do with this
                        error!(error = ?e, "Failed to get chain update");
                        break 'exec_loop;
                    },
                }
            }
        }
    }

    drop(tokio::time::timeout(Duration::from_secs(15), follower.close()).await);
}

/// Processes a chain follower task command.
async fn process_command(cmd: Command, follower: &cardano_chain_follower::Follower) -> bool {
    match cmd {
        Command::SetReadPointer(follow_from, res_tx) => {
            // Set the follower as stopped in case we fail set the
            // read pointer or the point can't be found.
            let mut should_stop = true;

            let result = follower.set_read_pointer(follow_from).await;

            match &result {
                Ok(Some(point)) => {
                    should_stop = false;
                    trace!(slot = point.slot_or_default(), "Follower read pointer set");
                },
                // TODO(FelipeRosa): Decide what to do with these. For now we just
                // will not resume the follower.
                Ok(None) => {
                    warn!("Couldn't set follower read pointer: point not found");
                },
                Err(e) => {
                    error!(error = ?e, "Failed to set read pointer");
                },
            }

            // Ignore if the receiver is closed.
            drop(res_tx.send(result));

            should_stop
        },
        Command::Stop(res_tx) => {
            let _ = res_tx.send(());
            true
        },
        Command::Continue(res_tx) => {
            let _ = res_tx.send(());
            false
        },
    }
}

/// Processes a single chain update.
fn process_chain_update(
    chain_update: cardano_chain_follower::ChainUpdate, module_state_key: &ModuleStateKey,
    chain_id: CardanoBlockchainId, event_subscriptions: &EventSubscriptions,
) -> anyhow::Result<u64> {
    let (block_data, immutable, rollback, tip, context) = match chain_update {
        ChainUpdate::ImmutableBlock(block_data) => {
            (
                block_data,
                true,
                false,
                false, // There are always live blocks in front of immutable ones.
                "Processing block chain update (Immutable)",
            )
        },
        ChainUpdate::ImmutableBlockRollback(block_data) => {
            (
                block_data,
                true,
                true,
                false, // There are always live blocks in front of immutable ones.
                "Processing block chain update (Immutable Rollback)",
            )
        },
        ChainUpdate::Block(block_data) => {
            (
                block_data,
                false,
                false,
                false,
                "Processing block chain update (Live Block)",
            )
        },
        ChainUpdate::BlockTip(block_data) => {
            (
                block_data,
                false,
                false,
                true,
                "Processing block chain update (Live Block @ Tip)",
            )
        },
        ChainUpdate::Rollback(rollback_data) => {
            (
                rollback_data,
                false,
                true,
                false, // By definition there are always blocks in front of a rollback.
                "Processing rollback chain update",
            )
        },
    };

    process_block_chain_update(
        module_state_key,
        chain_id,
        &block_data,
        event_subscriptions,
        immutable,
        rollback,
        tip,
    )
    .context(context)
}

/// Processes a block chain update.
///
/// This means decoding the block data, building and sending the event to the
/// Event Queue.
fn process_block_chain_update(
    module_state_key: &ModuleStateKey, chain_id: CardanoBlockchainId,
    block_data: &cardano_chain_follower::MultiEraBlockData,
    event_subscriptions: &EventSubscriptions, immutable: bool, rollback: bool, tip: bool,
) -> anyhow::Result<u64> {
    let decoded_block_data = block_data.decode();
    let block_number = decoded_block_data.number();

    // We send block data first.
    if event_subscriptions.blocks {
        build_and_send_block_event(
            module_state_key,
            chain_id,
            block_data,
            &decoded_block_data,
            immutable,
            rollback,
            tip,
        )
        .context("Sending Cardano block event to Event Queue")?;

        trace!(block_number, "Generated Cardano block event");
    }

    // TODO(SJ): Don't send transactions until the block has been fully processed.

    // Then if requested, the individual transactions.
    if event_subscriptions.txns {
        let txs = decoded_block_data.txs();

        build_and_send_txns_event(
            module_state_key,
            chain_id,
            &decoded_block_data,
            &txs,
            immutable,
            rollback,
            tip,
        )
        .context("Sending Cardano block transaction events to Event Queue")?;

        let tx_count = txs.len();
        trace!(
            block_number,
            tx_count,
            "Generated Cardano block transactions events"
        );
    }

    Ok(decoded_block_data.slot())
}

/// Get summary details about a particular block.
fn get_details(
    chain_id: CardanoBlockchainId, block_data: &MultiEraBlock, immutable: bool, rollback: bool,
    tip: bool,
) -> BlockDetail {
    let mut src = BlockSrc::empty();

    // Is the block Immutable or Live?
    if immutable {
        src |= BlockSrc::IMMUTABLE;
    };

    // Set the tip bit flag if at Tip of the chain.
    if tip {
        src |= BlockSrc::TIP;
    };

    // Set the rollback bit flag, if the block was from a rollback.
    if rollback {
        src |= BlockSrc::ROLLBACK;
    };

    let era = format!("{:?}", block_data.era());
    let height = block_data.number();
    let slot = block_data.slot();
    let hash = block_data.hash().to_vec();

    let wall_clock = match chain_id {
        CardanoBlockchainId::Mainnet => block_data.wallclock(&GenesisValues::mainnet()),
        CardanoBlockchainId::Preprod => block_data.wallclock(&GenesisValues::preprod()),
        CardanoBlockchainId::Preview => block_data.wallclock(&GenesisValues::preview()),
    };

    BlockDetail {
        era,
        src,
        height,
        slot: (slot, hash),
        wall_clock: Datetime {
            seconds: wall_clock,
            nanoseconds: 0,
        },
    }
}

/// Builds a [`super::event::OnCardanoBlockEvent`] from the block data and
/// sends it to the given module through the Event Queue.
fn build_and_send_block_event(
    module_state_key: &ModuleStateKey, chain_id: CardanoBlockchainId,
    block_data: &MultiEraBlockData, decoded_block: &MultiEraBlock, immutable: bool, rollback: bool,
    tip: bool,
) -> anyhow::Result<()> {
    let details = get_details(chain_id, decoded_block, immutable, rollback, tip);

    let on_block_event = super::event::OnCardanoBlockEvent {
        blockchain: chain_id,
        block: block_data.clone().into_raw_data(),
        details,
    };

    crate::event::queue::send(HermesEvent::new(
        on_block_event,
        TargetApp::List(vec![module_state_key.0.clone()]),
        TargetModule::List(vec![module_state_key.1.clone()]),
    ))
}

/// Builds [`super::event::OnCardanoTxnEvent`] for every transaction on the block data
/// and sends them to the given module through the Event Queue.
fn build_and_send_txns_event(
    module_state_key: &ModuleStateKey, chain_id: CardanoBlockchainId, block_data: &MultiEraBlock,
    txs: &[MultiEraTx], immutable: bool, rollback: bool, tip: bool,
) -> anyhow::Result<()> {
    let details = get_details(chain_id, block_data, immutable, rollback, tip);

    for (tx, index) in txs.iter().zip(0u32..) {
        let on_txn_event = super::event::OnCardanoTxnEvent {
            blockchain: chain_id,
            txn_index: index,
            txn: tx.encode(),
            details: details.clone(),
        };

        // Stop at the first error.
        crate::event::queue::send(HermesEvent::new(
            on_txn_event,
            TargetApp::List(vec![module_state_key.0.clone()]),
            TargetModule::List(vec![module_state_key.1.clone()]),
        ))?;
    }

    Ok(())
}

/// Gets the event subscription flags for a given module.
fn get_event_subscriptions(
    module_state_key: &ModuleStateKey,
) -> anyhow::Result<EventSubscriptions> {
    let sub_state = STATE
        .subscriptions
        .get(module_state_key)
        .ok_or(anyhow::anyhow!("Module subscription not found"))?;

    Ok(EventSubscriptions {
        blocks: sub_state.subscribed_to_blocks,
        txns: sub_state.subscribed_to_txns,
    })
}

/// Updates the module's state with the current slot the follower is at.
fn update_current_slot(module_state_key: &ModuleStateKey, current_slot: u64) -> anyhow::Result<()> {
    let mut sub_state = STATE
        .subscriptions
        .get_mut(module_state_key)
        .ok_or(anyhow::anyhow!("Module subscription not found"))?;

    sub_state.current_slot = current_slot;

    Ok(())
}
