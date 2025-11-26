//! Doc Sync host module.

use cardano_chain_follower::pallas_codec::minicbor::{self, Encode, Encoder, data::Tag};
use stringzilla::stringzilla::Sha256;
use wasmtime::component::Resource;

use crate::{
    ipfs::hermes_ipfs_subscribe,
    runtime_context::HermesRuntimeContext,
    runtime_extensions::{
        bindings::hermes::doc_sync::api::{
            ChannelName, DocData, DocLoc, DocProof, Errno, Host, HostSyncChannel, ProverId,
            SyncChannel,
        },
        hermes::doc_sync::DOC_SYNC_STATE,
    },
};

/// CBOR multicodec identifier.
///
/// See: <https://github.com/multiformats/multicodec/blob/master/table.csv>
const CBOR_CODEC: u64 = 0x51;

/// SHA2-256 multihash code.
const SHA2_256_CODE: u64 = 0x12;

/// CBOR tag for IPLD CID (Content Identifier).
///
/// See: <https://github.com/ipld/cid-cbor/>
const CID_CBOR_TAG: u64 = 42;

/// Wrapper for `hermes_ipfs::Cid` to implement `minicbor::Encode` for it.
struct Cid(hermes_ipfs::Cid);

impl minicbor::Encode<()> for Cid {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        // Encode as tag(42) containing the CID bytes
        e.tag(Tag::new(CID_CBOR_TAG))?;
        e.bytes(&self.0.to_bytes())?;
        Ok(())
    }
}

#[allow(clippy::todo)]
impl Host for HermesRuntimeContext {
    /// Get the Document ID for the given Binary Document
    ///
    /// See: <https://docs.dev.projectcatalyst.io/hermes/main/architecture/08_concepts/document_sync/protocol_spec/#cidv1-binary-encoding-poc-focus>
    ///
    /// # Note
    ///
    /// We expect to receive doc as cbor bytes.
    fn id_for(
        &mut self,
        doc: DocData,
    ) -> wasmtime::Result<Vec<u8>> {
        // Compute SHA2-256 hash
        let mut hasher = Sha256::new();
        hasher.update(&doc);
        let hash_digest = hasher.digest();

        // Create multihash from digest using the wrap() API
        // The generic parameter <64> is the max digest size we support
        let multihash = multihash::Multihash::<64>::wrap(SHA2_256_CODE, &hash_digest)?;

        // Create CID v1 with CBOR codec
        let cid = hermes_ipfs::Cid::new_v1(CBOR_CODEC, multihash);

        let mut e = minicbor::Encoder::new(Vec::new());
        Cid(cid).encode(&mut e, &mut ())?;

        Ok(e.into_writer())
    }
}

#[allow(clippy::todo)]
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
        let hash = blake2b_simd::blake2b(name.as_bytes());

        // The digest is a 64-byte array ([u8; 64]) for 512-bit output.
        // Take the first 4 bytes to use them as resource id.
        //
        // Assumption:
        // Number of channels is way more less then u32, so collisions are
        // acceptable but unlikely in practice. We use the first 4 bytes of
        // the cryptographically secure Blake2b hash as a fast, 32-bit ID
        // to minimize lock contention when accessing state via DOC_SYNC_STATE.
        #[allow(clippy::indexing_slicing)]
        let prefix_bytes: [u8; 4] = hash.as_bytes()[..4].try_into().map_err(|err| {
            wasmtime::Error::msg(format!("error trimming channel name hash: {err}"))
        })?;

        let resource: u32 = u32::from_be_bytes(prefix_bytes);

        // Code block is used to minimize locking scope.
        {
            let entry = DOC_SYNC_STATE.entry(resource).or_insert(name.clone());
            if &name != entry.value() {
                return Err(wasmtime::Error::msg(format!(
                    "Collision occurred with previous value = {} and new one = {name}",
                    entry.value()
                )));
            }
        }

        if let Err(err) = hermes_ipfs_subscribe(self.app_name(), name) {
            DOC_SYNC_STATE.remove(&resource);
            return Err(wasmtime::Error::msg(format!("Subscription failed: {err}",)));
        }

        Ok(wasmtime::component::Resource::new_own(resource))
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
        self_: Resource<SyncChannel>,
    ) -> wasmtime::Result<Result<bool, Errno>> {
        inner_close(self, self_)
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
        _doc: DocData,
    ) -> wasmtime::Result<Result<DocLoc, Errno>> {
        todo!()
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
        todo!()
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
        todo!()
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
        todo!()
    }

    /// Wasmtime resource drop callback.
    fn drop(
        &mut self,
        res: Resource<SyncChannel>,
    ) -> wasmtime::Result<()> {
        inner_close(self, res)??;

        Ok(())
    }
}

/// This function is required cause reusage of `self.close`
/// inside drop causes invalid behavior during codegen.
#[allow(clippy::unnecessary_wraps)]
fn inner_close(
    _ctx: &mut HermesRuntimeContext,
    _res: Resource<SyncChannel>,
) -> wasmtime::Result<Result<bool, Errno>> {
    Ok(Ok(true))
}
