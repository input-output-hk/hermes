//! HTTP Gateway Subscription Management
//!
//! This module provides intelligent routing of HTTP requests to specific WASM modules
//! based on configurable endpoint subscriptions. Instead of broadcasting every request
//! to all modules, it uses pattern matching and specificity scoring to route requests
//! directly to the most appropriate handler.
//!
//! ## Key Features
//!
//! - **Best Match Routing**: Uses regex specificity scoring to find the most specific match
//! - **Performance Optimization**: Early exit for "perfect" matches to reduce latency
//! - **Flexible Matching**: Supports method, path, and content-type filtering
//! - **Configuration Driven**: Endpoint rules loaded from configuration files
//!
//! ## How It Works
//!
//! 1. **Subscription Registration**: Modules register endpoint patterns with methods,
//!    path regexes, and content types they can handle
//! 2. **Request Matching**: Incoming requests are matched against all subscriptions
//! 3. **Specificity Scoring**: Each matching subscription gets a specificity score
//! 4. **Best Match Selection**: The subscription with the highest score handles the request
//!
//! ## Example
//!
//! ```ignore
//! // High specificity (score: ~26)
//! Subscription {
//!     path_regex: "^/api/v1/users/[0-9]+$",
//!     methods: ["GET", "PUT"],
//!     module_id: "user_service"
//! }
//!
//! // Lower specificity (score: ~12)
//! Subscription {
//!     path_regex: "^/api/.*$",
//!     methods: [],  // Accept all methods
//!     module_id: "general_api"
//! }
//!
//! // Request: GET /api/v1/users/123
//! // Result: Routed to "user_service" (higher specificity)
//! ```

use crate::runtime_extensions::utils::regex_ranking::regex_specificity_score;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

//
// Core Data Types
//

/// HTTP endpoint subscription defining which module handles specific request patterns
///
/// Each subscription represents a route pattern that a WASM module can handle.
/// The gateway uses these subscriptions to route incoming HTTP requests directly
/// to the appropriate module without broadcasting to all modules.
///
/// ## Fields
///
/// - `subscription_id`: Unique identifier for logging and debugging
/// - `module_id`: Target WASM module that will handle matching requests
/// - `methods`: HTTP methods accepted (empty = accept all methods)
/// - `path_regex`: Regular expression pattern for URL path matching
/// - `content_types`: MIME types accepted (empty = accept all types)
/// - `json_schema`: Optional JSON schema path for request validation
/// - `specificity_score`: Calculated priority score (higher = more specific)
///
/// ## Specificity Scoring
///
/// The scoring system ensures that more specific patterns take precedence:
/// - More literal characters = higher score
/// - Anchors (^, $) = bonus points
/// - Wildcards (.*) = penalty
/// - Specific methods/content-types = bonus points
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointSubscription {
    /// Unique identifier for this subscription (used in logs and debugging)
    pub subscription_id: String,

    /// WASM module that will handle requests matching this subscription
    pub module_id: String,

    /// HTTP methods this subscription accepts (empty vec = accept all methods)
    /// Common values: `["GET"]`, `["POST"]`, `["GET", "POST", "PUT", "DELETE"]`
    pub methods: Vec<String>,

    /// Regular expression pattern to match request paths
    /// Examples: "^/api/users$", "^/api/v1/registration(/.*)?$"
    pub path_regex: String,

    /// Compiled regex for efficient runtime matching (not serialized)
    #[serde(skip)]
    pub compiled_regex: Option<Regex>,

    /// Content types this subscription accepts (empty vec = accept all types)
    /// Examples: `["application/json"]`, `["text/html", "application/json"]`
    pub content_types: Vec<String>,

    /// Optional path to JSON Schema file for request body validation
    /// Only used when `content_types` includes "application/json"
    pub json_schema: Option<String>,

    /// Calculated specificity score for priority ordering (higher = more specific)
    /// This is computed automatically during subscription creation
    pub specificity_score: i32,
}

