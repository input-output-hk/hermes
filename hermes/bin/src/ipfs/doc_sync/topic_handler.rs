use std::sync::Arc;

use hermes_ipfs::doc_sync::payload::{self, CommonFields, DocumentDisseminationBody};

use crate::ipfs::{
    task::{
        DocReconciliation, create_reconciliation_state, process_broadcasted_cids,
        start_reconciliation,
    },
    topic_message_context::TopicMessageContext,
};

pub(crate) trait DocSyncTopicHandler<'a>: Sized
where Self: minicbor::Decode<'a, ()>
{
    const TOPIC_SUFFIX: &'static str;

    fn decode(payload: &'a [u8]) -> Result<Self, minicbor::decode::Error> {
        minicbor::decode::<Self>(&payload)
    }

    fn handle(
        self,
        topic: &str,
        source: Option<hermes_ipfs::PeerId>,
        context: &TopicMessageContext,
    ) -> Result<(), ()>;
}

impl DocSyncTopicHandler<'_> for payload::New {
    const TOPIC_SUFFIX: &'static str = ".new";

    fn handle(
        self,
        topic: &str,
        source: Option<hermes_ipfs::PeerId>,
        context: &TopicMessageContext,
    ) -> Result<(), ()> {
        let Some(tree) = context.tree() else {
            tracing::error!("Context for payload::New handler must contain an SMT.");
            // TODO[RC]: Add error type and log errors in the upper layer.
            return Err(());
        };

        match DocumentDisseminationBody::from(self) {
            DocumentDisseminationBody::Docs {
                docs,
                common_fields:
                    CommonFields {
                        root: their_root,
                        count: their_count,
                        ..
                    },
            } => {
                // DO NOT remove this log, since it is used in tests
                tracing::info!("RECEIVED PubSub message with CIDs: {:?}", docs);

                if docs.is_empty() {
                    match create_reconciliation_state(their_root, their_count, tree.as_ref()) {
                        Ok(doc_reconciliation) => {
                            match doc_reconciliation {
                                DocReconciliation::NotNeeded => {
                                    tracing::info!("reconciliation not needed");
                                    return Ok(());
                                },
                                DocReconciliation::Needed(doc_reconciliation_data) => {
                                    tracing::info!("starting reconciliation");
                                    let Some(channel_name) = topic.strip_suffix(Self::TOPIC_SUFFIX)
                                    else {
                                        tracing::error!(%topic, "Wrong topic suffix, expected {}", Self::TOPIC_SUFFIX);
                                        return Err(());
                                    };
                                    if let Err(err) = start_reconciliation(
                                        doc_reconciliation_data,
                                        context.app_name(),
                                        Arc::clone(tree),
                                        channel_name,
                                        context.module_ids(),
                                        source.map(|p| p.to_string()),
                                    ) {
                                        tracing::error!(%err, "Failed to start reconciliation");
                                        return Err(());
                                    }
                                    return Ok(());
                                },
                            }
                        },
                        Err(err) => {
                            tracing::error!(%err, "Failed to create reconciliation state");
                            return Err(());
                        },
                    }
                } else {
                    let Some(channel_name) = topic.strip_suffix(Self::TOPIC_SUFFIX) else {
                        tracing::error!(%topic, "Wrong topic suffix, expected {}", Self::TOPIC_SUFFIX);
                        return Err(());
                    };
                    process_broadcasted_cids(
                        &topic,
                        channel_name,
                        docs,
                        source,
                        context.module_ids(),
                    );
                    return Ok(());
                }
            },
            DocumentDisseminationBody::Manifest { .. } => {
                tracing::error!("Manifest is not supported in a .new payload");
                return Err(());
            },
        }
    }
}

pub(crate) fn handle_doc_sync_topic<'a, TH: DocSyncTopicHandler<'a>>(
    message: &'a hermes_ipfs::rust_ipfs::GossipsubMessage,
    topic: String,
    context: TopicMessageContext,
) -> Result<(), ()> {
    if !topic.ends_with(TH::TOPIC_SUFFIX) {
        return Err(());
    }

    let decoded = <TH as DocSyncTopicHandler>::decode(&message.data);
    match decoded {
        Ok(handler) => handler.handle(&topic, message.source, &context),
        Err(err) => {
            tracing::error!(%topic, %err, topic_suffix = %TH::TOPIC_SUFFIX, "Failed to decode payload from IPFS message");
            return Err(());
        },
    }
}
