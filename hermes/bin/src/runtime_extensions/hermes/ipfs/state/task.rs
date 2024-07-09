//! IPFS Task
use std::str::FromStr;

use hermes_ipfs::{
    pin_mut, AddIpfsFile, Cid, HermesIpfs, IpfsPath as PathIpfsFile, PeerId as TargetPeerId,
    StreamExt,
};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

use super::HERMES_IPFS_STATE;
use crate::{
    event::{queue::send, HermesEvent},
    runtime_extensions::{
        bindings::hermes::ipfs::api::{
            DhtKey, DhtValue, Errno, IpfsPath, PeerId, PubsubMessage, PubsubTopic,
        },
        hermes::ipfs::event::OnTopicEvent,
    },
};

/// IPFS Command
pub(crate) enum IpfsCommand {
    /// Add a new IPFS file
    AddFile(AddIpfsFile, oneshot::Sender<Result<IpfsPath, Errno>>),
    /// Get a file from IPFS
    GetFile(PathIpfsFile, oneshot::Sender<Result<Vec<u8>, Errno>>),
    /// Pin a file
    PinFile(Cid, oneshot::Sender<Result<bool, Errno>>),
    /// Get DHT value
    GetDhtValue(DhtKey, oneshot::Sender<Result<DhtValue, Errno>>),
    /// Put DHT value
    PutDhtValue(DhtKey, DhtValue, oneshot::Sender<Result<bool, Errno>>),
    /// Subscribe to a topic
    Subscribe(PubsubTopic, oneshot::Sender<Result<JoinHandle<()>, Errno>>),
    /// Evict Peer from node
    EvictPeer(PeerId, oneshot::Sender<Result<bool, Errno>>),
}

#[allow(dead_code)]
/// IPFS
pub(crate) async fn ipfs_task(mut queue_rx: mpsc::Receiver<IpfsCommand>) -> anyhow::Result<()> {
    let hermes_node = HermesIpfs::start().await?;
    if let Some(ipfs_command) = queue_rx.recv().await {
        match ipfs_command {
            IpfsCommand::AddFile(ipfs_file, tx) => {
                let ipfs_path = hermes_node.add_ipfs_file(ipfs_file).await?;
                if let Err(_err) = tx.send(Ok(ipfs_path.to_string())) {
                    tracing::error!("Failed to send IPFS path");
                }
            },
            IpfsCommand::GetFile(ipfs_path, tx) => {
                let contents = hermes_node.get_ipfs_file(ipfs_path.into()).await?;
                if let Err(_err) = tx.send(Ok(contents)) {
                    tracing::error!("Failed to get IPFS contents");
                }
            },
            IpfsCommand::PinFile(cid, tx) => {
                let status = match hermes_node.insert_pin(&cid).await {
                    Ok(()) => Ok(true),
                    Err(err) if err.to_string().contains("already pinned recursively") => {
                        tracing::debug!(cid = %cid, "file already pinned");
                        Ok(true)
                    },
                    Err(err) => {
                        tracing::error!("Failed to pin block {}: {}", cid, err);
                        Err(Errno::FilePinError)
                    },
                };
                if let Err(err) = tx.send(status) {
                    tracing::error!("sending response of pin IPFS file should not fail: {err:?}");
                }
            },
            IpfsCommand::GetDhtValue(key, tx) => {
                let response = hermes_node
                    .dht_get(key)
                    .await
                    .map_err(|_| Errno::DhtGetError);
                if let Err(err) = tx.send(response) {
                    tracing::error!("sending DHT value should not fail: {err:?}");
                }
            },
            IpfsCommand::PutDhtValue(key, value, tx) => {
                let status = hermes_node.dht_put(key, value).await.is_ok();
                if let Err(err) = tx.send(Ok(status)) {
                    tracing::error!("sending status of DHT put should not fail: {err:?}");
                }
            },
            IpfsCommand::Subscribe(topic, tx) => {
                let stream = hermes_node
                    .pubsub_subscribe(topic)
                    .await
                    .map_err(|_| Errno::PubsubSubscribeError)?;
                let handle = tokio::spawn(async move {
                    pin_mut!(stream);
                    while let Some(msg) = stream.next().await {
                        let msg_topic = msg.topic.into_string();
                        let on_topic_event = OnTopicEvent {
                            message: PubsubMessage {
                                topic: msg_topic.clone(),
                                message: msg.data,
                                publisher: msg.source.map(|p| p.to_string()),
                            },
                        };
                        let app_names = HERMES_IPFS_STATE.apps.subscribed_apps(&msg_topic);
                        if let Err(err) = send(HermesEvent::new(
                            on_topic_event.clone(),
                            crate::event::TargetApp::List(app_names),
                            crate::event::TargetModule::All,
                        )) {
                            tracing::error!(on_topic_event = ?on_topic_event, "failed to send on_topic_event {err:?}");
                        }
                    }
                });
                if let Err(_err) = tx.send(Ok(handle)) {
                    tracing::error!("Failed to subscribe to topic");
                }
            },
            IpfsCommand::EvictPeer(peer, tx) => {
                let peer_id = TargetPeerId::from_str(&peer).map_err(|_| Errno::InvalidPeerId)?;
                let status = hermes_node.ban_peer(peer_id).await.is_ok();
                if let Err(err) = tx.send(Ok(status)) {
                    tracing::error!("sending status of peer eviction should not fail: {err:?}");
                }
            },
        }
    }
    hermes_node.stop().await;
    Ok(())
}
