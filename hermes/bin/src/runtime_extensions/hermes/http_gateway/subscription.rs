//! HTTP Gateway Subscription Management
//!
//! Routes HTTP requests to specific WASM modules based on endpoint subscriptions.
//! Uses regex pattern matching with simple specificity-based priority.

use crate::runtime_extensions::utils::regex_ranking::regex_specificity_score;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// HTTP endpoint subscription defining which module handles specific request patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointSubscription {
    /// Unique identifier for this subscription
    pub subscription_id: String,

    /// WASM module that will handle matching requests
    pub module_id: String,

    /// HTTP methods accepted (empty = accept all)
    pub methods: Vec<String>,

    /// Regular expression pattern for path matching
    pub path_regex: String,

    /// Compiled regex for matching (not serialized)
    #[serde(skip)]
    pub compiled_regex: Option<Regex>,

    /// Content types accepted (empty = accept all)
    pub content_types: Vec<String>,

    /// Optional JSON schema file path
    pub json_schema: Option<String>,
}

impl EndpointSubscription {
    /// Creates a new endpoint subscription
    pub fn new(
        subscription_id: String,
        module_id: String,
        methods: Vec<String>,
        path_regex: String,
        content_types: Vec<String>,
        json_schema: Option<String>,
    ) -> Result<Self, String> {
        let compiled_regex = Regex::new(&path_regex)
            .map_err(|e| format!("Invalid regex pattern '{path_regex}': {e}"))?;

        Ok(Self {
            subscription_id,
            module_id,
            methods,
            path_regex,
            compiled_regex: Some(compiled_regex),
            content_types,
            json_schema,
        })
    }

    /// Checks if this subscription matches the request
    pub fn matches(
        &self,
        method: &str,
        path: &str,
        content_type: Option<&str>,
    ) -> bool {
        self.matches_method(method)
            && self.matches_path(path)
            && self.matches_content_type(content_type)
    }

    /// Checks if the HTTP method is accepted
    fn matches_method(
        &self,
        method: &str,
    ) -> bool {
        self.methods.is_empty() || self.methods.contains(&method.to_uppercase())
    }

    /// Checks if the path matches the regex pattern
    fn matches_path(
        &self,
        path: &str,
    ) -> bool {
        self.compiled_regex
            .as_ref()
            .is_some_and(|regex| regex.is_match(path))
    }

    /// Checks if the content type is accepted
    fn matches_content_type(
        &self,
        content_type: Option<&str>,
    ) -> bool {
        if self.content_types.is_empty() {
            return true;
        }

        if let Some(ct) = content_type {
            self.content_types
                .iter()
                .any(|accepted| ct.contains(accepted))
        } else {
            false
        }
    }

    /// Gets the specificity score for this subscription
    pub fn specificity_score(&self) -> i32 {
        regex_specificity_score(&self.path_regex)
    }
}

/// Manages endpoint subscriptions with simple priority-based routing
#[derive(Debug, Default)]
pub struct SubscriptionManager {
    /// All subscriptions, sorted by specificity (most specific first)
    subscriptions: Vec<EndpointSubscription>,
}

impl SubscriptionManager {
    /// Creates a new subscription manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new endpoint subscription
    pub fn register_endpoint_subscription(
        &mut self,
        mut subscription: EndpointSubscription,
    ) -> Result<(), String> {
        // Ensure regex is compiled
        if subscription.compiled_regex.is_none() {
            subscription.compiled_regex = Some(
                Regex::new(&subscription.path_regex)
                    .map_err(|e| format!("Invalid regex pattern: {e}"))?,
            );
        }

        // Insert in correct position to maintain sorted order
        let specificity = subscription.specificity_score();
        let insert_pos = self
            .subscriptions
            .binary_search_by_key(&std::cmp::Reverse(specificity), |s| {
                std::cmp::Reverse(s.specificity_score())
            })
            .unwrap_or_else(|pos| pos);

        self.subscriptions.insert(insert_pos, subscription);
        Ok(())
    }

