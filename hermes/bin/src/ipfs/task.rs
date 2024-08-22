//! IPFS Task
use std::str::FromStr;

use hermes_ipfs::{
    subscription_stream_task, AddIpfsFile, Cid, HermesIpfs, IpfsPath as PathIpfsFile,
    MessageId as PubsubMessageId, PeerId as TargetPeerId,
};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

use super::HERMES_IPFS;
use crate::{
    event::{queue::send, HermesEvent},
    runtime_extensions::{
        bindings::hermes::ipfs::api::{
            DhtKey, DhtValue, Errno, MessageData, PeerId, PubsubMessage, PubsubTopic,
        },
        hermes::ipfs::event::OnTopicEvent,
    },
};

/// IPFS Command
pub(crate) enum IpfsCommand {
    /// Add a new IPFS file
    AddFile(AddIpfsFile, oneshot::Sender<Result<PathIpfsFile, Errno>>),
    /// Get a file from IPFS
    GetFile(PathIpfsFile, oneshot::Sender<Result<Vec<u8>, Errno>>),
    /// Pin a file
    PinFile(Cid, oneshot::Sender<Result<bool, Errno>>),
    /// Un-pin a file
    UnPinFile(Cid, oneshot::Sender<Result<bool, Errno>>),
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

/// Handle IPFS commands in asynchronous task.
pub(crate) async fn ipfs_command_handler(
    hermes_node: HermesIpfs, mut queue_rx: mpsc::Receiver<IpfsCommand>,
) -> anyhow::Result<()> {
    while let Some(ipfs_command) = queue_rx.recv().await {
        match ipfs_command {
            IpfsCommand::AddFile(ipfs_file, tx) => {
                let response = hermes_node
                    .add_ipfs_file(ipfs_file)
                    .await
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
                        tracing::error!(cid = %cid, "failed to pin: {}", err);
                        Ok(false)
                    },
                };
                send_response(response, tx);
            },
            IpfsCommand::UnPinFile(cid, tx) => {
                let response = match hermes_node.remove_pin(&cid).await {
                    Ok(()) => Ok(true),
                    Err(err) => {
                        tracing::error!(cid = %cid, "failed to un-pin: {}", err);
                        Ok(false)
                    },
                };
                send_response(response, tx);
            },
            IpfsCommand::GetDhtValue(key, tx) => {
                let response = hermes_node.dht_get(key.clone()).await.map_err(|err| {
                    tracing::error!(dht_key = ?key, "failed to get DHT value: {}", err);
                    Errno::DhtGetError
                });
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
                let handle = subscription_stream_task(stream, topic_stream_app_handler);
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

/// Handler function for topic message streams.
fn topic_stream_app_handler(msg: hermes_ipfs::rust_ipfs::libp2p::gossipsub::Message) {
    if let Some(ipfs) = HERMES_IPFS.get() {
        let msg_topic = msg.topic.into_string();
        let on_topic_event = OnTopicEvent {
            message: PubsubMessage {
                topic: msg_topic.clone(),
                message: msg.data,
                publisher: msg.source.map(|p| p.to_string()),
            },
        };
        let app_names = ipfs.apps.subscribed_apps(&msg_topic);
        // Dispatch Hermes Event
        if let Err(err) = send(HermesEvent::new(
            on_topic_event.clone(),
            crate::event::TargetApp::List(app_names),
            crate::event::TargetModule::All,
        )) {
            tracing::error!(on_topic_event = ?on_topic_event, "failed to send on_topic_event {err:?}");
        }
    } else {
        tracing::error!("failed to send on_topic_event. IPFS is uninitialized");
    }
}

/// Send the response of the IPFS command
fn send_response<T>(response: T, tx: oneshot::Sender<T>) {
    if tx.send(response).is_err() {
        tracing::error!("sending IPFS command response should not fail");
    }
}
