//! Flag to control if chain sync for a blockchain is ready.
//! Can not consume the blockchain data until it is.

use std::time::Duration;

use crossbeam_skiplist::SkipMap;
use once_cell::sync::Lazy;
use strum::IntoEnumIterator;
use tokio::{
    sync::{broadcast, oneshot, RwLock},
    time::sleep,
};
use tracing::error;

use crate::{ChainUpdate, Network};

/// Data we hold related to sync being ready or not.
struct SyncReady {
    /// MPMC Receive queue for Blockchain Updates
    rx: broadcast::Receiver<ChainUpdate>,
    /// MPMC Transmit queue for Blockchain Updates
    tx: broadcast::Sender<ChainUpdate>,
    /// Sync is ready flag. (Prevents data race conditions)
    ready: bool,
}

impl SyncReady {
    /// Create a new `SyncReady` state.
    fn new() -> Self {
        // Can buffer up to 3 update messages before lagging.
        let (tx, rx) = broadcast::channel::<ChainUpdate>(3);
        Self {
            tx,
            rx,
            ready: false,
        }
    }
}

/// Waiter for sync to become ready, use `signal` when it is.
pub(crate) struct SyncReadyWaiter {
    /// The oneshot queue we use to signal ready.
    signal: Option<oneshot::Sender<()>>,
}

impl SyncReadyWaiter {
    /// Create a new `SyncReadyWaiter` state.
    pub(crate) fn signal(&mut self) {
        if let Some(signaler) = self.signal.take() {
            if let Err(error) = signaler.send(()) {
                error!("sync ready waiter signal should not fail: {error:?}");
            }
        } else {
            error!("sync ready waiter signal should not be called more than once.");
        }
    }
}

/// Lock to prevent using any blockchain data for a network UNTIL it is synced to TIP.
/// Pre-initialized for all possible blockchains, so it's safe to use `expect` to access a
/// value.
static SYNC_READY: Lazy<SkipMap<Network, RwLock<SyncReady>>> = Lazy::new(|| {
    let map = SkipMap::new();
    for network in Network::iter() {
        map.insert(network, RwLock::new(SyncReady::new()));
    }
    map
});

/// Write Lock the `SYNC_READY` lock for a network.
/// When we are signaled to be ready, set it to true and release the lock.
pub(crate) fn wait_for_sync_ready(chain: Network) -> SyncReadyWaiter {
    let (tx, rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        // We are safe to use `expect` here because the SYNC_READY list is exhaustively
        // initialized. Its a Serious BUG if that not True, so panic is OK.
        #[allow(clippy::expect_used)]
        let lock_entry = SYNC_READY.get(&chain).expect("network should exist");

        let lock = lock_entry.value();

        let mut status = lock.write().await;

        // If we successfully get told to unlock, we do.
        if let Ok(()) = rx.await {
            status.ready = true;
        }

        // If the channel closes early, we can NEVER use the Blockchain data.
    });

    SyncReadyWaiter { signal: Some(tx) }
}

/// Get a Read lock on the Sync State, and return if we are ready or not.
async fn check_sync_ready(chain: Network) -> bool {
    // We are safe to use `expect` here because the SYNC_READY list is exhaustively
    // initialized. Its a Serious BUG if that not True, so panic is OK.
    #[allow(clippy::expect_used)]
    let lock_entry = SYNC_READY.get(&chain).expect("network should exist");
    let lock = lock_entry.value();

    let status = lock.read().await;

    // If the transmitter has not been taken, we are not really ready.
    status.ready
}

/// Number of seconds to wait if we detect a `SyncReady` race condition.
const SYNC_READY_RACE_BACKOFF_SECS: u64 = 1;

/// Block until the chain is synced to TIP.
/// This is necessary to ensure the Blockchain data is fully intact before attempting to
/// consume it.
pub(crate) async fn block_until_sync_ready(chain: Network) {
    // There is a potential race where we haven't yet write locked the SYNC_READY lock when we
    // check it. So, IF the ready state returns as false, sleep a while and try again.
    while !check_sync_ready(chain).await {
        sleep(Duration::from_secs(SYNC_READY_RACE_BACKOFF_SECS)).await;
    }
}

/// Get the Broadcast Receive queue for the given chain updates.
pub(crate) async fn get_chain_update_rx_queue(chain: Network) -> broadcast::Receiver<ChainUpdate> {
    // We are safe to use `expect` here because the SYNC_READY list is exhaustively
    // initialized. Its a Serious BUG if that not True, so panic is OK.
    #[allow(clippy::expect_used)]
    let lock_entry = SYNC_READY.get(&chain).expect("network should exist");

    let lock = lock_entry.value();

    let status = lock.read().await;

    status.rx.resubscribe()
}

/// Get the Broadcast Transmit queue for the given chain updates.
pub(crate) async fn get_chain_update_tx_queue(
    chain: Network,
) -> Option<broadcast::Sender<ChainUpdate>> {
    // We are safe to use `expect` here because the SYNC_READY list is exhaustively
    // initialized. Its a Serious BUG if that not True, so panic is OK.
    #[allow(clippy::expect_used)]
    let lock_entry = SYNC_READY.get(&chain).expect("network should exist");

    let lock = lock_entry.value();

    if let Ok(status) = lock.try_read() {
        return Some(status.tx.clone());
    }

    None
}
