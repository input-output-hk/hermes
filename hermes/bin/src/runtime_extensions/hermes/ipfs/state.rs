//! Hermes IPFS Internal State

use std::str::FromStr;
use dashmap::DashMap;
use hermes_ipfs::{AddIpfsFile, Cid, HermesIpfs, IpfsPath as PathIpfsFile, SubscriptionStream};
use once_cell::sync::Lazy;
use tokio::{
    runtime::Builder,
    sync::{mpsc, oneshot},
};

use crate::{
    app::HermesAppName,
    runtime_extensions::bindings::hermes::ipfs::api::{
        DhtKey, DhtValue, Errno, IpfsContent, IpfsPath, PubsubTopic,
    },
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
    HermesIpfsState {
        sender,
    }
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
pub(crate) fn hermes_ipfs_pin_file(cid: Cid) -> Result<bool, Errno> {
    HERMES_IPFS_STATE.file_pin(cid.to_string())
}

/// Get DHT Value
pub(crate) fn hermes_ipfs_get_dht_value(key: DhtKey) -> Result<DhtValue, Errno> {
    todo!();
}

/// Put DHT Value
pub(crate) fn hermes_ipfs_put_dht_value(key: DhtKey, value: DhtValue) -> bool {
    todo!();
}

/// Subscribe to a topic
pub(crate) fn hermes_ipfs_subscribe(topic: PubsubTopic) -> bool {
    todo!();
}

/// Hermes IPFS Internal State
struct HermesIpfsState {
    /// Send events to the IPFS node.
    sender: Option<mpsc::Sender<IpfsCommand>>,
}

impl HermesIpfsState {
    /// Add file
    fn file_add(&self, contents: IpfsContent) -> Result<IpfsPath, Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self
            .sender
            .as_ref()
            .ok_or(Errno::FileAddError)?
            .blocking_send(IpfsCommand::AddFile(AddIpfsFile::Stream((None, contents)), cmd_tx))
            .map_err(|_| Errno::FileAddError)?;
        cmd_rx.blocking_recv().map_err(|_| Errno::FileAddError)?
    }

    /// Get file
    #[allow(clippy::needless_pass_by_value)]
    fn file_get(&self, ipfs_path: IpfsPath) -> Result<IpfsContent, Errno> {
        let ipfs_path = PathIpfsFile::from_str(&ipfs_path).map_err(|_| Errno::InvalidIpfsPath)?;
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self
            .sender
            .as_ref()
            .ok_or(Errno::FileGetError)?
            .blocking_send(IpfsCommand::GetFile(ipfs_path, cmd_tx))
            .map_err(|_| Errno::FileGetError)?;
        cmd_rx.blocking_recv().map_err(|_| Errno::FileGetError)?
    }

    fn file_pin(&self, ipfs_path: IpfsPath) -> Result<bool, Errno> {
        let ipfs_path = PathIpfsFile::from_str(&ipfs_path).map_err(|_| Errno::InvalidIpfsPath)?;
        let cid = ipfs_path.root().cid().ok_or(Errno::InvalidCid)?;
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self
            .sender
            .as_ref()
            .ok_or(Errno::FilePinError)?
            .blocking_send(IpfsCommand::PinFile(*cid, cmd_tx))
            .map_err(|_| Errno::FilePinError)?;
        cmd_rx.blocking_recv().map_err(|_| Errno::FilePinError)
    }

    fn dht_put(&self, _key: DhtKey, _contents: IpfsContent) -> anyhow::Result<bool> {
        todo!();
    }

    fn dht_get(&self, _key: DhtKey) -> Result<DhtValue, Errno> {
        todo!();
    }

    fn pubsub_subscribe(&self, _topic: PubsubTopic) -> anyhow::Result<bool> {
        todo!();
    }
}

/// IPFS Command
enum IpfsCommand {
    /// Add a new IPFS file
    AddFile(AddIpfsFile, oneshot::Sender<Result<IpfsPath, Errno>>),
    /// Get a file from IPFS
    GetFile(PathIpfsFile, oneshot::Sender<Result<Vec<u8>, Errno>>),
    /// Pin a file
    PinFile(Cid, oneshot::Sender<bool>),
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
            }
            IpfsCommand::GetFile(ipfs_path, tx) => {
                let contents = hermes_node.get_ipfs_file(ipfs_path.into()).await?;
                if let Err(_err) = tx.send(Ok(contents)) {
                    tracing::error!("Failed to get IPFS contents");
                }
            }
            IpfsCommand::PinFile(cid, tx) => {
                let status = match hermes_node.insert_pin(&cid).await {
                    Ok(_) => true,
                    Err(err) => {
                        tracing::error!("Failed to pin block {}: {}", cid, err);
                        false
                    }
                };
                if let Err(_err) = tx.send(status) {
                    tracing::error!("Failed to pin IPFS file");
                }
            }
        }
    }
    hermes_node.stop().await;
    Ok(())
}