//
// EndpointSubscription Implementation
//

impl EndpointSubscription {
    /// Creates a new endpoint subscription with automatic specificity calculation
    ///
    /// ## Parameters
    /// - `subscription_id`: Unique identifier for this subscription
    /// - `module_id`: Target WASM module name
    /// - `methods`: HTTP methods to accept (empty = all methods)
    /// - `path_regex`: Regular expression for path matching
    /// - `content_types`: MIME types to accept (empty = all types)
    /// - `json_schema`: Optional JSON schema file path
    ///
    /// ## Returns
    /// - `Ok(EndpointSubscription)`: Successfully created subscription
    /// - `Err(String)`: Error message if regex pattern is invalid
    ///
    /// ## Example
    /// ```ignore
    /// let subscription = EndpointSubscription::new(
    ///     "user_api".to_string(),
    ///     "user_service".to_string(),
    ///     vec!["GET".to_string(), "POST".to_string()],
    ///     "^/api/v1/users(/[0-9]+)?$".to_string(),
    ///     vec!["application/json".to_string()],
    ///     Some("user-schema.json".to_string()),
    /// )?;
    /// ```
    pub fn new(
        subscription_id: String,
        module_id: String,
        methods: Vec<String>,
        path_regex: String,
        content_types: Vec<String>,
        json_schema: Option<String>,
    ) -> Result<Self, String> {
        // Validate and compile the regex pattern
        let compiled_regex = Self::compile_regex(&path_regex)
            .map_err(|e| format!("Invalid regex pattern '{path_regex}': {e}"))?;

        // Calculate specificity score for priority ordering
        let specificity_score =
            Self::calculate_specificity(&path_regex, &methods, json_schema.as_ref());

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

    /// Compiles a regex pattern and returns detailed error information
    fn compile_regex(pattern: &str) -> Result<Regex, String> {
        Regex::new(pattern).map_err(|e| format!("Regex compilation failed: {e}"))
    }

    /// Calculates the specificity score for priority ordering
    ///
    /// Higher scores indicate more specific patterns that should take precedence.
    /// The score considers:
    /// - Base regex specificity (literal chars, anchors, wildcards)
    /// - Method specificity bonus
    /// - JSON schema validation bonus
    fn calculate_specificity(
        path_regex: &str,
        methods: &[String],
        json_schema: Option<&String>,
    ) -> i32 {
        let mut score = regex_specificity_score(path_regex);

        // Bonus for having specific HTTP methods (not catch-all)
        if !methods.is_empty() {
            score = score.saturating_add(5);
        }

        // Bonus for having JSON schema validation
        if json_schema.is_some() {
            score = score.saturating_add(3);
        }

        score
    }

    /// Checks if this subscription matches the given request parameters
    ///
    /// A subscription matches if:
    /// 1. HTTP method is accepted (or methods list is empty)
    /// 2. Path matches the regex pattern
    /// 3. Content type is accepted (or `content_types` list is empty)
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

    /// Checks if the HTTP method is accepted by this subscription
    fn matches_method(
        &self,
        method: &str,
    ) -> bool {
        // Empty methods list means "accept all methods"
        self.methods.is_empty() || self.methods.contains(&method.to_uppercase())
    }

    /// Checks if the path matches this subscription's regex pattern
    fn matches_path(
        &self,
        path: &str,
    ) -> bool {
        self.compiled_regex
            .as_ref()
            .is_some_and(|regex| regex.is_match(path))
    }

    /// Checks if the content type is accepted by this subscription
    fn matches_content_type(
        &self,
        content_type: Option<&str>,
    ) -> bool {
        // Empty content_types list means "accept all content types"
        if self.content_types.is_empty() {
            return true;
        }

        // Check if provided content type matches any accepted type
        if let Some(ct) = content_type {
            self.content_types
                .iter()
                .any(|accepted| ct.contains(accepted))
        } else {
            // No content type provided - only match if we accept all types
            false
        }
    }
}

//
// Subscription Manager
//

/// Manages endpoint subscriptions with optimized best-match routing
///
/// The `SubscriptionManager` stores and indexes endpoint subscriptions for efficient
/// request routing. It uses a two-tier optimization strategy:
///
/// 1. **Score-based Organization**: Groups subscriptions by specificity score
/// 2. **Early Exit Optimization**: Stops searching when a "perfect" match is found
///
/// ## Internal Structure
///
/// ```ignore
/// subscriptions_by_score: {
///     26: [user_specific_subscription, product_specific_subscription],
///     15: [api_general_subscription],
///     -5: [catch_all_subscription]
/// }
/// sorted_scores: [26, 15, -5]  // Cached for performance
/// ```
///
/// This organization allows the manager to:
/// - Check highest specificity subscriptions first
/// - Stop early when a perfect match is found
/// - Fall back to lower specificity matches if needed
#[derive(Debug, Default)]
pub struct SubscriptionManager {
    /// Subscriptions grouped by their specificity scores
    /// Higher scores are checked first during matching
    subscriptions_by_score: HashMap<i32, Vec<EndpointSubscription>>,