    /// Finds the best matching subscription for a request
    pub fn find_endpoint_subscription(
        &self,
        method: &str,
        path: &str,
        content_type: Option<&str>,
    ) -> Option<&EndpointSubscription> {
        // Simple linear search through sorted subscriptions
        // Returns first match, which is automatically the most specific
        self.subscriptions
            .iter()
            .find(|sub| sub.matches(method, path, content_type))
    }

    /// Returns the number of registered subscriptions
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.subscriptions.len()
    }

    /// Checks if the manager has no subscriptions
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.subscriptions.is_empty()
    }
}

/// Global subscription manager instance (singleton)
///
/// This is initialized lazily when first accessed and provides thread-safe
/// access to the subscription management system.
static SUBSCRIPTION_MANAGER: once_cell::sync::Lazy<Arc<RwLock<SubscriptionManager>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(SubscriptionManager::new())));

/// Gets a reference to the global subscription manager
pub fn get_subscription_manager() -> Arc<RwLock<SubscriptionManager>> {
    Arc::clone(&SUBSCRIPTION_MANAGER)
}

/// Registers a new endpoint subscription globally
pub async fn register_global_endpoint_subscription(
    subscription: EndpointSubscription
) -> Result<(), String> {
    let manager = get_subscription_manager();
    let mut manager_lock = manager.write().await;
    manager_lock.register_endpoint_subscription(subscription)
}

/// Finds the best matching subscription for an HTTP request
pub async fn find_global_endpoint_subscription(
    method: &str,
    path: &str,
    content_type: Option<&str>,
) -> Option<EndpointSubscription> {
    let manager = get_subscription_manager();
    let manager_lock = manager.read().await;
    manager_lock
        .find_endpoint_subscription(method, path, content_type)
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscription_creation() {
        let sub = EndpointSubscription::new(
            "test".to_string(),
            "test_module".to_string(),
            vec!["GET".to_string()],
            "^/api/test$".to_string(),
            vec!["application/json".to_string()],
            None,
        );
        assert!(sub.is_ok());
    }

    #[test]
    fn test_matching_priority() {
        let mut manager = SubscriptionManager::new();

        // Add general pattern first
        let general = EndpointSubscription::new(
            "general".to_string(),
            "general_module".to_string(),
            vec!["GET".to_string()],
            "^/api/.*$".to_string(),
            vec![],
            None,
        )
        .unwrap();

        // Add specific pattern second
        let specific = EndpointSubscription::new(
            "specific".to_string(),
            "specific_module".to_string(),
            vec!["GET".to_string()],
            "^/api/users/[0-9]+$".to_string(),
            vec![],
            None,
        )
        .unwrap();

        manager.register_endpoint_subscription(general).unwrap();
        manager.register_endpoint_subscription(specific).unwrap();

        // Should match specific pattern
        let result = manager.find_endpoint_subscription("GET", "/api/users/123", None);
        assert_eq!(result.unwrap().subscription_id, "specific");
    }

    #[test]
    fn test_method_matching() {
        let sub = EndpointSubscription::new(
            "test".to_string(),
            "test_module".to_string(),
            vec!["GET".to_string(), "POST".to_string()],
            "^/api/test$".to_string(),
            vec![],
            None,
        )
        .unwrap();

        assert!(sub.matches("GET", "/api/test", None));
        assert!(sub.matches("POST", "/api/test", None));
        assert!(!sub.matches("DELETE", "/api/test", None));
    }

    #[test]
    fn test_empty_methods_accepts_all() {
        let sub = EndpointSubscription::new(
            "test".to_string(),
            "test_module".to_string(),
            vec![], // Empty = accept all methods
            "^/api/test$".to_string(),
            vec![],
            None,
        )
        .unwrap();

        assert!(sub.matches("GET", "/api/test", None));
        assert!(sub.matches("POST", "/api/test", None));
        assert!(sub.matches("DELETE", "/api/test", None));
    }
}
