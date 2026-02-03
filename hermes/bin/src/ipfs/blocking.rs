use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};

use catalyst_types::smt::Tree;
use tokio::{sync::oneshot, task::JoinHandle};

use super::task::IpfsCommand;
pub(crate) use super::task::SubscriptionKind;
use crate::{
    app::ApplicationName,
    ipfs::HermesIpfsNode,
    runtime_extensions::{
        bindings::hermes::ipfs::api::{Errno, MessageData, PeerId, PubsubTopic},
        hermes,
    },
    wasm::module::ModuleId,
};

impl<N> HermesIpfsNode<N>
where N: hermes_ipfs::rust_ipfs::NetworkBehaviour<ToSwarm = Infallible> + Send + Sync
{
    /// Get the peer identity in a non-async context.
    pub(super) fn get_peer_identity_blocking(
        &self,
        peer: Option<PeerId>,
    ) -> Result<Option<hermes_ipfs::PeerInfo>, Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.sender
            .as_ref()
            .ok_or(Errno::GetPeerIdError)?
            .blocking_send(IpfsCommand::Identity(peer, cmd_tx))
            .map_err(|_| Errno::GetPeerIdError)?;
        cmd_rx.blocking_recv().map_err(|_| Errno::GetPeerIdError)?
    }

    /// Publish message to a `PubSub` topic in the non-async context.
    pub(super) fn pubsub_publish_blocking(
        &self,
        topic: &PubsubTopic,
        message: MessageData,
    ) -> Result<(), Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.sender
            .as_ref()
            .ok_or(Errno::PubsubPublishError)?
            .blocking_send(IpfsCommand::Publish(topic.clone(), message, cmd_tx))
            .map_err(|_| Errno::PubsubPublishError)?;
        cmd_rx
            .blocking_recv()
            .map_err(|_| Errno::PubsubPublishError)?
    }

    /// Subscribe to a `PubSub` topic in a non-async context.
    pub(super) fn pubsub_subscribe_blocking(
        &self,
        kind: SubscriptionKind,
        topic: &PubsubTopic,
        tree: Option<Arc<Mutex<Tree<hermes::doc_sync::Cid>>>>,
        app_name: &ApplicationName,
        module_ids: Option<&Vec<ModuleId>>,
    ) -> Result<JoinHandle<()>, Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        let module_ids_owned = module_ids.cloned();
        self.sender
            .as_ref()
            .ok_or(Errno::PubsubSubscribeError)?
            .blocking_send(IpfsCommand::Subscribe(
                topic.clone(),
                kind,
                tree,
                app_name.clone(),
                module_ids_owned,
                cmd_tx,
            ))
            .map_err(|_| Errno::PubsubSubscribeError)?;
        cmd_rx
            .blocking_recv()
            .map_err(|_| Errno::PubsubSubscribeError)?
    }

    /// Unsubscribe from a `PubSub` topic in the non-async context
    pub(super) fn pubsub_unsubscribe_blocking(
        &self,
        topic: &PubsubTopic,
    ) -> Result<(), Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.sender
            .as_ref()
            .ok_or(Errno::PubsubUnsubscribeError)?
            .blocking_send(IpfsCommand::Unsubscribe(topic.clone(), cmd_tx))
            .map_err(|_| Errno::PubsubUnsubscribeError)?;
        cmd_rx
            .blocking_recv()
            .map_err(|_| Errno::PubsubUnsubscribeError)?
    }
}
