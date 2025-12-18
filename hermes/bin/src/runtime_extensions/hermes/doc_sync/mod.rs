//! Doc Sync extension implementation.

use std::sync::Arc;

use dashmap::DashMap;
use hermes_ipfs::doc_sync::timers::state::SyncTimersState;
use once_cell::sync::Lazy;

mod event;
mod host;

pub(crate) use event::OnNewDocEvent;

pub(crate) use event::OnNewDocEvent;

/// In-memory representation for an opened doc-sync channel.
#[derive(Clone)]
pub(super) struct ChannelState {
    /// Name of the channel.
    pub channel_name: String,
    /// Timer state.
    pub timers: Option<Arc<SyncTimersState>>,
}

impl ChannelState {
    /// Create a new state entry for the provided channel name.
    fn new(channel_name: &str) -> Self {
        Self {
            channel_name: channel_name.to_string(),
            timers: None,
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