    /// Fast lookup table for finding subscriptions by ID
    /// Used for debugging, logging, and administrative operations
    subscriptions_by_id: HashMap<String, EndpointSubscription>,

    /// Pre-sorted list of specificity scores in descending order
    /// Cached for performance - rebuilt when subscriptions change
    sorted_scores: Vec<i32>,

    /// Indicates whether `sorted_scores` cache needs to be rebuilt
    scores_dirty: bool,
}

impl SubscriptionManager {
    /// Creates a new empty subscription manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks if the manager has no registered subscriptions
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.subscriptions_by_id.is_empty()
    }

    /// Registers a new endpoint subscription in the manager
    ///
    /// The subscription is indexed by both its specificity score and ID for
    /// efficient lookup during request routing and administrative operations.
    ///
    /// ## Parameters
    /// - `subscription`: The endpoint subscription to register
    ///
    /// ## Returns
    /// - `Ok(())`: Subscription registered successfully
    /// - `Err(String)`: Error if subscription is invalid (e.g., bad regex)
    ///
    /// ## Side Effects
    /// - Adds subscription to internal indexes
    /// - Marks score cache as dirty for rebuild
    /// - Recompiles regex if not already compiled
    pub fn register_endpoint_subscription(
        &mut self,
        mut subscription: EndpointSubscription,
    ) -> Result<(), String> {
        // Ensure the regex is compiled for runtime matching
        Self::ensure_regex_compiled(&mut subscription)?;

        // Recalculate specificity to ensure consistency with current algorithm
        Self::update_specificity_score(&mut subscription);

        // Add to both indexes
        self.add_to_indexes(subscription);

        Ok(())
    }

    /// Ensures the subscription has a compiled regex pattern
    fn ensure_regex_compiled(subscription: &mut EndpointSubscription) -> Result<(), String> {
        if subscription.compiled_regex.is_none() {
            subscription.compiled_regex = Some(
                Regex::new(&subscription.path_regex)
                    .map_err(|e| format!("Invalid regex pattern: {e}"))?,
            );
        }
        Ok(())
    }

    /// Updates the specificity score to ensure consistency
    fn update_specificity_score(subscription: &mut EndpointSubscription) {
        subscription.specificity_score = EndpointSubscription::calculate_specificity(
            &subscription.path_regex,
            &subscription.methods,
            subscription.json_schema.as_ref(),
        );
    }

