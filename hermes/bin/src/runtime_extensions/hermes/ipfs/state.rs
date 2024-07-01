//! Hermes IPFS Internal State

use std::str::FromStr;

use hermes_ipfs::{
    AddIpfsFile, Cid, HermesIpfs, IpfsPath as PathIpfsFile, PeerId as TargetPeerId,
    SubscriptionStream,
};
use once_cell::sync::Lazy;
use tokio::{
    runtime::Builder,
    sync::{mpsc, oneshot},
};

use crate::runtime_extensions::bindings::hermes::ipfs::api::{
    DhtKey, DhtValue, Errno, IpfsContent, IpfsPath, PeerId, PubsubTopic,
};

/// Hermes IPFS Internal State
static HERMES_IPFS_STATE: Lazy<HermesIpfsState> = Lazy::new(|| {
    let sender = if let Ok(runtime) = Builder::new_current_thread().enable_all().build() {
        let (sender, receiver) = mpsc::channel(1);
        let _handle = std::thread::spawn(move || {
            runtime.block_on(async move {
                let h = tokio::spawn(ipfs_task(receiver));
                drop(tokio::join!(h));
            });
            std::process::exit(0);
        });
        Some(sender)
    } else {
        // Failed to start the IPFS task
        tracing::error!("Failed to start the IPFS task");
        None
    };
    HermesIpfsState::new(sender)
});

/// Add File to IPFS
pub(crate) fn hermes_ipfs_add_file(contents: IpfsContent) -> Result<IpfsPath, Errno> {
    HERMES_IPFS_STATE.file_add(contents)
}

/// Get File from Ipfs
pub(crate) fn hermes_ipfs_get_file(path: IpfsPath) -> Result<IpfsContent, Errno> {
    HERMES_IPFS_STATE.file_get(path)
}

/// Pin IPFS File
pub(crate) fn hermes_ipfs_pin_file(path: IpfsPath) -> Result<bool, Errno> {
    HERMES_IPFS_STATE.file_pin(path)
}

/// Get DHT Value
pub(crate) fn hermes_ipfs_get_dht_value(key: DhtKey) -> Result<DhtValue, Errno> {
    HERMES_IPFS_STATE.dht_get(key)
}

/// Put DHT Value
pub(crate) fn hermes_ipfs_put_dht_value(key: DhtKey, value: DhtValue) -> Result<bool, Errno> {
    HERMES_IPFS_STATE.dht_put(key, value)
}

/// Subscribe to a topic
pub(crate) fn hermes_ipfs_subscribe(topic: PubsubTopic) -> Result<bool, Errno> {
    let _stream = HERMES_IPFS_STATE.pubsub_subscribe(topic)?;
    Ok(true)
}

/// Evict Peer from node
pub(crate) fn hermes_ipfs_evict_peer(peer: PeerId) -> Result<bool, Errno> {
    HERMES_IPFS_STATE.peer_evict(peer)
}

/// Hermes IPFS Internal State
struct HermesIpfsState {
    /// State related to `HermesAppName`
    apps: AppIpfsState,
}

impl HermesIpfsState {
    /// Create a new `HermesIpfsState`
    fn new(sender: Option<mpsc::Sender<IpfsCommand>>) -> Self {
        Self {
            apps: AppIpfsState { sender },
        }
    }

