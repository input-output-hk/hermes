//! Hermes IPFS service.
//!
//! Handlers for IPFS topics.

use std::{pin::Pin, sync::Arc};

use hermes_ipfs::doc_sync::{
    payload::{self},
    syn_payload,
};

use super::HERMES_IPFS;
use crate::{
    ipfs::{
        SubscriptionKind, doc_sync::handle_doc_sync_topic,
        topic_message_context::TopicMessageContext,
    },
    runtime_extensions::{
        bindings::hermes::ipfs::api::PubsubMessage, hermes::ipfs::event::OnTopicEvent,
    },
};

/// A handler for messages from the IPFS pubsub topic
#[derive(Clone)]
pub(super) struct TopicMessageHandler {
    /// The topic.
    topic: String,

    /// The handler implementation.
    #[allow(
        clippy::type_complexity,
        reason = "to be revisited after the doc sync functionality is fully implemented as this type still evolves"
    )]
    callback: Arc<
        dyn Fn(
                hermes_ipfs::rust_ipfs::GossipsubMessage,
                String,
                TopicMessageContext, /* TODO[rafal-ch]: Should become a borrow, but if not
                                      * possible, at least an Arc */
            ) -> Pin<Box<dyn Future<Output = ()> + Send>>
            + Send
            + Sync
            + 'static,
    >,

    /// The context.
    context: TopicMessageContext,
}

impl TopicMessageHandler {
    /// Creates the new handler.
    pub fn new<F, Fut>(
        topic: &str,
        handler: F,
        context: TopicMessageContext,
    ) -> Self
    where
        F: Fn(hermes_ipfs::rust_ipfs::GossipsubMessage, String, TopicMessageContext) -> Fut
            + Send
            + Sync
            + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        Self {
            topic: topic.to_string(),
            callback: Arc::new(move |msg, topic, ctx| Box::pin(handler(msg, topic, ctx))),
            context,
        }
    }

    /// Forwards the message to the handler.
    pub async fn handle(
        &self,
        msg: hermes_ipfs::rust_ipfs::GossipsubMessage,
    ) {
        (self.callback)(msg, self.topic.clone(), self.context.clone()).await;
    }
}

/// A handler for subscribe/unsubscribe events from the IPFS pubsub topic
#[derive(Clone)]
pub(super) struct TopicSubscriptionStatusHandler<T>
where T: Fn(hermes_ipfs::SubscriptionStatusEvent, String) + Send + Sync + 'static
{
    /// The topic.
    topic: String,

    /// The handler implementation.
    callback: T,
}

impl<T> TopicSubscriptionStatusHandler<T>
where T: Fn(hermes_ipfs::SubscriptionStatusEvent, String) + Send + Sync + 'static
{
    /// Creates the new handler.
    pub fn new(
        topic: &impl ToString,
        callback: T,
    ) -> Self {
        Self {
            topic: topic.to_string(),
            callback,
        }
    }

    /// Passes the subscription event to the handler.
    pub fn handle(
        &self,
        subscription_event: hermes_ipfs::SubscriptionStatusEvent,
    ) {
        (self.callback)(subscription_event, self.topic.clone());
    }
}

/// Handler function for topic message streams.
pub(super) async fn topic_message_handler(
    message: hermes_ipfs::rust_ipfs::GossipsubMessage,
    topic: String,
    context: TopicMessageContext,
) {
    if let Some(ipfs) = HERMES_IPFS.get() {
        let app_names = ipfs.apps.subscribed_apps(SubscriptionKind::Default, &topic);

        drop(
            OnTopicEvent::new(PubsubMessage {
                topic,
                message: message.data.into(),
                publisher: message.source.map(|p| p.to_string()),
            })
            .build_and_send(app_names, context.module_ids()),
        );
    } else {
        tracing::error!("Failed to send on_topic_event. IPFS is uninitialized");
    }
}

async fn try_handlers(
    message: &hermes_ipfs::rust_ipfs::GossipsubMessage,
    topic: &str,
    context: &TopicMessageContext,
) -> Option<anyhow::Result<()>> {
    handle_doc_sync_topic::<payload::New>(message, topic, context.clone())
        .await
        .or(handle_doc_sync_topic::<syn_payload::MsgSyn>(message, topic, context.clone()).await)
}

/// Handler for Doc Sync `PubSub` messages.
#[allow(
    clippy::needless_pass_by_value,
    reason = "the other handler consumes the message and we need to keep the signatures consistent"
)]
pub(super) async fn doc_sync_topic_message_handler(
    message: hermes_ipfs::rust_ipfs::GossipsubMessage,
    topic: String,
    context: TopicMessageContext,
) {
    if let Ok(msg_str) = std::str::from_utf8(&message.data) {
        tracing::info!(
            "RECEIVED PubSub message on topic: {topic} - data: {}",
            &msg_str.chars().take(100).collect::<String>()
        );
    }

    let result = try_handlers(&message, &topic, &context).await;
    if let Some(Err(err)) = result {
        tracing::error!("Failed to handle IPFS message: {}", err);
    }
}