    /// Adds the subscription to both internal indexes
    fn add_to_indexes(
        &mut self,
        subscription: EndpointSubscription,
    ) {
        let score = subscription.specificity_score;
        let id = subscription.subscription_id.clone();

        // Add to score-based index for efficient matching
        self.subscriptions_by_score
            .entry(score)
            .or_default()
            .push(subscription.clone());

        // Add to ID-based index for administrative operations
        self.subscriptions_by_id.insert(id, subscription);

        // Mark score cache as needing rebuild
        self.scores_dirty = true;
    }

    /// Finds the best matching subscription for an HTTP request
    ///
    /// Uses an optimized two-phase algorithm:
    /// 1. **Perfect Match Phase**: Checks for high-specificity matches and returns immediately
    /// 2. **Best Match Phase**: Continues searching and returns the highest scoring match
    ///
    /// ## Parameters
    /// - `method`: HTTP method (GET, POST, etc.)
    /// - `path`: Request path to match against regex patterns
    /// - `content_type`: Optional content type from request headers
    ///
    /// ## Returns
    /// - `Some(subscription)`: Best matching subscription
    /// - `None`: No subscriptions match the request
    ///
    /// ## Performance
    /// - **Average case**: O(k) where k is subscriptions checked before perfect match
    /// - **Worst case**: O(n) where n is total subscriptions
    /// - **Best case**: O(1) when first subscription is a perfect match
    pub fn find_endpoint_subscription(
        &mut self,
        method: &str,
        path: &str,
        content_type: Option<&str>,
    ) -> Option<&EndpointSubscription> {
        self.ensure_scores_sorted();

        let mut best_candidate = BestMatchCandidate::none();

        // Check subscriptions in decreasing specificity order
        for &current_score in &self.sorted_scores {
            // Early exit: if we have a match and current score can't beat it, stop
            if best_candidate.can_stop_searching(current_score) {
                break;
            }

            // Check all subscriptions with this specificity score
            if let Some(subscriptions) = self.subscriptions_by_score.get(&current_score) {
                for subscription in subscriptions {
                    if subscription.matches(method, path, content_type) {
                        // Perfect match optimization: return immediately for high-specificity matches
                        if Self::is_perfect_match(subscription) {
                            return Some(subscription);
                        }

                        // Track the best match found so far
                        best_candidate.update_if_better(subscription, current_score);
                    }
                }
            }
        }

        best_candidate.get_subscription()
    }

    /// Ensures the score cache is up to date
    fn ensure_scores_sorted(&mut self) {
        if self.scores_dirty {
            self.rebuild_score_cache();
        }
    }

    /// Rebuilds the sorted scores cache
    fn rebuild_score_cache(&mut self) {
        self.sorted_scores = self.subscriptions_by_score.keys().copied().collect();
        self.sorted_scores.sort_by(|a, b| b.cmp(a)); // Descending order
        self.scores_dirty = false;
    }

    /// Determines if a subscription qualifies as a "perfect" match for early exit optimization
    ///
    /// Perfect matches are high-specificity subscriptions that represent well-defined
    /// endpoints rather than catch-all patterns. Finding a perfect match allows us to
    /// stop searching immediately, improving performance.
    ///
    /// ## Perfect Match Criteria
    /// 1. **Very High Specificity**: Score > 25 (always perfect)
    /// 2. **High Specificity + Quality**: Score > 15 AND has specific methods OR JSON schema
    ///
    /// ## Examples
    /// - `^/api/v1/users/[0-9]+$` with specific methods: Perfect (high specificity + specific)
    /// - `^/api/.*$` with no methods: Not perfect (lower specificity, catch-all)
    /// - `^/api/data$` with JSON schema: Perfect (moderate specificity + validation)
    fn is_perfect_match(subscription: &EndpointSubscription) -> bool {
        const HIGH_SPECIFICITY_THRESHOLD: i32 = 15;
        const VERY_HIGH_SPECIFICITY_THRESHOLD: i32 = 25;

        let score = subscription.specificity_score;

        // Very high specificity is always considered perfect
        if score > VERY_HIGH_SPECIFICITY_THRESHOLD {
            return true;
        }

        // High specificity with additional quality indicators
        if score > HIGH_SPECIFICITY_THRESHOLD {
            let has_specific_methods = !subscription.methods.is_empty();
            let has_validation = subscription.json_schema.is_some();

            // Either specific methods or validation qualifies as perfect
            return has_specific_methods || has_validation;
        }

        false
    }
}

