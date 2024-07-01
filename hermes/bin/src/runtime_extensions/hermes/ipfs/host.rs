//! IPFS host implementation for WASM runtime.

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::{
        bindings::hermes::ipfs::api::{
            DhtKey, DhtValue, Errno, Host, IpfsContent, IpfsPath, PeerId, PubsubTopic,
        },
        hermes::ipfs::state::{
            hermes_ipfs_add_file, hermes_ipfs_evict_peer, hermes_ipfs_get_dht_value,
            hermes_ipfs_get_file, hermes_ipfs_pin_file, hermes_ipfs_put_dht_value,
            hermes_ipfs_subscribe,
        },
    },
};

impl Host for HermesRuntimeContext {
    fn file_add(&mut self, contents: IpfsContent) -> wasmtime::Result<Result<IpfsPath, Errno>> {
        let path: IpfsPath = hermes_ipfs_add_file(contents)?.to_string();
        Ok(Ok(path))
    }

    fn file_get(&mut self, path: IpfsPath) -> wasmtime::Result<Result<IpfsContent, Errno>> {
        let contents = hermes_ipfs_get_file(path)?;
        Ok(Ok(contents))
    }

    fn file_pin(&mut self, ipfs_path: IpfsPath) -> wasmtime::Result<Result<bool, Errno>> {
        Ok(hermes_ipfs_pin_file(ipfs_path))
    }

    fn dht_put(
        &mut self, key: DhtKey, contents: IpfsContent,
    ) -> wasmtime::Result<Result<bool, Errno>> {
        Ok(hermes_ipfs_put_dht_value(key, contents))
    }

    fn dht_get(&mut self, key: DhtKey) -> wasmtime::Result<Result<DhtValue, Errno>> {
        Ok(hermes_ipfs_get_dht_value(key))
    }

    fn pubsub_subscribe(&mut self, topic: PubsubTopic) -> wasmtime::Result<Result<bool, Errno>> {
        Ok(hermes_ipfs_subscribe(topic))
    }

    fn peer_evict(&mut self, peer: PeerId) -> wasmtime::Result<Result<bool, Errno>> {
        Ok(hermes_ipfs_evict_peer(peer))
    }
}
