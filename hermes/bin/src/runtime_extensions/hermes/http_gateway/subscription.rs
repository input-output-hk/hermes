// src/runtime_extensions/hermes/http_gateway/subscriptions.rs
use crate::runtime_extensions::utils::regex_ranking::regex_specificity_score;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointSubscription {
    /// Unique identifier for this subscription
    pub subscription_id: String,
    /// Module that owns this subscription
    pub module_id: String,
    /// HTTP methods this subscription handles (GET, POST, etc.)
    pub methods: Vec<String>,
    /// Regex pattern to match request paths
    pub path_regex: String,
    /// Compiled regex for efficient matching
    #[serde(skip)]
    pub compiled_regex: Option<Regex>,
    /// Content types this subscription accepts
    pub content_types: Vec<String>,
    /// JSON schema for request validation (if content type includes JSON)
    pub json_schema: Option<String>,
    /// Specificity score for priority ordering (higher = more specific)
    pub specificity_score: i32,
}

impl EndpointSubscription {
    /// Create a new subscription with automatic specificity scoring
    pub fn new(
        subscription_id: String,
        module_id: String,
        methods: Vec<String>,
        path_regex: String,
        content_types: Vec<String>,
        json_schema: Option<String>,
    ) -> Result<Self, String> {
        // Compile regex for validation
        let compiled_regex = Regex::new(&path_regex)
            .map_err(|e| format!("Invalid regex pattern '{}': {}", path_regex, e))?;

        // Calculate specificity score using existing function
        let specificity_score = regex_specificity_score(&path_regex);

        Ok(Self {
            subscription_id,
            module_id,
            methods,
            path_regex,
            compiled_regex: Some(compiled_regex),
            content_types,
            json_schema,
            specificity_score,
        })
    }
}

/// Container for managing endpoint subscriptions with priority-based lookup
#[derive(Debug, Default)]
pub struct SubscriptionManager {
    /// Subscriptions organized by specificity score for efficient matching
    subscriptions_by_score: HashMap<i32, Vec<EndpointSubscription>>,
    /// Quick lookup by subscription ID
    subscriptions_by_id: HashMap<String, EndpointSubscription>,
    /// Cached sorted scores for efficient iteration (highest first)
    sorted_scores: Vec<i32>,
    /// Flag to indicate if sorted_scores needs refresh
    scores_dirty: bool,
}

impl SubscriptionManager {
    /// Check if the manager has no subscriptions
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.subscriptions_by_id.is_empty()
    }

    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new endpoint subscription with automatic specificity calculation
    pub fn register_endpoint_subscription(
        &mut self,
        mut subscription: EndpointSubscription,
    ) -> Result<(), String> {
        // Ensure regex is compiled
        if subscription.compiled_regex.is_none() {
            subscription.compiled_regex = Some(
                Regex::new(&subscription.path_regex)
                    .map_err(|e| format!("Invalid regex pattern: {}", e))?,
            );
        }

        // Recalculate specificity score to ensure consistency
        subscription.specificity_score = regex_specificity_score(&subscription.path_regex);

        let score = subscription.specificity_score;
        let id = subscription.subscription_id.clone();

        // Add to score-based lookup
        self.subscriptions_by_score
            .entry(score)
            .or_insert_with(Vec::new)
            .push(subscription.clone());

        // Add to ID-based lookup
        self.subscriptions_by_id.insert(id, subscription);

        // Mark scores as dirty for re-sorting
        self.scores_dirty = true;

        Ok(())
    }

    /// Find matching subscription for a request (highest specificity first)
    pub fn find_endpoint_subscription(
        &mut self,
        method: &str,
        path: &str,
        content_type: Option<&str>,
    ) -> Option<&EndpointSubscription> {
        self.refresh_sorted_scores();

        // Iterate through scores in descending order (most specific first)
        for &score in &self.sorted_scores {
            if let Some(subscriptions) = self.subscriptions_by_score.get(&score) {
                for subscription in subscriptions {
                    if self.matches_subscription(subscription, method, path, content_type) {
                        return Some(subscription);
                    }
                }
            }
        }

        None
    }

    /// Check if a subscription matches the request criteria
    fn matches_subscription(
        &self,
        subscription: &EndpointSubscription,
        method: &str,
        path: &str,
        content_type: Option<&str>,
    ) -> bool {
        // Check HTTP method
        if !subscription.methods.contains(&method.to_uppercase()) {
            return false;
        }

        // Check path regex
        if let Some(ref regex) = subscription.compiled_regex {
            if !regex.is_match(path) {
                return false;
            }
        } else {
            return false;
        }

        // Check content type if specified
        if let Some(ct) = content_type {
            if !subscription.content_types.is_empty()
                && !subscription
                    .content_types
                    .iter()
                    .any(|accepted| ct.contains(accepted))
            {
                return false;
            }
        }

        true
    }

    /// Refresh the sorted scores cache if dirty
    fn refresh_sorted_scores(&mut self) {
        if self.scores_dirty {
            self.sorted_scores = self.subscriptions_by_score.keys().copied().collect();
            self.sorted_scores.sort_by(|a, b| b.cmp(a)); // Descending order (highest first)
            self.scores_dirty = false;
        }
    }
}

