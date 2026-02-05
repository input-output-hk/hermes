//! IPFS module related to the topic handling

use std::sync::Arc;

use hermes_ipfs::doc_sync::{
    payload::{self, CommonFields, DocumentDisseminationBody},
    syn_payload::{self, MsgSyn},
};

use crate::ipfs::{
    doc_sync::reconciliation, task::process_broadcasted_cids,
    topic_message_context::TopicMessageContext,
};

/// A helper trait to handle the IPFS messages of a specific topic.
pub(crate) trait TopicHandler: Sized {
    /// A suffix of the IPFS topic to which the handler is subscribed
    const TOPIC_SUFFIX: &'static str;

    /// Decodes the payload of the IPFS message.
    fn decode<'a>(payload: &'a [u8]) -> Result<Self, minicbor::decode::Error>
    where Self: minicbor::Decode<'a, ()> {
        minicbor::decode::<Self>(payload)
    }

    /// Handles the IPFS message.
    async fn handle(
        self,
        topic: &str,
        source: Option<hermes_ipfs::PeerId>,
        context: TopicMessageContext,
    ) -> anyhow::Result<()>;
}

impl TopicHandler for payload::New {
    const TOPIC_SUFFIX: &'static str = ".new";

    async fn handle(
        self,
        topic: &str,
        source: Option<hermes_ipfs::PeerId>,
        context: TopicMessageContext,
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
                                    Ok(())
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
                                    )
                                    .await
                                    {
                                        return Err(anyhow::anyhow!(
                                            "Failed to start reconciliation: {err}",
                                        ));
                                    }
                                    Ok(())
                                },
                            }
                        },
                        Err(err) => {
                            Err(anyhow::anyhow!(
                                "Failed to create reconciliation state: {err}",
                            ))
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
                        topic,
                        channel_name,
                        docs,
                        source,
                        context.module_ids(),
                    );
                    Ok(())
                }
            },
            DocumentDisseminationBody::Manifest { .. } => {
                Err(anyhow::anyhow!(
                    "Manifest is not supported in a .new payload",
                ))
            },
        }
    }
}

impl TopicHandler for syn_payload::MsgSyn {
    const TOPIC_SUFFIX: &'static str = ".syn";

    async fn handle(
        self,
        _topic: &str,
        _source: Option<hermes_ipfs::PeerId>,
        context: TopicMessageContext,
    ) -> anyhow::Result<()> {
        let Some(_tree) = context.tree() else {
            return Err(anyhow::anyhow!(
                "Context for payload::New handler must contain an SMT."
            ));
        };

        let msg: MsgSyn = MsgSyn::from(self);
        tracing::info!(root = %msg.root.to_hex(), count = %msg.count, "Received SYN message");
        Ok(())
    }
}

/// Handles the IPFS messages of a specific topic.
pub(crate) async fn handle_doc_sync_topic<'a, TH: TopicHandler + minicbor::Decode<'a, ()>>(
    message: &'a hermes_ipfs::rust_ipfs::GossipsubMessage,
    topic: &str,
    context: TopicMessageContext,
) -> Option<anyhow::Result<()>> {
    if !topic.ends_with(TH::TOPIC_SUFFIX) {
        return None;
    }

    let decoded = <TH as TopicHandler>::decode(&message.data);
    match decoded {
        Ok(handler) => Some(handler.handle(topic, message.source, context).await),
        Err(err) => {
            Some(Err(anyhow::anyhow!(
                "Failed to decode payload from IPFS message on topic {topic}: {err}"
            )))
        },
    }
}