    /// Add file
    fn file_add(&self, contents: IpfsContent) -> Result<IpfsPath, Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.apps
            .sender
            .as_ref()
            .ok_or(Errno::FileAddError)?
            .blocking_send(IpfsCommand::AddFile(
                AddIpfsFile::Stream((None, contents)),
                cmd_tx,
            ))
            .map_err(|_| Errno::FileAddError)?;
        cmd_rx.blocking_recv().map_err(|_| Errno::FileAddError)?
    }

    /// Get file
    #[allow(clippy::needless_pass_by_value)]
    fn file_get(&self, ipfs_path: IpfsPath) -> Result<IpfsContent, Errno> {
        let ipfs_path = PathIpfsFile::from_str(&ipfs_path).map_err(|_| Errno::InvalidIpfsPath)?;
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.apps
            .sender
            .as_ref()
            .ok_or(Errno::FileGetError)?
            .blocking_send(IpfsCommand::GetFile(ipfs_path, cmd_tx))
            .map_err(|_| Errno::FileGetError)?;
        cmd_rx.blocking_recv().map_err(|_| Errno::FileGetError)?
    }

    /// Pin file
    #[allow(clippy::needless_pass_by_value)]
    fn file_pin(&self, ipfs_path: IpfsPath) -> Result<bool, Errno> {
        let ipfs_path = PathIpfsFile::from_str(&ipfs_path).map_err(|_| Errno::InvalidIpfsPath)?;
        let cid = ipfs_path.root().cid().ok_or(Errno::InvalidCid)?;
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.apps
            .sender
            .as_ref()
            .ok_or(Errno::FilePinError)?
            .blocking_send(IpfsCommand::PinFile(*cid, cmd_tx))
            .map_err(|_| Errno::FilePinError)?;
        cmd_rx.blocking_recv().map_err(|_| Errno::FilePinError)?
    }

    /// Put DHT Key-Value
    fn dht_put(&self, key: DhtKey, contents: IpfsContent) -> Result<bool, Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.apps
            .sender
            .as_ref()
            .ok_or(Errno::DhtPutError)?
            .blocking_send(IpfsCommand::PutDhtValue(key, contents, cmd_tx))
            .map_err(|_| Errno::DhtPutError)?;
        cmd_rx.blocking_recv().map_err(|_| Errno::DhtPutError)?
    }

    /// Get DHT Value by Key
    fn dht_get(&self, key: DhtKey) -> Result<DhtValue, Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.apps
            .sender
            .as_ref()
            .ok_or(Errno::DhtGetError)?
            .blocking_send(IpfsCommand::GetDhtValue(key, cmd_tx))
            .map_err(|_| Errno::DhtGetError)?;
        cmd_rx.blocking_recv().map_err(|_| Errno::DhtGetError)?
    }

    #[allow(clippy::needless_pass_by_value)]
    /// Subscribe to a `PubSub` topic
    fn pubsub_subscribe(&self, topic: PubsubTopic) -> Result<SubscriptionStream, Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.apps
            .sender
            .as_ref()
            .ok_or(Errno::PubsubSubscribeError)?
            .blocking_send(IpfsCommand::Subscribe(topic, cmd_tx))
            .map_err(|_| Errno::PubsubSubscribeError)?;
        cmd_rx
            .blocking_recv()
            .map_err(|_| Errno::PubsubSubscribeError)?
    }

    #[allow(clippy::needless_pass_by_value)]
    /// Evict peer
    fn peer_evict(&self, peer: PeerId) -> Result<bool, Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.apps
            .sender
            .as_ref()
            .ok_or(Errno::PeerEvictionError)?
            .blocking_send(IpfsCommand::EvictPeer(peer, cmd_tx))
            .map_err(|_| Errno::PeerEvictionError)?;
        cmd_rx
            .blocking_recv()
            .map_err(|_| Errno::PeerEvictionError)?
    }
}

/// IPFS app state
struct AppIpfsState {
    /// Send events to the IPFS node.
    sender: Option<mpsc::Sender<IpfsCommand>>,
}

/// IPFS Command
enum IpfsCommand {
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
    Subscribe(
        PubsubTopic,
        oneshot::Sender<Result<SubscriptionStream, Errno>>,
    ),
    /// Evict Peer from node
    EvictPeer(PeerId, oneshot::Sender<Result<bool, Errno>>),
}

#[allow(dead_code)]
/// IPFS
async fn ipfs_task(mut queue_rx: mpsc::Receiver<IpfsCommand>) -> anyhow::Result<()> {
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
                    Ok(()) => true,
                    Err(err) => {
                        tracing::error!("Failed to pin block {}: {}", cid, err);
                        false
                    },
                };
                if let Err(err) = tx.send(Ok(status)) {
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
                let status = hermes_node
                    .pubsub_subscribe(topic)
                    .await
                    .map_err(|_| Errno::PubsubSubscribeError);
                if let Err(_err) = tx.send(status) {
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
