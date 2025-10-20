//! HTTP Gateway

use gateway_task::spawn;
use serde::Deserialize;
use subscription::{register_global_endpoint_subscription, EndpointSubscription};
use tracing::{error, info};

mod event;
mod gateway_task;
mod host;
/// Gateway routing logic
mod routing;
/// Subscription management for targeted routing
mod subscription;

/// endpoint sub
#[derive(Debug, Deserialize)]
struct EndpointSubscriptionConfig {
    /// Unique identifier for the subscription
    subscription_id: String,
    /// Module that will handle this endpoint
    module_id: String,
    /// HTTP methods this endpoint accepts (GET, POST, etc.)
    methods: Vec<String>,
    /// Regular expression pattern to match request paths
    path_regex: String,
    /// Content types this endpoint accepts
    content_types: Vec<String>,
    /// Optional JSON schema for request validation
    json_schema: Option<String>,
}

// Embed config at compile time
const EMBEDDED_CONFIG: &str = include_str!("config/endpoints.json");

/// Load endpoint configurations from embedded config
pub(crate) async fn load_embedded_endpoints() -> Result<(), String> {
    #[derive(serde::Deserialize)]
    struct EndpointConfig {
        /// List of endpoint subscription configurations
        subscriptions: Vec<EndpointSubscriptionConfig>,
    }

    let config: EndpointConfig = serde_json::from_str(EMBEDDED_CONFIG)
        .map_err(|e| format!("Failed to parse embedded config: {e}"))?;

    info!(
        "Loaded {} embedded endpoint subscriptions",
        config.subscriptions.len()
    );

    for sub_config in config.subscriptions {
        let subscription = EndpointSubscription::new(
            sub_config.subscription_id,
            sub_config.module_id,
            sub_config.methods,
            sub_config.path_regex,
            sub_config.content_types,
            sub_config.json_schema,
        )?;

        register_global_endpoint_subscription(subscription).await?;
    }

    Ok(())
}

/// Ensure HTTP gateway is initialized only once
static GATEWAY_INIT: std::sync::Once = std::sync::Once::new();

/// New context
pub(crate) fn new_context(_ctx: &crate::runtime_context::HermesRuntimeContext) {
    GATEWAY_INIT.call_once(|| {
        // Load endpoints first
        load_endpoints_sync();

        // Then start the gateway
        spawn();
    });
}

/// Load endpoints synchronously to avoid race conditions
fn load_endpoints_sync() {
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            error!("Failed to create runtime for endpoint loading: {}", e);
            return;
        },
    };

    rt.block_on(async {
        if let Err(e) = load_embedded_endpoints().await {
            error!("Failed to load endpoints: {}", e);
        } else {
            info!("HTTP Gateway endpoints loaded successfully");
        }
    });
}
