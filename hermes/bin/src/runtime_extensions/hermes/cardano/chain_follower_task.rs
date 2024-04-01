//! A Chain Follower task is responsible for managing a Cardano Chain Follower
//! that is controlled by the Cardano Runtime Extension.

use tracing::{error, instrument, trace, warn};

use super::{Error, Result, STATE};
use crate::{
    app::HermesAppName,
    event::{HermesEvent, TargetApp, TargetModule},
    runtime_extensions::bindings::hermes::cardano::api::{BlockSrc, CardanoBlockchainId},
    wasm::module::ModuleId,
};

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
            .blocking_send(Command::SetReadPointer(at, res_tx))
            .map_err(|_| Error::InternalError)?;

        res_rx
            .blocking_recv()
            .map_err(|_| Error::InternalError)?
            .map_err(|_| Error::InternalError)
    }

    /// Sends a command to the chain follower executor task to stop following.
    /// The follower continues active and following can be resumed by calling
    /// [`Self::resume`].
    pub fn stop(&self) -> Result<()> {
        let (res_tx, res_rx) = tokio::sync::oneshot::channel();

        self.cmd_tx
            .blocking_send(Command::Stop(res_tx))
            .map_err(|_| Error::InternalError)?;

        drop(res_rx.blocking_recv());

        Ok(())
    }

    /// Sends a command to the chain follower executor task to resume following
    /// from the point it was previously stopped.
    ///
    /// This has no effect if the follower is not stopped.
    pub fn resume(&self) -> Result<()> {
        let (res_tx, res_rx) = tokio::sync::oneshot::channel();

        self.cmd_tx
            .blocking_send(Command::Continue(res_tx))
            .map_err(|_| Error::InternalError)?;

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

                match cmd {
                    Command::SetReadPointer(follow_from, res_tx) => {
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
                    Command::Stop(res_tx) => {
                        stopped = true;
                        let _ = res_tx.send(());
                    }
                    Command::Continue(res_tx) => {
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

                let current_slot = match chain_update {
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

                            for (tx, index) in txs.iter().zip(0u32..) {
                                let on_txn_event = super::event::OnCardanoTxnEvent {
                                    blockchain: chain_id,
                                    slot,
                                    txn_index: index,
                                    txn: tx.encode(),
                                };

                                let res = crate::event::queue::send(HermesEvent::new(
                                    on_txn_event,
                                    TargetApp::List(vec![module_state_key.0.clone()]),
                                    TargetModule::_List(vec![module_state_key.1.clone()]),
                                ));

                                if let Err(err) = res {
                                    error!(error = ?err, "Failed to send Cardano transaction event to the Event queue");
                                } else {
                                    trace!(block_number, tx_count = txs.len(), "Generated Cardano block transactions events");
                                }
                            }
                        }

                        if subscribed_to_blocks {
                            let on_block_event = super::event::OnCardanoBlockEvent {
                                blockchain: chain_id,
                                block: block_data.into_raw_data(),
                                // TODO(FelipeRosa): In order to implement this we need the
                                // cardano-chain-follower crate to give this information along
                                // with the chain update.
                                source: BlockSrc::NODE,
                            };

                            let res = crate::event::queue::send(HermesEvent::new(
                                on_block_event,
                                TargetApp::List(vec![module_state_key.0.clone()]),
                                TargetModule::_List(vec![module_state_key.1.clone()]),
                            ));

                            if let Err(err) = res {
                                error!(error = ?err, "Failed to send Cardano block event to the Event queue");
                            } else {
                                trace!(block_number, "Generated Cardano block event");
                            }
                        }

                        slot
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

                        let on_rollback_event = super::event::OnCardanoRollback {
                            blockchain: CardanoBlockchainId::Preprod,
                            slot,
                        };

                        let res = crate::event::queue::send(HermesEvent::new(
                            on_rollback_event,
                            TargetApp::List(vec![module_state_key.0.clone()]),
                            TargetModule::_List(vec![module_state_key.1.clone()]),
                        ));

                        if let Err(err) = res {
                            error!(error = ?err, "Failed to send Cardano block event to the Event queue");
                        } else {
                            trace!(block_number, "Generated Cardano rollback event");
                        }

                        slot
                    },
                };

                if let Some(mut sub_state) = STATE.subscriptions.get_mut(&module_state_key) {
                    sub_state.current_slot = current_slot;
                } else {
                    break 'exec_loop;
                };
            }
        }
    }

    // TODO(FelipeRosa): Stop waiting if this times out.
    drop(follower.close().await);
}
