//! Doc Sync extension implementation.

use std::sync::Arc;

use dashmap::DashMap;
use once_cell::sync::Lazy;

use crate::runtime_extensions::hermes::doc_sync::timers::state::SyncTimersState;

mod event;
mod host;
mod timers;

/// In-memory representation for an opened doc-sync channel.
#[derive(Clone)]
pub(super) struct ChannelState {
    /// Channel topic (e.g. `documents`)
    pub topic: String,
    /// Timer state driving quiet-period keepalives.
    pub timers: Option<Arc<SyncTimersState>>,
}

impl ChannelState {
    /// Create a new state entry for the provided channel name.
    fn new(topic: String) -> Self {
        Self {
            topic,
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
