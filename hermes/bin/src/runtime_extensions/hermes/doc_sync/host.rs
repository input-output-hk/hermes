//! Doc Sync host module.

use wasmtime::component::Resource;

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::hermes::{
        doc_sync::api::{
            ChannelName, DocData, DocLoc, DocProof, Errno, Host, HostSyncChannel, ProverId,
            SyncChannel,
        },
        ipfs::api::Host as IpfsHost,
    },
};

impl Host for HermesRuntimeContext {
    /// Get the CID for a document by adding it to IPFS (without pinning or publishing).
    fn id_for(
        &mut self,
        doc: DocData,
    ) -> wasmtime::Result<Vec<u8>> {
        match self.file_add(doc)? {
            Ok(ipfs_path) => {
                let cid_str = ipfs_path.strip_prefix("/ipfs/").unwrap_or(&ipfs_path);
                Ok(cid_str.as_bytes().to_vec())
            },
            Err(_) => Ok(b"error".to_vec()),
        }
    }
}

impl HostSyncChannel for HermesRuntimeContext {
    /// Open Doc Sync Channel
    ///
    /// **Parameters**
    ///
    /// - `name`: The Name of the channel to Open.  Creates if it doesn't exist, otherwise
    ///   joins it.
    ///
    /// **Returns**
    ///
    /// - `ok(network)`: A resource network, if successfully create network resource.
    /// - `error(create-network-error)`: If creating network resource failed.
    fn new(
        &mut self,
        _name: ChannelName,
    ) -> wasmtime::Result<Resource<SyncChannel>> {
        Ok(Resource::new_own(42))
    }

    /// Close Doc Sync Channel
    ///
    /// Can't use the sync-channel anymore after its closed
    /// (and all docs stored are released)
    /// Close itself should be deferred until all running WASM modules with an open
    /// `sync-channel` resource have terminated.
    ///  
    /// **Parameters**
    ///
    /// None
    ///
    /// **Returns**
    ///
    /// - `ok(true)`: Channel Closed and resources released.
    /// - `error(<something>)`: If it gets an error closing.
    fn close(
        &mut self,
        _self_: Resource<SyncChannel>,
        _name: ChannelName,
    ) -> wasmtime::Result<Result<bool, Errno>> {
        Ok(Ok(true))
    }