//
// Helper Types
//

/// Tracks the best matching subscription candidate during search
///
/// This helper struct encapsulates the logic for tracking the best match
/// found so far and determining when we can stop searching early.
struct BestMatchCandidate<'a> {
    /// The best matching subscription found so far
    subscription: Option<&'a EndpointSubscription>,
    /// The specificity score of the best match
    score: i32,
}

impl<'a> BestMatchCandidate<'a> {
    /// Creates a new empty candidate
    fn none() -> Self {
        Self {
            subscription: None,
            score: i32::MIN,
        }
    }

    /// Updates the candidate if the new subscription has a better score
    fn update_if_better(
        &mut self,
        subscription: &'a EndpointSubscription,
        score: i32,
    ) {
        if score > self.score {
            self.subscription = Some(subscription);
            self.score = score;
        }
    }

    /// Determines if we can stop searching based on the current score
    ///
    /// We can stop if we have a match and the current score level can't beat it
    /// (since scores are processed in descending order)
    fn can_stop_searching(
        &self,
        current_score: i32,
    ) -> bool {
        self.subscription.is_some() && current_score < self.score
    }

    /// Returns the best subscription found, if any
    fn get_subscription(self) -> Option<&'a EndpointSubscription> {
        self.subscription
    }
}

//
// Global Subscription Manager
//

/// Global subscription manager instance (singleton)
///
/// This is initialized lazily when first accessed and provides thread-safe
/// access to the subscription management system.
static SUBSCRIPTION_MANAGER: once_cell::sync::Lazy<Arc<RwLock<SubscriptionManager>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(SubscriptionManager::new())));

/// Gets a reference to the global subscription manager
///
/// This function provides access to the singleton subscription manager instance.
/// The returned Arc can be cloned cheaply and shared across async tasks.
///
/// ## Returns
/// - `Arc<RwLock<SubscriptionManager>>`: Thread-safe reference to the global manager
pub fn get_subscription_manager() -> Arc<RwLock<SubscriptionManager>> {
    Arc::clone(&SUBSCRIPTION_MANAGER)
}

/// Registers a new endpoint subscription globally
///
/// This is the primary function used during gateway initialization to register
/// endpoint subscriptions loaded from configuration files.
///
/// ## Parameters
/// - `subscription`: The endpoint subscription to register
///
/// ## Returns
/// - `Ok(())`: Subscription registered successfully
/// - `Err(String)`: Registration failed with error message
///
/// ## Example
/// ```ignore
/// let subscription = EndpointSubscription::new(
///     "api_users".to_string(),
///     "user_service".to_string(),
///     vec!["GET".to_string(), "POST".to_string()],
///     "^/api/v1/users(/.*)?$".to_string(),
///     vec!["application/json".to_string()],
///     None,
/// )?;
/// register_global_endpoint_subscription(subscription).await?;
/// ```
pub async fn register_global_endpoint_subscription(
    subscription: EndpointSubscription
) -> Result<(), String> {
    let manager = get_subscription_manager();
    let mut manager_lock = manager.write().await;
    manager_lock.register_endpoint_subscription(subscription)
}

