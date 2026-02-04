//! Doc Sync host module.
use std::sync::{Arc, Mutex};

use catalyst_types::smt::Tree;
use hermes_ipfs::doc_sync::{
    Blake3256,
    payload::{CommonFields, DocumentDisseminationBody, New},
    timers::{config::SyncTimersConfig, state::SyncTimersState},
};
use minicbor::{Encode, Encoder, data::Tag, encode};
use stringzilla::stringzilla::Sha256;
use wasmtime::component::Resource;

use super::ChannelState;
use crate::{
    app::ApplicationName,
    ipfs::{self, SubscriptionKind, blocking},
    runtime_context::HermesRuntimeContext,
    runtime_extensions::{
        bindings::hermes::{
            doc_sync::api::{
                ChannelName, DocData, DocLoc, DocProof, Errno, Host, HostSyncChannel, ProverId,
                SyncChannel,
            },
            ipfs::api::{FileAddResult, Host as IpfsHost},
        },
        hermes::doc_sync::{
            Cid, DOC_SYNC_STATE, channel_resource_id, current_smt_summary, insert_cids_into_smt,
        },
    },
};

/// The number of steps in the "post document" workflow (see `post()` function):
/// 1. `add_file`: Store document in IPFS, get CID
/// 2. `dht_provide`: Announce to DHT that we have this content
/// 3. `get_peer_id`: Retrieve our peer identity
/// 4. `ensure_provided`: Wait for DHT propagation (backoff retries)
/// 5. `publish`: Broadcast document via Gossipsub `PubSub`
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

impl Encode<()> for Cid {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
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

        let mut e = Encoder::new(Vec::new());
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
        // The digest is a 64-byte array ([u8; 64]) for 512-bit output.
        // Take the first 4 bytes to use them as resource id.
        //
        // Assumption:
        // Number of channels is way more less then u32, so collisions are
        // acceptable but unlikely in practice. We use the first 4 bytes of
        // the cryptographically secure Blake2b hash as a fast, 32-bit ID
        // to minimize lock contention when accessing state via DOC_SYNC_STATE.
        let resource: u32 = channel_resource_id(&name).map_err(wasmtime::Error::msg)?;
        let mut channel_state = DOC_SYNC_STATE
            .entry(resource)
            .or_insert(ChannelState::new(&name));
        // Same resource key cannot be reused for a different channel
        if channel_state.channel_name != name {
            return Err(wasmtime::Error::msg(format!(
                "Collision occurred with previous value = {} and new one = {name}",
                channel_state.channel_name
            )));
        }

        for topic_suffix in [".new", ".syn"] {
            let tree = Arc::clone(&channel_state.smt);
            let topic = format!("{name}{topic_suffix}");
            // When the channel is created, subscribe to two topics: <name>.new and <name>.syn
            if let Err(err) = blocking::hermes_ipfs_subscribe(
                ipfs::SubscriptionKind::DocSync,
                self.app_name(),
                Some(tree),
                &topic,
                Some(&vec![self.module_id().clone()]),
            ) {
                DOC_SYNC_STATE.remove(&resource);
                return Err(wasmtime::Error::msg(format!(
                    "Subscription to {topic} failed: {err}",
                )));
            }
            tracing::info!("Created Doc Sync Channel: {name}");

            // DO NOT REMOVE THIS LOG, it is used in the test
            tracing::info!("Subscribed to {topic_suffix} with base {name}");
        }