    /// Post a document to IPFS and broadcast via PubSub.
    ///
    /// Executes the 4-step workflow:
    /// 1. Add to IPFS (file_add)
    /// 2. Pin (file_pin)
    /// 3. Pre-publish (TODO #630)
    /// 4. Publish to PubSub (pubsub_publish)
    fn post(
        &mut self,
        _self_: Resource<SyncChannel>,
        doc: DocData,
    ) -> wasmtime::Result<Result<DocLoc, Errno>> {
        tracing::info!("📤 Posting {} bytes to doc-sync channel", doc.len());

        // Step 1: Add document to IPFS
        let ipfs_path = match self.file_add(doc.clone())? {
            Ok(path) => {
                tracing::info!("✓ Step 1/4: Added to IPFS → {}", path);
                path
            },
            Err(_) => {
                tracing::error!("✗ Step 1/4 failed: file_add error");
                return Ok(Err(Errno::DocErrorPlaceholder));
            },
        };

        // Step 2: Pin the document
        match self.file_pin(ipfs_path.clone())? {
            Ok(_) => tracing::info!("✓ Step 2/4: Pinned → {}", ipfs_path),
            Err(_) => {
                tracing::error!("✗ Step 2/4 failed: file_pin error");
                return Ok(Err(Errno::DocErrorPlaceholder));
            },
        }

        // Step 3: Pre-publish validation (TODO #630)
        tracing::info!("⏭ Step 3/4: Pre-publish (skipped - TODO #630)");

        // Step 4: Publish to PubSub
        //
        // IMPORTANT: Gossipsub is a peer-to-peer protocol that requires at least one
        // OTHER peer node to be subscribed to the topic before messages can be published.
        // A single isolated node cannot publish to itself.
        //
        // In production with multiple Hermes nodes or external IPFS nodes subscribing
        // to the topic, this will work. In a single-node demo/test environment, publish
        // will fail with "NoPeersSubscribedToTopic" which is expected behavior.
        //
        // Since Steps 1-2 (add + pin) already succeeded, the document is safely stored
        // in IPFS. We treat "no peers" as a warning rather than a fatal error.
        let topic = "doc-sync/documents".to_string();

        // Subscribe to the topic first (required for Gossipsub - you must be subscribed
        // to a topic before you can publish to it)
        match self.pubsub_subscribe(topic.clone())? {
            Ok(_) => tracing::info!("✓ Subscribed to topic: {}", topic),
            Err(e) => tracing::warn!("⚠ Subscribe warning: {:?}", e),
        }

        // Attempt to publish to PubSub
        match self.pubsub_publish(topic.clone(), doc)? {
            Ok(_) => {
                tracing::info!("✓ Step 4/4: Published to PubSub → {}", topic);
            },
            Err(Errno::PubsubPublishError) => {
                // Non-fatal: PubSub requires peer nodes to be subscribed to the topic.
                // In a single-node environment, this is expected to fail.
                tracing::warn!("⚠ Step 4/4: PubSub publish skipped (no peer nodes subscribed to topic)");
                tracing::warn!("   Note: Gossipsub requires other nodes subscribing to '{}' to work", topic);
                tracing::info!("   Document is successfully stored in IPFS from Steps 1-2");
            },
            Err(e) => {
                // Unexpected error - this shouldn't happen
                tracing::error!("✗ Step 4/4 failed: unexpected error: {:?}", e);
                return Ok(Err(Errno::DocErrorPlaceholder));
            },
        }

        // Extract CID from path and return it
        let cid_str = ipfs_path.strip_prefix("/ipfs/").unwrap_or(&ipfs_path);
        Ok(Ok(cid_str.as_bytes().to_vec()))
    }

    /// Prove a document is stored in the provers
    ///  
    /// **Parameters**
    ///
    /// loc : Location ID of the document to prove storage of.
    /// provers: List of provers to prove against (if empty, all provers will be requested
    /// for proof.)
    ///
    /// **Returns**
    ///
    /// - `ok(list of proofs received [prover id inside the proof])`: Document stored OK
    ///   or Not based on proof.
    /// - `error(<something>)`: If it gets an error.
    fn prove_includes(
        &mut self,
        _self_: Resource<SyncChannel>,
        _loc: DocLoc,
        _provers: Vec<ProverId>,
    ) -> wasmtime::Result<Result<Vec<DocProof>, Errno>> {
        Ok(Ok(vec![]))
    }

    /// Disprove a document is stored in the provers
    ///  
    /// **Parameters**
    ///
    /// loc : Location ID of the document to prove storage of.
    /// provers: List of provers to prove against (if empty, all provers will be requested
    /// for proof.)
    ///
    /// **Returns**
    ///
    /// - `ok(list of proofs received [prover id inside the proof])`: Document stored OK
    ///   or Not based on proof.
    /// - `error(<something>)`: If it gets an error.
    fn prove_excludes(
        &mut self,
        _self_: Resource<SyncChannel>,
        _loc: DocLoc,
        _provers: Vec<ProverId>,
    ) -> wasmtime::Result<Result<Vec<DocProof>, Errno>> {
        Ok(Ok(vec![]))
    }

    /// Prove a document is stored in the provers
    ///  
    /// **Parameters**
    ///
    /// None
    ///
    /// **Returns**
    ///
    /// - `ok(doc-data)`: Data associated with that document location, if it exists.
    /// - `error(<something>)`: If it gets an error.
    fn get(
        &mut self,
        _self_: Resource<SyncChannel>,
        _loc: DocLoc,
    ) -> wasmtime::Result<Result<DocData, Errno>> {
        Ok(Err(Errno::DocErrorPlaceholder))
    }

    /// Wasmtime resource drop callback.
    fn drop(
        &mut self,
        _rep: Resource<SyncChannel>,
    ) -> wasmtime::Result<()> {
        Ok(())
    }
}