/// Global subscription manager instance
static SUBSCRIPTION_MANAGER: once_cell::sync::Lazy<Arc<RwLock<SubscriptionManager>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(SubscriptionManager::new())));

/// Get a reference to the global subscription manager
pub async fn get_subscription_manager() -> Arc<RwLock<SubscriptionManager>> {
    Arc::clone(&SUBSCRIPTION_MANAGER)
}

/// Register a new endpoint subscription globally
pub async fn register_global_endpoint_subscription(
    subscription: EndpointSubscription
) -> Result<(), String> {
    let manager = get_subscription_manager().await;
    let mut manager_lock = manager.write().await;
    manager_lock.register_endpoint_subscription(subscription)
}

/// Find a matching subscription for an incoming request
pub async fn find_global_endpoint_subscription(
    method: &str,
    path: &str,
    content_type: Option<&str>,
) -> Option<EndpointSubscription> {
    let manager = get_subscription_manager().await;
    let mut manager_lock = manager.write().await;
    manager_lock
        .find_endpoint_subscription(method, path, content_type)
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscription_creation_with_scoring() {
        // More specific regex should get higher score
        let specific_sub = EndpointSubscription::new(
            "test1".to_string(),
            "module1".to_string(),
            vec!["GET".to_string()],
            "^/api/users/[0-9]+$".to_string(),
            vec!["application/json".to_string()],
            None,
        )
        .unwrap();

        let general_sub = EndpointSubscription::new(
            "test2".to_string(),
            "module2".to_string(),
            vec!["GET".to_string()],
            "/api/.*".to_string(),
            vec!["application/json".to_string()],
            None,
        )
        .unwrap();

        // More specific pattern should have higher score
        assert!(specific_sub.specificity_score > general_sub.specificity_score);
    }

    #[test]
    fn test_subscription_manager_priority_matching() {
        let mut manager = SubscriptionManager::new();

        // Register less specific subscription first
        let general_sub = EndpointSubscription::new(
            "general".to_string(),
            "general_module".to_string(),
            vec!["GET".to_string()],
            "/api/.*".to_string(),
            vec!["application/json".to_string()],
            None,
        )
        .unwrap();

        let specific_sub = EndpointSubscription::new(
            "specific".to_string(),
            "specific_module".to_string(),
            vec!["GET".to_string()],
            "^/api/users/[0-9]+$".to_string(),
            vec!["application/json".to_string()],
            None,
        )
        .unwrap();

        manager.register_endpoint_subscription(general_sub).unwrap();
        manager
            .register_endpoint_subscription(specific_sub)
            .unwrap();

        // Should match the more specific subscription first
        let matched =
            manager.find_endpoint_subscription("GET", "/api/users/123", Some("application/json"));

        assert!(matched.is_some());
        assert_eq!(matched.unwrap().subscription_id, "specific");
    }
}