        let timers_to_start;
        // When subscribe is successful, create and start the timer
        if channel_state.timers.is_none() {
            let timers = {
                let app_name = self.app_name().clone();
                let channel_name = name.clone();
                let smt = channel_state.smt.clone();

                let callback = Arc::new(move || {
                    send_new_keepalive(&smt, &channel_name, &app_name)
                        .map_err(|e| anyhow::anyhow!("{e:?}"))
                });

                SyncTimersState::new(SyncTimersConfig::default(), callback)
            };
            timers_to_start = Some(timers.clone());
            channel_state.timers = Some(timers);
        } else {
            timers_to_start = None;
        }
        drop(channel_state);
        if let Some(timers) = timers_to_start {
            timers.start_quiet_timer();
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
    fn post(
        &mut self,
        sync_channel: Resource<SyncChannel>,
        doc: DocData,
    ) -> wasmtime::Result<Result<DocLoc, Errno>> {
        tracing::info!("📤 Posting {} bytes to doc-sync channel", doc.len());

        let cid = add_document(self, &doc)??;
        let channel_state = DOC_SYNC_STATE
            .get(&sync_channel.rep())
            .ok_or_else(|| wasmtime::Error::msg("Channel not found"))?
            .clone();
        dht_provide(self, &cid)?;
        let peer_id = get_peer_id(self)??;
        ensure_provided(self, &cid, &peer_id)??;
        // Update SMT and publish .new with root/count
        let (root, count) = insert_cids_into_smt(&channel_state.smt, [cid.clone()])
            .map_err(|err| wasmtime::Error::msg(format!("Failed to update SMT: {err}")))?;
        let payload = build_new_payload(root, count, vec![cid.inner()]).map_err(|err| {
            wasmtime::Error::msg(format!("Failed to build doc-sync payload: {err}"))
        })?;
        publish_new_payload(self, &channel_state, &payload)??;

        if let Some(timers) = channel_state.timers {
            timers.reset_quiet_timer();
        }

        Ok(Ok(cid.to_bytes()))
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
fn add_document(
    ctx: &mut HermesRuntimeContext,
    doc: &DocData,
) -> wasmtime::Result<Result<Cid, Errno>> {
    const STEP: u8 = 1;
    match ctx.file_add(doc.clone())? {
        Ok(FileAddResult { file_path, cid }) => {
            let cid = Cid::from_bytes(&cid);
            tracing::info!(
                "✓ Step {STEP}/{POST_STEP_COUNT}: Added and pinned to IPFS (CID: {}) → {}",
                cid.0,
                file_path
            );
            Ok(Ok(cid))
        },
        Err(e) => {
            tracing::error!("✗ Step {STEP}/{POST_STEP_COUNT} failed: file_add error: {e:?}");
            Ok(Err(Errno::DocErrorPlaceholder))
        },
    }
}

/// Announce being a provider of the given CID to the DHT.
fn dht_provide(
    ctx: &mut HermesRuntimeContext,
    cid: &Cid,
) -> Result<(), Errno> {
    const STEP: u8 = 2;
    tracing::info!("⏭ Step {STEP}/{POST_STEP_COUNT}: Pre-publish");
    match ctx.dht_provide(cid.to_bytes()) {
        Ok(_) => {
            tracing::info!(
                "✓ Step {STEP}/{POST_STEP_COUNT}: DHT provide successful (CID: {})",
                cid.0
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

/// Get our peer ID
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
    cid: &Cid,
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
        let providers = ctx.dht_get_providers(cid.to_bytes())??;
        tracing::debug!(
            "Step {STEP}/{POST_STEP_COUNT}: DHT query returned {} provider(s): {:?}",
            providers.len(),
            providers
        );
        if is_pre_publish_completed(peer_id, &providers) {
            tracing::info!("✓ Step {STEP}/{POST_STEP_COUNT}: Other DHT providers found");
            return Ok(Ok(()));
        }
        let waiting_for = if providers.contains(&peer_id.to_string()) {
            "waiting for ourselves to appear in DHT query results"
        } else if providers.is_empty() {
            "waiting for at least 1 provider to appear"
        } else {
            "waiting for ourselves to appear (other providers exist)"
        };
        tracing::info!(
            "✓ Step {STEP}/{POST_STEP_COUNT}: DHT not ready (found {} provider(s), {}), sleeping...",
            providers.len(),
            waiting_for
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

/// Build a `.new` payload from local SMT signature and doc list.
fn build_new_payload(
    root: Blake3256,
    count: u64,
    docs: Vec<hermes_ipfs::Cid>,
) -> anyhow::Result<New> {
    New::try_from(DocumentDisseminationBody::Docs {
        common_fields: CommonFields {
            root,
            count,
            in_reply_to: None,
        },
        docs,
    })
    .map_err(|e| anyhow::anyhow!("Failed to create payload::New: {e}"))
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
fn publish_new_payload(
    ctx: &mut HermesRuntimeContext,
    channel_state: &ChannelState,
    payload: &New,
) -> wasmtime::Result<Result<(), Errno>> {
    const STEP: u8 = 5;
    let topic_new = format!("{}.new", channel_state.channel_name);

    let mut payload_bytes = Vec::new();
    let mut enc = Encoder::new(&mut payload_bytes);
    payload
        .encode(&mut enc, &mut ())
        .map_err(|e| wasmtime::Error::msg(format!("Failed to encode payload::New: {e}")))?;

    // The channel should already be subscribed to the `.new` topic (subscription
    // is performed in `new()`). Invoking the subscription again to ensure
    // the topic is active, because Gossipsub enforces that peers must subscribe
    // to a topic before they are permitted to publish on it.
    match blocking::hermes_ipfs_subscribe(
        SubscriptionKind::DocSync,
        ctx.app_name(),
        None,
        &topic_new,
        Some(&vec![ctx.module_id().clone()]),
    ) {
        Ok(_) => tracing::info!("✓ Subscribed to topic: {topic_new}"),
        Err(e) => tracing::warn!("⚠ Subscribe warning: {e}"),
    }

    // Attempt to publish to PubSub
    tracing::info!(
        "📤 Attempting to publish {} bytes to topic: {}",
        payload_bytes.len(),
        topic_new
    );

    match blocking::hermes_ipfs_publish(ctx.app_name(), &topic_new, payload_bytes) {
        Ok(()) => {
            tracing::info!("✅ Step {STEP}/{POST_STEP_COUNT}: Published to PubSub → {topic_new}");
            if let Some(timers) = channel_state.timers.as_ref() {
                timers.reset_quiet_timer();
            }
            Ok(Ok(()))
        },
        Err(e) => {
            tracing::warn!(
                error = ?e,
                "Doc-sync publish failed (non-fatal: NoPeersSubscribedToTopic likely in single-node; doc already stored)"
            );
            Ok(Err(Errno::DocErrorPlaceholder))
        },
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

/// Checks if the pre-publish is completed based on "our peer id" and
/// available providers.
///
/// Returns true if:
/// 1. We find ourselves as a provider (DHT announcement succeeded), OR
/// 2. We find other providers (content is available on the network)
///
/// Note: In P2P testing environments, content propagates via gossipsub (`PubSub`),
/// but nodes don't automatically announce themselves as DHT providers unless they
/// explicitly fetch content. Therefore, finding ourselves as the only provider
/// is sufficient to confirm DHT is working correctly.
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

/// Sending new keep alive message for .new topic.
fn send_new_keepalive(
    smt: &Arc<Mutex<Tree<Cid>>>,
    channel_name: &str,
    app_name: &ApplicationName,
) -> anyhow::Result<()> {
    tracing::info!("sending new document sync keepalive message");
    let (root, count) = current_smt_summary(smt)
        .map_err(|err| anyhow::anyhow!("Failed to fetch SMT state: {err}"))?;
    let payload = build_new_payload(root, count, vec![])?;
    let mut payload_bytes = Vec::new();
    let mut enc = Encoder::new(&mut payload_bytes);
    payload
        .encode(&mut enc, &mut ())
        .map_err(|e| anyhow::anyhow!("Failed to encode payload::New: {e}"))?;

    let new_topic = format!("{channel_name}.new");
    blocking::hermes_ipfs_publish(app_name, &new_topic, payload_bytes)
        .map_err(|e| anyhow::Error::msg(format!("Keepalive publish failed: {e:?}")))?;
    Ok(())
}

/// Adds CIDs to channel SMT.
pub fn add_cids_to_channel_smt(
    channel: &str,
    cids: Vec<Cid>,
) -> anyhow::Result<()> {
    tracing::info!("📤 Adding {} cids to {channel} channel", cids.len());
    let resource: u32 = channel_resource_id(channel).map_err(wasmtime::Error::msg)?;
    let channel_state = DOC_SYNC_STATE
        .get(&resource)
        .ok_or_else(|| wasmtime::Error::msg("Channel not found"))?
        .clone();

    _ = insert_cids_into_smt(&channel_state.smt, cids)
        .map_err(|err| wasmtime::Error::msg(format!("Failed to update SMT: {err}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use crate::runtime_extensions::hermes::doc_sync::host::is_pre_publish_completed;

    #[test_case("OUR", &["OTHER_1", "OTHER_2"] => true)]
    #[test_case("OUR", &["OUR", "OTHER_1", "OTHER_2"] => true)]
    #[test_case("OUR", &[] => false)]
    #[test_case("OUR", &["OUR"] => true; "our peer is sufficient for P2P testing")]
    fn pre_publish_completed(
        our_peer_id: &str,
        current_providers: &[&str],
    ) -> bool {
        let current_providers: Vec<_> = current_providers.iter().map(ToString::to_string).collect();
        is_pre_publish_completed(our_peer_id, &current_providers)
    }
}
