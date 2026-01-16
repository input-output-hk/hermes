//! Doc Sync extension implementation.

use std::sync::{Arc, Mutex};

use anyhow::Context;
use catalyst_types::smt::{Tree, Value};
use dashmap::DashMap;
use hermes_ipfs::doc_sync::{Blake3256, timers::state::SyncTimersState};
use once_cell::sync::Lazy;

mod event;
mod host;

pub(crate) use event::{OnNewDocEvent, ReadComponentInstanceExt};
pub use host::add_cids_to_channel_smt;

/// Wrapper for `hermes_ipfs::Cid` to implement `catalyst_types::smt::Value`.
#[derive(Clone, Debug, Default)]
pub struct Cid(pub hermes_ipfs::Cid);

impl Cid {
    /// Returns CID bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }

    /// Creates from raw bytes (defaults to empty CID on error).
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self(hermes_ipfs::Cid::try_from(bytes).unwrap_or_default())
    }

    /// Access inner CID.
    pub fn inner(&self) -> hermes_ipfs::Cid {
        self.0
    }
}

impl Value for Cid {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        Self::from_bytes(bytes)
    }
}

/// In-memory representation for an opened doc-sync channel.
#[derive(Clone)]
pub struct ChannelState {
    /// Name of the channel.
    pub channel_name: String,
    /// Timer state.
    pub timers: Option<Arc<SyncTimersState>>,
    /// Local SMT storing document CIDs.
    pub smt: Arc<Mutex<Tree<Cid>>>,
}

impl ChannelState {
    /// Create a new state entry for the provided channel name.
    pub fn new(channel_name: &str) -> Self {
        Self {
            channel_name: channel_name.to_string(),
            timers: None,
            smt: Arc::new(Mutex::new(Tree::new())),
        }
    }
}

/// Initialize state. Maps hashing prefix to channel metadata.
///
/// Note:
///
/// If large amount of sync channels is expected it would lead to great
/// amount of collision, so should be more strictly stored.
pub(super) type State = DashMap<u32, ChannelState>;

/// Global state to hold the resources.
static DOC_SYNC_STATE: Lazy<State> = Lazy::new(DashMap::new);

/// Compute a resource id from channel name using BLAKE2b/4 bytes.
///
/// The `BLAKE2b` digest is 64 bytes (512-bit); we take the first 4 bytes as a fast
/// 32-bit identifier. Number of channels is expected to be ≪ u32, so collisions
/// are unlikely in practice. This keeps the ID small to reduce contention when
/// accessing `DOC_SYNC_STATE`.
pub(super) fn channel_resource_id(name: &str) -> Result<u32, String> {
    blake2b_simd::Params::new()
        .hash_length(4)
        .hash(name.as_bytes())
        .as_bytes()
        .try_into()
        .map(u32::from_be_bytes)
        .map_err(|err| format!("BLAKE2b hash output length must be 4 bytes: {err}"))
}

/// Inserts a collection of CIDs into the Sparse Merkle Tree (SMT) and returns the updated
/// state.
///
/// This implementation follows the protocol's requirement for idempotent set inserts.
/// It is used when processing `.new` or `.dif` messages to update the local document set.
pub(super) fn insert_cids_into_smt(
    smt: &Arc<Mutex<Tree<Cid>>>,
    cids: impl IntoIterator<Item = Cid>,
) -> anyhow::Result<(Blake3256, u64)> {
    let mut guard = smt
        .lock()
        .map_err(|err| anyhow::anyhow!("failed to lock SMT: {err}"))?;

    for cid in cids {
        guard.insert(&cid).context("insert CID into SMT")?;
    }

    summary_from_tree(&guard)
}

/// Reads the current SMT summary (root hash and document count) without modification.
///
/// This "snapshot" is used to detect divergence against remote peers or to
/// generate quiet-period `.new` keepalive announcements.
pub(super) fn current_smt_summary(smt: &Arc<Mutex<Tree<Cid>>>) -> anyhow::Result<(Blake3256, u64)> {
    let guard = smt
        .lock()
        .map_err(|err| anyhow::anyhow!("failed to lock SMT: {err}"))?;
    summary_from_tree(&guard)
}

/// Internal helper to extract the BLAKE3-256 root and the u64 document count.
///
/// The root represents the cryptographic commitment to the set, while the count
/// is used to calculate the prefix depth for reconciliation.
fn summary_from_tree(tree: &Tree<Cid>) -> anyhow::Result<(Blake3256, u64)> {
    let root_bytes: [u8; 32] = tree
        .root()
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("SMT root should be 32 bytes"))?;
    let count = u64::try_from(tree.count()).context("number of leaves is too big")?;
    Ok((Blake3256::from(root_bytes), count))
}
