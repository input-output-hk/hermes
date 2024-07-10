//! IPFS Task
use std::str::FromStr;

use hermes_ipfs::{
    pin_mut, AddIpfsFile, Cid, HermesIpfs, IpfsPath as PathIpfsFile, PeerId as TargetPeerId,
    PubsubMessageId, StreamExt,
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
            DhtKey, DhtValue, Errno, IpfsPath, MessageData, PeerId, PubsubMessage, PubsubTopic,
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
    /// Publish to a topic
    Publish(
        PubsubTopic,
        MessageData,
        oneshot::Sender<Result<PubsubMessageId, Errno>>,
    ),
    /// Subscribe to a topic
    Subscribe(PubsubTopic, oneshot::Sender<Result<JoinHandle<()>, Errno>>),
    /// Evict Peer from node
    EvictPeer(PeerId, oneshot::Sender<Result<bool, Errno>>),
}

/// IPFS asynchronous task
pub(crate) async fn ipfs_task(mut queue_rx: mpsc::Receiver<IpfsCommand>) -> anyhow::Result<()> {
    let hermes_node = HermesIpfs::start().await?;
    if let Some(ipfs_command) = queue_rx.recv().await {
        match ipfs_command {
            IpfsCommand::AddFile(ipfs_file, tx) => {
                let response = hermes_node
                    .add_ipfs_file(ipfs_file)
                    .await
                    .map(|ipfs_path| ipfs_path.to_string())
                    .map_err(|_| Errno::FileAddError);
                send_response(response, tx);
            },
            IpfsCommand::GetFile(ipfs_path, tx) => {
                let response = hermes_node
                    .get_ipfs_file(ipfs_path.into())
                    .await
                    .map_err(|_| Errno::FileGetError);
                send_response(response, tx);
            },
            IpfsCommand::PinFile(cid, tx) => {
                let response = match hermes_node.insert_pin(&cid).await {
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
                send_response(response, tx);
            },
            IpfsCommand::GetDhtValue(key, tx) => {
                let response = hermes_node
                    .dht_get(key)
                    .await
                    .map_err(|_| Errno::DhtGetError);
                send_response(response, tx);
            },
            IpfsCommand::PutDhtValue(key, value, tx) => {
                let response = hermes_node.dht_put(key, value).await.is_ok();
                send_response(Ok(response), tx);
            },
            IpfsCommand::Publish(topic, message, tx) => {
                let message_id = hermes_node
                    .pubsub_publish(topic, message)
                    .await
                    .map_err(|_| Errno::PubsubPublishError)?;
                send_response(Ok(message_id), tx);
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
                        // Dispatch Hermes Event
                        if let Err(err) = send(HermesEvent::new(
                            on_topic_event.clone(),
                            crate::event::TargetApp::List(app_names),
                            crate::event::TargetModule::All,
                        )) {
                            tracing::error!(on_topic_event = ?on_topic_event, "failed to send on_topic_event {err:?}");
                        }
                    }
                });
                send_response(Ok(handle), tx);
            },
            IpfsCommand::EvictPeer(peer, tx) => {
                let peer_id = TargetPeerId::from_str(&peer).map_err(|_| Errno::InvalidPeerId)?;
                let status = hermes_node.ban_peer(peer_id).await.is_ok();
                send_response(Ok(status), tx);
            },
        }
    }
    hermes_node.stop().await;
    Ok(())
}

/// Send the response of the IPFS command
fn send_response<T>(response: T, tx: oneshot::Sender<T>) {
    if tx.send(response).is_err() {
        tracing::error!("sending IPFS command response should not fail");
    }
}
