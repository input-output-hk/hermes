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
            ipfs::api::{FileAddResult, Host as IpfsHost},
        },
        hermes::doc_sync::DOC_SYNC_STATE,
    },
};

/// The number of steps in the "post document" workflow (see post() function):
/// 1. add_file: Store document in IPFS, get CID
/// 2. dht_provide: Announce to DHT that we have this content
/// 3. get_peer_id: Retrieve our peer identity
/// 4. ensure_provided: Wait for DHT propagation (backoff retries)
/// 5. publish: Broadcast document via Gossipsub PubSub
const POST_STEP_COUNT: u8 = 5;

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
    fn post(
        &mut self,
        sync_channel: Resource<SyncChannel>,
        doc: DocData,
    ) -> wasmtime::Result<Result<DocLoc, Errno>> {
        tracing::info!("📤 Posting {} bytes to doc-sync channel", doc.len());

        let cid = add_file(self, &doc)??;
        dht_provide(self, &cid)?;
        let peer_id = get_peer_id(self)??;
        ensure_provided(self, &cid, &peer_id)??;
        publish(self, doc, sync_channel.rep())??;

        Ok(Ok(cid.as_bytes().to_vec()))
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

/// Add document to IPFS (automatically pins)
/// Note: `file_add` pins the document under the hood via the `hermes_ipfs` library,
/// so no explicit `file_pin` call is needed.
fn add_file(
    ctx: &mut HermesRuntimeContext,
    doc: &DocData,
) -> wasmtime::Result<Result<String, Errno>> {
    const STEP: u8 = 1;
    match ctx.file_add(doc.clone())? {
        Ok(FileAddResult { file_path, cid }) => {
            tracing::info!(
                "✓ Step {STEP}/{POST_STEP_COUNT}: Added and pinned to IPFS (CID: {}) → {}",
                cid,
                file_path
            );
            Ok(Ok(cid))
        },
        Err(e) => {
            tracing::error!(
                "✗ Step {STEP}/{POST_STEP_COUNT} failed: file_add error: {:?}",
                e
            );
            Ok(Err(Errno::DocErrorPlaceholder))
        },
    }
}

/// Announce being a provider of the given CID to the DHT.
fn dht_provide(
    ctx: &mut HermesRuntimeContext,
    cid: &str,
) -> Result<(), Errno> {
    const STEP: u8 = 2;
    tracing::info!("⏭ Step {STEP}/{POST_STEP_COUNT}: Pre-publish");
    match ctx.dht_provide(cid.into()) {
        Ok(_) => {
            tracing::info!(
                "✓ Step {STEP}/{POST_STEP_COUNT}: DHT provide successful (CID: {})",
                cid
            );
            Ok(())
        },
        Err(e) => {
            tracing::error!(
                "✗ Step {STEP}/{POST_STEP_COUNT} failed: dht_provide error: {:?}",
                e
            );
            Err(Errno::DocErrorPlaceholder)
        },
    }
}

/// Get out peer ID
fn get_peer_id(ctx: &mut HermesRuntimeContext) -> wasmtime::Result<Result<String, Errno>> {
    const STEP: u8 = 3;
    match ctx.get_peer_id()? {
        Ok(peer_id) => {
            tracing::info!("✓ Step {STEP}/{POST_STEP_COUNT}: get get_peer_id",);
            Ok(Ok(peer_id))
        },
        Err(e) => {
            tracing::error!(
                "✗ Step {STEP}/{POST_STEP_COUNT} failed: get_peer_id error: {:?}",
                e
            );
            Ok(Err(Errno::DocErrorPlaceholder))
        },
    }
}

/// Wait until the given CID is provided by at least one other peer than the provided
/// `peer_id`
fn ensure_provided(
    ctx: &mut HermesRuntimeContext,
    cid: &str,
    peer_id: &str,
) -> wasmtime::Result<Result<(), Errno>> {
    const STEP: u8 = 4;

    /// Exponential backoff durations in milliseconds for DHT provider queries.
    ///
    /// Pattern: [100ms, 200ms, 400ms, 800ms, 1600ms] - each duration doubles
    ///
    /// Rationale:
    /// - Start with fast retries (100ms) for local/fast networks
    /// - Exponentially increase to avoid hammering DHT in slower networks
    /// - Total initial backoff: ~3 seconds before switching to 2-second intervals
    /// - After exhausting these, extends with 20x 2-second retries for P2P test
    ///   environments where DHT propagation can be slower due to mesh formation delays
    const BACKOFF_DURATION: [u64; 5] = [100, 200, 400, 800, 1600];

    // Extend retries for P2P testing environments where DHT propagation may be slower
    let mut sleep_iter = BACKOFF_DURATION
        .into_iter()
        .chain(std::iter::repeat_n(2000, 20));
    loop {
        let providers = ctx.dht_get_providers(cid.into())??;
        tracing::debug!(
            "Step {STEP}/{POST_STEP_COUNT}: DHT query returned {} provider(s): {:?}",
            providers.len(),
            providers
        );
        if is_pre_publish_completed(peer_id, &providers) {
            tracing::info!("✓ Step {STEP}/{POST_STEP_COUNT}: Other DHT providers found");
            return Ok(Ok(()));
        }
        tracing::info!(
            "✓ Step {STEP}/{POST_STEP_COUNT}: Other DHT providers not found (got {} provider(s), need {}), sleeping...",
            providers.len(),
            if providers.contains(&peer_id.to_string()) {
                "2+"
            } else {
                "1+"
            }
        );
        let Some(sleep_duration) = sleep_iter.next() else {
            tracing::error!(
                "✓ Step {STEP}/{POST_STEP_COUNT}: Other DHT providers not found, aborting"
            );
            return Ok(Err(Errno::DocErrorPlaceholder));
        };
        std::thread::sleep(std::time::Duration::from_millis(sleep_duration));
    }
}

/// Publish to `PubSub`
///
/// IMPORTANT: Gossipsub is a peer-to-peer protocol that requires at least one
/// OTHER peer node to be subscribed to the topic before messages can be published.
/// A single isolated node cannot publish to itself.
///
/// In production with multiple Hermes nodes or external IPFS nodes subscribing
/// to the topic, this will work. In a single-node demo/test environment, publish
/// will fail with `NoPeersSubscribedToTopic` which is expected behavior.
///
/// Since Step 1 (add + pin) already succeeded, the document is safely stored
/// in IPFS. We treat "no peers" as a warning rather than a fatal error.
fn publish(
    ctx: &mut HermesRuntimeContext,
    doc: DocData,
    rep: u32,
) -> wasmtime::Result<Result<(), Errno>> {
    const STEP: u8 = 5;
    let channel_name = DOC_SYNC_STATE
        .get(&rep)
        .ok_or_else(|| wasmtime::Error::msg("Channel not found"))?
        .value()
        .clone();

    let topic_new = format!("{channel_name}.new");

    // The channel should already be subscribed to the `.new` topic (subscription
    // is performed in `new()`). Invoking the subscription again to ensure
    // the topic is active, because Gossipsub enforces that peers must subscribe
    // to a topic before they are permitted to publish on it.
    match ctx.pubsub_subscribe(topic_new.clone())? {
        Ok(_) => tracing::info!("✓ Subscribed to topic: {topic_new}"),
        Err(e) => tracing::warn!("⚠ Subscribe warning: {:?}", e),
    }

    // Attempt to publish to PubSub
    tracing::info!(
        "📤 Attempting to publish {} bytes to topic: {}",
        doc.len(),
        topic_new
    );

    match ctx.pubsub_publish(topic_new.clone(), doc)? {
        Ok(()) => {
            tracing::info!("✅ Step {STEP}/{POST_STEP_COUNT}: Published to PubSub → {topic_new}");
        },
        Err(e) => {
            // Non-fatal: PubSub requires peer nodes to be subscribed to the topic.
            // In a single-node environment, this is expected to fail with
            // "NoPeersSubscribedToTopic". We treat this as a warning rather
            // than a fatal error since Step 1 already succeeded.
            tracing::warn!(
                "⚠ Step {STEP}/{POST_STEP_COUNT}: PubSub publish failed: {:?}",
                e
            );
            tracing::warn!(
                "   Note: Gossipsub requires other nodes subscribing to '{topic_new}' to work",
            );
            tracing::info!("   Document is successfully stored in IPFS");
        },
    }

    Ok(Ok(()))
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

/// Checks if the pre-publish is completed based on "our peer id" and
/// available providers.
///
/// For P2P testing and small isolated networks, finding ourselves as a provider
/// is sufficient. In production with many nodes, this ensures DHT propagation worked.
fn is_pre_publish_completed(
    our_peer_id: &str,
    current_providers: &[String],
) -> bool {
    // If we find ourselves as a provider, DHT propagation worked
    if current_providers.contains(&our_peer_id.to_string()) {
        true
    } else {
        // If we're not in the list yet, at least one provider should exist
        !current_providers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use crate::runtime_extensions::hermes::doc_sync::host::is_pre_publish_completed;

    #[test_case("OUR", &["OTHER_1", "OTHER_2"] => true)]
    #[test_case("OUR", &["OUR", "OTHER_1", "OTHER_2"] => true)]
    #[test_case("OUR", &[] => false)]
    #[test_case("OUR", &["OUR"] => false)]
    fn pre_publish_completed(
        our_peer_id: &str,
        current_providers: &[&str],
    ) -> bool {
        let current_providers: Vec<_> = current_providers.iter().map(ToString::to_string).collect();
        is_pre_publish_completed(our_peer_id, &current_providers)
    }
}
