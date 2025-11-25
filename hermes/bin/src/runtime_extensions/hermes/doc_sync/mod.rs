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
fn map_ipfs_topic_to_channel_name(topic: &str) -> anyhow::Result<Cow<str>> {
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
fn map_channel_name_to_ipfs_topic(channel_name: &str) -> anyhow::Result<Cow<str>> {
    match channel_name {
        "documents" => todo!(),
        _ => Err(anyhow!("Not a Doc Sync channel")),
    }
}
