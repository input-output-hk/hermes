//! HTTP Gateway

use gateway_task::spawn;
use tracing::{error, info};

mod event;
mod gateway_task;
mod host;
/// Gateway routing logic
mod routing;
/// Subscription management for targeted routing
mod subscription;

///  State.
static STATE: once_cell::sync::Lazy<()> = once_cell::sync::Lazy::new(|| {
    spawn();

    // Only spawn the async task if we're in a Tokio runtime context
    // This prevents panics when called from benchmarks or other non-async contexts
    if tokio::runtime::Handle::try_current().is_ok() {
        tokio::task::spawn(async {
            if let Err(e) = load_endpoints_from_config().await {
                error!("Failed to load endpoint configurations: {}", e);
            }
        });
    } else {
        // In benchmark or non-async context, we can't load endpoints
        // This is acceptable for benchmarking purposes
        info!("Not in Tokio runtime context, skipping endpoint configuration loading");
    }
});

/// New context
pub(crate) fn new_context(_ctx: &crate::runtime_context::HermesRuntimeContext) {
    // Init state event - this triggers endpoint loading
    let () = *STATE;
}

/// Load endpoint configurations from available config files
async fn load_endpoints_from_config() -> Result<(), String> {
    // Try multiple config locations
    let config_paths = [
        "config/endpoints.json",
        "endpoints.json",
        "../config/endpoints.json",
    ];

    for config_path in &config_paths {
        if std::path::Path::new(config_path).exists() {
            info!("Loading endpoint configuration from: {}", config_path);
            return load_endpoints_from_file(config_path).await;
        }
    }

    // If no config file found, just log it - don't load defaults
    info!("No endpoint config file found, no endpoints loaded");
    Ok(())
}

async fn load_endpoints_from_file(config_path: &str) -> Result<(), String> {
    #[derive(serde::Deserialize)]
    struct EndpointConfig {
        /// List of endpoint subscription configurations
        subscriptions: Vec<EndpointSubscriptionConfig>,
    }

    #[derive(serde::Deserialize)]
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

    use subscription::{register_global_endpoint_subscription, EndpointSubscription};

    let config_content =
        std::fs::read_to_string(config_path).map_err(|e| format!("Failed to read config: {e}"))?;

    let config: EndpointConfig = serde_json::from_str(&config_content)
        .map_err(|e| format!("Failed to parse config: {e}"))?;

    info!(
        "Loaded {} endpoint subscriptions",
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use subscription::get_subscription_manager;
    use tempfile::TempDir;
    use tokio::test;

    async fn clear_global_subscriptions() {
        let manager = get_subscription_manager();
        let mut manager_lock = manager.write().await;
        *manager_lock = subscription::SubscriptionManager::new();
    }

    #[test]
    async fn test_load_valid_config_file() {
        clear_global_subscriptions().await;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("endpoints.json");

        let config = r#"{
            "subscriptions": [
                {
                    "subscription_id": "test_endpoint",
                    "module_id": "test_module",
                    "methods": ["POST"],
                    "path_regex": "^/api/test$",
                    "content_types": ["application/json"],
                    "json_schema": null
                }
            ]
        }"#;

        fs::write(&config_path, config).unwrap();

        let result = load_endpoints_from_file(config_path.to_str().unwrap()).await;
        assert!(result.is_ok());

        // Verify endpoint was registered
        let manager = get_subscription_manager();
        let mut manager_lock = manager.write().await;
        let found =
            manager_lock.find_endpoint_subscription("POST", "/api/test", Some("application/json"));
        assert!(found.is_some());
        assert_eq!(found.unwrap().module_id, "test_module");
    }

    #[test]
    async fn test_no_config_file_found() {
        clear_global_subscriptions().await;

        let result = load_endpoints_from_config().await;
        assert!(result.is_ok()); // Should succeed even with no config

        // Verify no endpoints were loaded
        let manager = get_subscription_manager();
        let manager_lock = manager.read().await;
        assert!(manager_lock.is_empty());
    }

    #[test]
    async fn test_invalid_config_handling() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("bad.json");
        fs::write(&config_path, "{ invalid }").unwrap();

        let result = load_endpoints_from_file(config_path.to_str().unwrap()).await;
        assert!(result.is_err());
    }
}