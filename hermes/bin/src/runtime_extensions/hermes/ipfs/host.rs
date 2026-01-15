//! IPFS host implementation for WASM runtime.

use hermes_ipfs::Cid;

use crate::{
    ipfs::{
        self, hermes_ipfs_add_file, hermes_ipfs_content_validate, hermes_ipfs_dht_get_providers,
        hermes_ipfs_dht_provide, hermes_ipfs_evict_peer, hermes_ipfs_get_dht_value,
        hermes_ipfs_get_file, hermes_ipfs_get_peer_identity, hermes_ipfs_pin_file,
        hermes_ipfs_publish, hermes_ipfs_put_dht_value, hermes_ipfs_subscribe,
        hermes_ipfs_unpin_file,
    },
    runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::hermes::ipfs::api::{
        DhtKey, DhtValue, Errno, FileAddResult, Host, IpfsContent, IpfsFile, IpfsPath, MessageData,
        PeerId, PubsubTopic,
    },
};

impl From<hermes_ipfs::IpfsPath> for FileAddResult {
    fn from(value: hermes_ipfs::IpfsPath) -> Self {
        FileAddResult {
            file_path: value.to_string(),
            cid: value.root().cid().map(Cid::to_bytes).unwrap_or_default(),
        }
    }
}

impl Host for HermesRuntimeContext {
    fn file_add(
        &mut self,
        contents: IpfsFile,
    ) -> wasmtime::Result<Result<FileAddResult, Errno>> {
        let ipfs_path = hermes_ipfs_add_file(self.app_name(), contents)?;
        Ok(Ok(ipfs_path.into()))
    }

    fn file_get(
        &mut self,
        path: IpfsPath,
    ) -> wasmtime::Result<Result<IpfsFile, Errno>> {
        let contents = hermes_ipfs_get_file(self.app_name(), &path)?;
        Ok(Ok(contents))
    }

    fn file_pin(
        &mut self,
        ipfs_path: IpfsPath,
    ) -> wasmtime::Result<Result<bool, Errno>> {
        Ok(hermes_ipfs_pin_file(self.app_name(), &ipfs_path))
    }

    fn file_unpin(
        &mut self,
        ipfs_path: IpfsPath,
    ) -> wasmtime::Result<Result<bool, Errno>> {
        Ok(hermes_ipfs_unpin_file(self.app_name(), &ipfs_path))
    }

    fn dht_put(
        &mut self,
        key: DhtKey,
        value: DhtValue,
    ) -> wasmtime::Result<Result<bool, Errno>> {
        Ok(hermes_ipfs_put_dht_value(self.app_name(), key, value))
    }

    fn dht_get(
        &mut self,
        key: DhtKey,
    ) -> wasmtime::Result<Result<DhtValue, Errno>> {
        Ok(hermes_ipfs_get_dht_value(self.app_name(), key))
    }

    fn dht_provide(
        &mut self,
        key: DhtKey,
    ) -> wasmtime::Result<Result<(), Errno>> {
        Ok(hermes_ipfs_dht_provide(self.app_name(), key))
    }

    fn dht_get_providers(
        &mut self,
        key: DhtKey,
    ) -> wasmtime::Result<Result<Vec<PeerId>, Errno>> {
        Ok(hermes_ipfs_dht_get_providers(self.app_name(), key))
    }

    fn get_peer_id(&mut self) -> wasmtime::Result<Result<PeerId, Errno>> {
        let identity = hermes_ipfs_get_peer_identity(self.app_name())?;
        let peer_id = identity.peer_id;
        Ok(Ok(peer_id.to_string()))
    }

    fn pubsub_publish(
        &mut self,
        topic: PubsubTopic,
        message: MessageData,
    ) -> wasmtime::Result<Result<(), Errno>> {
        Ok(hermes_ipfs_publish(self.app_name(), &topic, message))
    }

    fn pubsub_subscribe(
        &mut self,
        topic: PubsubTopic,
    ) -> wasmtime::Result<Result<bool, Errno>> {
        Ok(hermes_ipfs_subscribe(
            ipfs::SubscriptionKind::Default,
            self.app_name(),
            &topic,
            None,
        ))
    }

    fn ipfs_content_validate(
        &mut self,
        content: IpfsContent,
    ) -> wasmtime::Result<Result<bool, Errno>> {
        Ok(Ok(hermes_ipfs_content_validate(self.app_name(), &content)))
    }

    fn peer_evict(
        &mut self,
        peer: PeerId,
    ) -> wasmtime::Result<Result<bool, Errno>> {
        Ok(hermes_ipfs_evict_peer(self.app_name(), peer))
    }
}
