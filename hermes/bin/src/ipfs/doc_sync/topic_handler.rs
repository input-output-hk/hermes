use std::sync::Arc;

use hermes_ipfs::doc_sync::payload::{self, CommonFields, DocumentDisseminationBody};

use crate::ipfs::{
    doc_sync::reconciliation, task::process_broadcasted_cids,
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
    ) -> anyhow::Result<()>;
}

impl DocSyncTopicHandler<'_> for payload::New {
    const TOPIC_SUFFIX: &'static str = ".new";

    fn handle(
        self,
        topic: &str,
        source: Option<hermes_ipfs::PeerId>,
        context: &TopicMessageContext,
    ) -> anyhow::Result<()> {
        let Some(tree) = context.tree() else {
            return Err(anyhow::anyhow!(
                "Context for payload::New handler must contain an SMT."
            ));
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
                    match reconciliation::create_reconciliation_state(
                        their_root,
                        their_count,
                        tree.as_ref(),
                    ) {
                        Ok(doc_reconciliation) => {
                            match doc_reconciliation {
                                reconciliation::DocReconciliation::NotNeeded => {
                                    tracing::info!("reconciliation not needed");
                                    return Ok(());
                                },
                                reconciliation::DocReconciliation::Needed(
                                    doc_reconciliation_data,
                                ) => {
                                    tracing::info!("starting reconciliation");
                                    let Some(channel_name) = topic.strip_suffix(Self::TOPIC_SUFFIX)
                                    else {
                                        return Err(anyhow::anyhow!(
                                            "Wrong topic, expected topic with suffix {}, but got {}",
                                            Self::TOPIC_SUFFIX,
                                            topic
                                        ));
                                    };
                                    if let Err(err) = reconciliation::start_reconciliation(
                                        doc_reconciliation_data,
                                        context.app_name(),
                                        Arc::clone(tree),
                                        channel_name,
                                        context.module_ids(),
                                        source.map(|p| p.to_string()),
                                    ) {
                                        return Err(anyhow::anyhow!(
                                            "Failed to start reconciliation: {err}",
                                        ));
                                    }
                                    return Ok(());
                                },
                            }
                        },
                        Err(err) => {
                            return Err(anyhow::anyhow!(
                                "Failed to create reconciliation state: {err}",
                            ));
                        },
                    }
                } else {
                    let Some(channel_name) = topic.strip_suffix(Self::TOPIC_SUFFIX) else {
                        return Err(anyhow::anyhow!(
                            "Wrong topic, expected topic with suffix {}, but got {}",
                            Self::TOPIC_SUFFIX,
                            topic,
                        ));
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
                return Err(anyhow::anyhow!(
                    "Manifest is not supported in a .new payload",
                ));
            },
        }
    }
}

pub(crate) fn handle_doc_sync_topic<'a, TH: DocSyncTopicHandler<'a>>(
    message: &'a hermes_ipfs::rust_ipfs::GossipsubMessage,
    topic: String,
    context: TopicMessageContext,
) -> Option<anyhow::Result<()>> {
    if !topic.ends_with(TH::TOPIC_SUFFIX) {
        return None;
    }

    let decoded = <TH as DocSyncTopicHandler>::decode(&message.data);
    match decoded {
        Ok(handler) => Some(handler.handle(&topic, message.source, &context)),
        Err(err) => {
            return Some(Err(anyhow::anyhow!(
                "Failed to decode payload from IPFS message on topic {}: {}",
                topic,
                err
            )));
        },
    }
}
