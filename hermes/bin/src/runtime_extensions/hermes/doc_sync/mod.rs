//! Doc Sync extension implementation.

use dashmap::DashMap;
use once_cell::sync::Lazy;

mod event;
mod host;

/// Initialize state. Which is mapping from String hash to String itself.
///
/// Note:
///
/// If large amount of sync channels is expected it would lead to great
/// amount of collision, so should be more strictly stored.
pub(super) type State = DashMap<u32, String>;

/// Global state to hold the resources.
static DOC_SYNC_STATE: Lazy<State> = Lazy::new(DashMap::new);
