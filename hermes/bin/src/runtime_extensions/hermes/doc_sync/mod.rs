//! Doc Sync extension implementation.

use dashmap::DashMap;
use once_cell::sync::Lazy;

mod event;
mod host;

use std::borrow::Cow;

use anyhow::anyhow;
pub(crate) use event::OnNewDocEvent;

/// Convert IPFS topic string to Doc Sync channel name.
///
/// # Errors
///
/// - Topic is not valid doc sync channel.
fn map_ipfs_topic_to_channel_name(topic: &'_ str) -> anyhow::Result<Cow<'_, str>> {
    match topic {
        "doc-sync/documents" => Ok("documents".into()),
        _ => Err(anyhow!("Not a Doc Sync IPFS topic")),
    }
}

/// Convert IPFS topic string to Doc Sync channel name.
///
/// # Errors
///
/// - Topic is not valid doc sync channel.
fn map_channel_name_to_ipfs_topic(channel_name: &'_ str) -> anyhow::Result<Cow<'_, str>> {
    match channel_name {
        "documents" => Ok("doc-sync/documents".into()),
        _ => Err(anyhow!("Not a Doc Sync channel")),
    }
}

/// Initialize state. Which is mapping from String hash to String itself.
///
/// Note:
///
/// If large amount of sync channels is expected it would lead to great
/// amount of collision, so should be more strictly stored.
pub(super) type State = DashMap<u32, String>;

/// Global state to hold the resources.
static DOC_SYNC_STATE: Lazy<State> = Lazy::new(DashMap::new);
