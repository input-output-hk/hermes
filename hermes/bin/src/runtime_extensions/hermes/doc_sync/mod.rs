//! Doc Sync extension implementation.

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
        "" => Err(anyhow!("Not a Doc Sync IPFS topic")),
        _ => Ok("documents".into()),
    }
}

/// Convert IPFS topic string to Doc Sync channel name.
///
/// # Errors
///
/// - Topic is not valid doc sync channel.
fn map_channel_name_to_ipfs_topic(channel_name: &'_ str) -> anyhow::Result<Cow<'_, str>> {
    match channel_name {
        "documents" => Ok("documents".into()),
        _ => Err(anyhow!("Not a Doc Sync channel")),
    }
}