/// Finds the best matching subscription for an incoming HTTP request
///
/// This is the primary lookup function used during request routing to determine
/// which WASM module should handle a specific HTTP request.
///
/// ## Parameters
/// - `method`: HTTP method (GET, POST, PUT, DELETE, etc.)
/// - `path`: Request URL path (e.g., "/api/v1/users/123")
/// - `content_type`: Optional content type from request headers
///
/// ## Returns
/// - `Some(subscription)`: Best matching subscription (cloned for thread safety)
/// - `None`: No subscription matches the request criteria
///
/// ## Example
/// ```ignore
/// let subscription = find_global_endpoint_subscription(
///     "POST",
///     "/api/v1/users/create",
///     Some("application/json"),
/// ).await;
///
/// if let Some(sub) = subscription {
///     println!("Routing to module: {}", sub.module_id);
/// }
/// ```
pub async fn find_global_endpoint_subscription(
    method: &str,
    path: &str,
    content_type: Option<&str>,
) -> Option<EndpointSubscription> {
    let manager = get_subscription_manager();
    let mut manager_lock = manager.write().await;
    manager_lock
        .find_endpoint_subscription(method, path, content_type)
        .cloned() // Clone to avoid holding the lock longer than necessary
}

//
// Tests
//

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that more specific regex patterns receive higher specificity scores
    #[test]
    fn test_subscription_creation_with_scoring() {
        let specific_sub = create_test_subscription(
            "specific",
            "^/api/users/[0-9]+$", // Specific pattern with character class
            vec!["GET"],
        );

        let general_sub = create_test_subscription(
            "general",
            "/api/.*", // General catch-all pattern
            vec!["GET"],
        );

        assert!(
            specific_sub.specificity_score > general_sub.specificity_score,
            "Specific pattern should have higher score than general pattern"
        );
    }

    /// Tests that subscription manager returns the highest scoring match
    #[test]
    fn test_subscription_manager_priority_matching() {
        let mut manager = SubscriptionManager::new();

        // Register general subscription first (lower specificity)
        let general_sub = create_test_subscription("general", "/api/.*", vec!["GET"]);
        manager.register_endpoint_subscription(general_sub).unwrap();

        // Register specific subscription second (higher specificity)
        let specific_sub = create_test_subscription("specific", "^/api/users/[0-9]+$", vec!["GET"]);
        manager
            .register_endpoint_subscription(specific_sub)
            .unwrap();

        // Should match the more specific subscription regardless of registration order
        let matched =
            manager.find_endpoint_subscription("GET", "/api/users/123", Some("application/json"));

        assert_eq!(matched.unwrap().subscription_id, "specific");
    }

    /// Tests the perfect match optimization for early exit
    #[test]
    fn test_perfect_match_optimization() {
        let mut manager = SubscriptionManager::new();

        // Perfect match: high specificity + JSON schema
        let perfect_sub = EndpointSubscription::new(
            "perfect".to_string(),
            "perfect_module".to_string(),
            vec!["POST".to_string()],
            "^/api/v1/users/create$".to_string(),
            vec!["application/json".to_string()],
            Some("user-schema.json".to_string()),
        )
        .unwrap();

        // Fallback match: lower specificity
        let fallback_sub = create_test_subscription("fallback", "^/api/.*$", vec!["POST"]);

        manager
            .register_endpoint_subscription(fallback_sub)
            .unwrap();
        manager.register_endpoint_subscription(perfect_sub).unwrap();

        let matched = manager.find_endpoint_subscription(
            "POST",
            "/api/v1/users/create",
            Some("application/json"),
        );

        assert_eq!(matched.unwrap().subscription_id, "perfect");
    }

    /// Tests that empty methods list accepts all HTTP methods
    #[test]
    fn test_empty_methods_accept_all() {
        let mut manager = SubscriptionManager::new();

        let catch_all_sub = EndpointSubscription::new(
            "catch_all".to_string(),
            "catch_all_module".to_string(),
            vec![], // Empty = accept all methods
            "^/api/v1/registration(/.*)?$".to_string(),
            vec![],
            None,
        )
        .unwrap();

        manager
            .register_endpoint_subscription(catch_all_sub)
            .unwrap();

        // Test multiple HTTP methods
        for method in ["GET", "POST", "PUT", "DELETE"] {
            let matched = manager.find_endpoint_subscription(method, "/api/v1/registration", None);
            assert!(matched.is_some(), "Should accept method: {}", method);
            assert_eq!(matched.unwrap().subscription_id, "catch_all");
        }
    }

    /// Tests that empty content types list accepts all content types
    #[test]
    fn test_empty_content_types_accept_all() {
        let mut manager = SubscriptionManager::new();

        let flexible_sub = EndpointSubscription::new(
            "flexible".to_string(),
            "flexible_module".to_string(),
            vec!["POST".to_string()],
            "^/api/upload$".to_string(),
            vec![], // Empty = accept all content types
            None,
        )
        .unwrap();

        manager
            .register_endpoint_subscription(flexible_sub)
            .unwrap();

        // Test different content types
        let test_cases = [
            Some("application/json"),
            Some("multipart/form-data"),
            Some("text/plain"),
            None, // No content type
        ];

        for content_type in test_cases {
            let matched = manager.find_endpoint_subscription("POST", "/api/upload", content_type);
            assert!(
                matched.is_some(),
                "Should accept content type: {:?}",
                content_type
            );
        }
    }

    /// Tests the perfect match detection criteria
    #[test]
    fn test_is_perfect_match_criteria() {
        // Very high specificity (should always be perfect)
        let very_high_sub = create_test_subscription(
            "very_high",
            "^/api/v1/users/[0-9]+/profile/settings$",
            vec![],
        );

        // High specificity with methods
        let high_with_methods = create_test_subscription(
            "high_methods",
            "^/api/v1/users/create$",
            vec!["GET", "POST"],
        );

        // High specificity with JSON schema
        let high_with_schema = EndpointSubscription::new(
            "high_schema".to_string(),
            "module".to_string(),
            vec![],
            "^/api/v1/data$".to_string(),
            vec!["application/json".to_string()],
            Some("schema.json".to_string()),
        )
        .unwrap();

        // Low specificity (should not be perfect)
        let low_specificity = create_test_subscription(
            "low",
            ".*", // Very general pattern
            vec!["GET"],
        );

        // Verify perfect match detection
        assert!(SubscriptionManager::is_perfect_match(&very_high_sub));
        assert!(SubscriptionManager::is_perfect_match(&high_with_methods));
        assert!(SubscriptionManager::is_perfect_match(&high_with_schema));
        assert!(!SubscriptionManager::is_perfect_match(&low_specificity));
    }

    /// Tests individual matching functions
    #[test]
    fn test_individual_matching_functions() {
        let subscription = EndpointSubscription::new(
            "test".to_string(),
            "test_module".to_string(),
            vec!["GET".to_string(), "POST".to_string()],
            "^/api/users/[0-9]+$".to_string(),
            vec!["application/json".to_string()],
            None,
        )
        .unwrap();

        // Test method matching
        assert!(subscription.matches("GET", "/api/users/123", Some("application/json")));
        assert!(subscription.matches("POST", "/api/users/123", Some("application/json")));
        assert!(!subscription.matches("DELETE", "/api/users/123", Some("application/json")));

        // Test path matching
        assert!(subscription.matches("GET", "/api/users/123", Some("application/json")));
        assert!(!subscription.matches("GET", "/api/posts/123", Some("application/json")));

        // Test content type matching
        assert!(subscription.matches("GET", "/api/users/123", Some("application/json")));
        assert!(!subscription.matches("GET", "/api/users/123", Some("text/html")));
    }

    /// Helper function to create test subscriptions with common patterns
    fn create_test_subscription(
        id: &str,
        path_regex: &str,
        methods: Vec<&str>,
    ) -> EndpointSubscription {
        EndpointSubscription::new(
            id.to_string(),
            "test_module".to_string(),
            methods.into_iter().map(|s| s.to_string()).collect(),
            path_regex.to_string(),
            vec!["application/json".to_string()],
            None,
        )
        .unwrap()
    }
}
