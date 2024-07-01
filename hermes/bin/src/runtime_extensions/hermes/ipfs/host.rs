//! IPFS host implementation for WASM runtime.

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::hermes::ipfs::api::{
        DhtKey, DhtValue, Errno, Host, IpfsContent, IpfsPath, PubsubTopic,
    },
};

impl Host for HermesRuntimeContext {
    fn file_add(&mut self, _contents: IpfsContent) -> wasmtime::Result<Result<IpfsPath, Errno>> {
        todo!();
    }

    fn file_get(&mut self, _path: IpfsPath) -> wasmtime::Result<Result<IpfsContent, Errno>> {
        todo!();
    }

    fn file_pin(&mut self, ipfs_path: IpfsPath) -> wasmtime::Result<Result<bool, Errno>> {
        todo!();
    }

    fn dht_put(
        &mut self, _key: DhtKey, _contents: IpfsContent,
    ) -> wasmtime::Result<Result<bool, Errno>> {
        todo!();
    }

    fn dht_get(&mut self, _key: DhtKey) -> wasmtime::Result<Result<DhtValue, Errno>> {
        todo!();
    }

    fn pubsub_subscribe(&mut self, _topic: PubsubTopic) -> wasmtime::Result<Result<bool, Errno>> {
        todo!();
    }
}
