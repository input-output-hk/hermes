//! Doc Sync host module.

use cardano_chain_follower::pallas_codec::minicbor::{self, Encode, Encoder, data::Tag};
use stringzilla::stringzilla::Sha256;
use wasmtime::component::Resource;

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::{
        bindings::hermes::{
            doc_sync::api::{
                ChannelName, DocData, DocLoc, DocProof, Errno, Host, HostSyncChannel, ProverId,
                SyncChannel,
            },
            ipfs::api::Host as IpfsHost,
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

impl HostSyncChannel for HermesRuntimeContext {
    /// Open Doc Sync Channel
    ///
    /// **Parameters**
    ///
    /// - `name`: The Name of the channel to Open.  Creates if it doesn't exist, otherwise
    ///   joins it.
    fn new(
        &mut self,
        name: ChannelName,
    ) -> wasmtime::Result<Resource<SyncChannel>> {
        let hash = blake2b_simd::Params::new()
            .hash_length(4)
            .hash(name.as_bytes());

        // The digest is a 64-byte array ([u8; 64]) for 512-bit output.
        // Take the first 4 bytes to use them as resource id.
        //
        // Assumption:
        // Number of channels is way more less then u32, so collisions are
        // acceptable but unlikely in practice. We use the first 4 bytes of
        // the cryptographically secure Blake2b hash as a fast, 32-bit ID
        // to minimize lock contention when accessing state via DOC_SYNC_STATE.
        let prefix_bytes: &[u8; 4] = hash.as_bytes().try_into().map_err(|err| {
            wasmtime::Error::msg(format!("BLAKE2b hash output length must be 4 bytes: {err}"))
        })?;

        let resource: u32 = u32::from_be_bytes(*prefix_bytes);

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

        // When the channel is created, subscribe to .new <base>.<topic>
        if let Err(err) = self.pubsub_subscribe(format!("{name}.new")) {
            // FIXME - Do we want to remove the entry from the map here?
            DOC_SYNC_STATE.remove(&resource);
            return Err(wasmtime::Error::msg(format!(
                "Subscription to {name}.new failed: {err}",
            )));
        }

        tracing::info!("Created Doc Sync Channel: {name}");

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
    /// Executes the 3-step workflow:
    /// 1. Add to IPFS (`file_add` - automatically pins)
    /// 2. Pre-publish (TODO #630)
    /// 3. Publish to `PubSub` (`pubsub_publish`)
    fn post(
        &mut self,
        self_: Resource<SyncChannel>,
        doc: DocData,
    ) -> wasmtime::Result<Result<DocLoc, Errno>> {
        tracing::info!("📤 Posting {} bytes to doc-sync channel", doc.len());

        // Step 1: Add document to IPFS (automatically pins)
        // Note: file_add pins the document under the hood via the hermes_ipfs library,
        // so no explicit file_pin call is needed.
        let ipfs_path = match self.file_add(doc.clone())? {
            Ok(path) => {
                tracing::info!("✓ Step 1/3: Added to IPFS (pinned) → {}", path);
                path
            },
            Err(e) => {
                tracing::error!("✗ Step 1/3 failed: file_add error: {:?}", e);
                return Ok(Err(Errno::DocErrorPlaceholder));
            },
        };

        // Step 2: Pre-publish validation (TODO #630)
        tracing::info!("⏭ Step 2/3: Pre-publish (skipped - TODO #630)");

        // Step 3: Publish to PubSub
        //
        // IMPORTANT: Gossipsub is a peer-to-peer protocol that requires at least one
        // OTHER peer node to be subscribed to the topic before messages can be published.
        // A single isolated node cannot publish to itself.
        //
        // In production with multiple Hermes nodes or external IPFS nodes subscribing
        // to the topic, this will work. In a single-node demo/test environment, publish
        // will fail with "NoPeersSubscribedToTopic" which is expected behavior.
        //
        // Since Step 1 (add + pin) already succeeded, the document is safely stored
        // in IPFS. We treat "no peers" as a warning rather than a fatal error.

        let channel_name = DOC_SYNC_STATE
            .get(&self_.rep())
            .ok_or_else(|| wasmtime::Error::msg("Channel not found"))?
            .value()
            .clone();

        let topic_new = format!("{channel_name}.new");

        // The channel should already be subscribed to the `.new` topic (subscription
        // is performed in `new()`). Invoking the subscription again to ensure
        // the topic is active, because Gossipsub enforces that peers must subscribe
        // to a topic before they are permitted to publish on it.
        match self.pubsub_subscribe(topic_new.clone())? {
            Ok(_) => tracing::info!("✓ Subscribed to topic: {topic_new}"),
            Err(e) => tracing::warn!("⚠ Subscribe warning: {:?}", e),
        }

        // Attempt to publish to PubSub
        if let Ok(()) = self.pubsub_publish(topic_new.clone(), doc)? {
            tracing::info!("✓ Step 3/3: Published to PubSub → {topic_new}",);
        } else {
            // Non-fatal: PubSub requires peer nodes to be subscribed to the topic.
            // In a single-node environment, this is expected to fail with
            // "NoPeersSubscribedToTopic". We treat this as a warning rather
            // than a fatal error since Step 1 already succeeded.
            tracing::warn!(
                "⚠ Step 3/3: PubSub publish skipped (no peer nodes subscribed to topic)"
            );
            tracing::warn!(
                "   Note: Gossipsub requires other nodes subscribing to '{topic_new}' to work",
            );
            tracing::info!("   Document is successfully stored in IPFS from Step 1");
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
        Ok(Ok(Vec::new()))
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
        Ok(Ok(Vec::new()))
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
    // TODO(anyone): Here we should clean up the state, since we would have a map that
    // associates app_name with app's subscriptions.
    Ok(Ok(true))
}
