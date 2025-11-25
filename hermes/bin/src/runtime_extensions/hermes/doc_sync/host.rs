//! Doc Sync host module.

use wasmtime::component::Resource;

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::hermes::doc_sync::api::{
        ChannelName, DocData, DocLoc, DocProof, Errno, Host, HostSyncChannel, ProverId, SyncChannel,
    },
};

impl Host for HermesRuntimeContext {
    /// Get the Document ID for the given Binary Document
    fn id_for(
        &mut self,
        _doc: DocData,
    ) -> wasmtime::Result<Vec<u8>> {
        eprintln!("WARNING: doc_sync::id_for is not yet implemented (returning placeholder CID)");
        // Return a placeholder CIDv1 (same as used in the module implementation)
        Ok(
            "bafkreigh2akiscaildcqabsyg3dfr6chu3fgpregiymsck7e7aqa4s52zy"
                .as_bytes()
                .to_vec(),
        )
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
        name: ChannelName,
    ) -> wasmtime::Result<Resource<SyncChannel>> {
        eprintln!(
            "WARNING: doc_sync::SyncChannel::new('{}') is not yet implemented (returning stub resource)",
            name
        );
        // Create a stub resource with a dummy ID (42)
        // A real implementation would create and track actual channel state
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
        name: ChannelName,
    ) -> wasmtime::Result<Result<bool, Errno>> {
        eprintln!(
            "WARNING: doc_sync::SyncChannel::close('{}') is not yet implemented",
            name
        );
        Ok(Ok(true))
    }

    /// Post the document to a channel
    ///
    /// **Parameters**
    ///
    /// None
    ///
    /// **Returns**
    ///
    /// - `ok(true)`: Channel Closed and resources released.
    /// - `error(<something>)`: If it gets an error closing.
    fn post(
        &mut self,
        _self_: Resource<SyncChannel>,
        doc: DocData,
    ) -> wasmtime::Result<Result<DocLoc, Errno>> {
        eprintln!(
            "WARNING: doc_sync::SyncChannel::post() is not yet implemented (doc_size: {} bytes)",
            doc.len()
        );
        // Return a placeholder CIDv1 as the document location
        Ok(Ok(
            "bafkreigh2akiscaildcqabsyg3dfr6chu3fgpregiymsck7e7aqa4s52zy"
                .as_bytes()
                .to_vec(),
        ))
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
        eprintln!(
            "WARNING: doc_sync::SyncChannel::prove_includes() is not yet implemented (returning empty proofs)"
        );
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
        eprintln!(
            "WARNING: doc_sync::SyncChannel::prove_excludes() is not yet implemented (returning empty proofs)"
        );
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
        eprintln!("WARNING: doc_sync::SyncChannel::get() is not yet implemented (returning error)");
        Ok(Err(Errno::DocErrorPlaceholder))
    }

    /// Wasmtime resource drop callback.
    fn drop(
        &mut self,
        _rep: Resource<SyncChannel>,
    ) -> wasmtime::Result<()> {
        eprintln!("WARNING: doc_sync::SyncChannel::drop() is not yet implemented");
        // Nothing to clean up in stub implementation
        Ok(())
    }
}
