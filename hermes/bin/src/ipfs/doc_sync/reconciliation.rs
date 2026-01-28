use std::sync::{Arc, Mutex};

use catalyst_types::smt::Tree;
use hermes_ipfs::doc_sync::{Blake3256, PublicKey, syn_payload::MsgSyn};
use minicbor::{Encode, Encoder};

use crate::{
    app::ApplicationName,
    ipfs::{
        self, api::hermes_ipfs_unsubscribe, hermes_ipfs_get_peer_identity, hermes_ipfs_publish,
        hermes_ipfs_subscribe,
    },
    runtime_extensions::{
        bindings::hermes::ipfs::api::PeerId,
        hermes::doc_sync::{self},
    },
    wasm::module::ModuleId,
};

/// A result of deciding whether the document reconciliation process is needed
pub(crate) enum DocReconciliation {
    /// Reconciliation is not needed
    NotNeeded,
    /// Reconciliation is needed and we have a proper context here.
    Needed(DocReconciliationData),
}

/// The context for document reconciliation process, if it is needed.
pub(crate) struct DocReconciliationData {
    /// Root of our SMT.
    our_root: Blake3256,
    /// Document count in our SMT.
    our_count: u64,
    /// Root of the SMT on the peer
    their_root: Blake3256,
    /// Document count on the peer side.
    their_count: u64,
    /// A set of SMT prefixes at a coarse height
    prefixes: Vec<Option<Blake3256>>,
}

impl DocReconciliationData {
    pub(crate) fn new(
        our_root: Blake3256,
        our_count: u64,
        their_root: Blake3256,
        their_count: u64,
        prefixes: Vec<Option<Blake3256>>,
    ) -> Self {
        Self {
            our_root,
            our_count,
            their_root,
            their_count,
            prefixes,
        }
    }
}

/// Starts the document reconciliation process.
pub(super) fn start_reconciliation(
    doc_reconciliation_data: DocReconciliationData,
    app_name: &ApplicationName,
    tree: Arc<Mutex<Tree<doc_sync::Cid>>>,
    channel: &str,
    module_ids: Option<&Vec<ModuleId>>,
    peer: Option<PeerId>,
) -> anyhow::Result<()> {
    subscribe_to_dif(app_name, tree, channel, module_ids)?;
    tracing::info!(%channel, "subscribed to .dif");

    let syn_payload = make_syn_payload(doc_reconciliation_data, app_name, peer)?;
    tracing::info!("SYN payload created");

    if let Err(err) = send_syn_payload(&syn_payload, app_name, channel) {
        unsubscribe_from_dif(app_name, channel)?;
        tracing::info!(%channel, "unsubscribed from .dif");
        return Err(err);
    }
    tracing::info!("SYN payload sent");

    Ok(())
}

/// Subscribes to ".dif" topic in order to receive responses for the ".syn" requests.
fn subscribe_to_dif(
    app_name: &ApplicationName,
    tree: Arc<Mutex<Tree<doc_sync::Cid>>>,
    channel: &str,
    module_ids: Option<&Vec<ModuleId>>,
) -> anyhow::Result<()> {
    let topic = format!("{channel}.dif");
    hermes_ipfs_subscribe(
        ipfs::SubscriptionKind::DocSync,
        app_name,
        Some(tree),
        &topic,
        module_ids,
    )?;
    Ok(())
}

/// Unsubscribes from ".dif" topic.
fn unsubscribe_from_dif(
    app_name: &ApplicationName,
    channel: &str,
) -> anyhow::Result<()> {
    let topic = format!("{channel}.dif");
    hermes_ipfs_unsubscribe(ipfs::SubscriptionKind::DocSync, app_name, &topic)?;
    Ok(())
}

/// Creates the new SYN payload.
fn make_syn_payload(
    DocReconciliationData {
        our_root,
        our_count,
        prefixes,
        their_root,
        their_count,
    }: DocReconciliationData,
    app_name: &ApplicationName,
    peer: Option<PeerId>,
) -> anyhow::Result<MsgSyn> {
    let peer_info = hermes_ipfs_get_peer_identity(app_name, peer);
    let public_key = match peer_info {
        Ok(peer_info) => {
            let public_key = peer_info.public_key;
            let maybe_ed25519_public_key = public_key.try_into_ed25519();
            match maybe_ed25519_public_key {
                Ok(key) => {
                    let ed25519_public_key_bytes = key.to_bytes();
                    let ed25519_public_key_hermes = PublicKey::try_from(ed25519_public_key_bytes)?;
                    Some(ed25519_public_key_hermes)
                },
                Err(err) => {
                    tracing::info!(%err, "failed to convert key to ed25519, sending SYN request without explicit 'to' field");
                    None
                },
            }
        },
        Err(err) => {
            tracing::info!(%err, "failed to get peer identity, sending SYN request without explicit 'to' field");
            None
        },
    };

    Ok(MsgSyn {
        root: our_root,
        count: our_count,
        to: public_key,
        prefixes: (!prefixes.is_empty()).then_some(prefixes),
        peer_root: their_root,
        peer_count: their_count,
    })
}

/// Sends the SYN payload to request the reconciliation data.
fn send_syn_payload(
    payload: &MsgSyn,
    app_name: &ApplicationName,
    channel: &str,
) -> anyhow::Result<()> {
    let mut payload_bytes = Vec::new();
    let topic = format!("{channel}.syn");
    let mut enc = Encoder::new(&mut payload_bytes);
    payload
        .encode(&mut enc, &mut ())
        .map_err(|e| anyhow::anyhow!("Failed to encode syn_payload::MsgSyn: {e}"))?;
    hermes_ipfs_publish(app_name, &topic, payload_bytes)?;
    Ok(())
}
