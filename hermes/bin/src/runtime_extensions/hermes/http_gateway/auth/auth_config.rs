//! Auth configuration

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::runtime_extensions::bindings::hermes::http_gateway;

/// Auth configuration file
const AUTH_CONFIG_FILE: &str = include_str!("config/auth.json");

/// Auth configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct AuthConfig {
    /// Auth rules
    pub auth_rules: Vec<AuthRule>,
    /// Default auth level, if no rule matches
    pub default_auth_level: AuthLevel,
}

/// Auth rule
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct AuthRule {
    /// Path regex
    pub path_regex: String,
    /// Method
    pub method: String,
    /// Auth level
    pub auth_level: AuthLevel,
}

/// Auth level
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum AuthLevel {
    /// Required auth
    Required,
    /// Optional auth
    Optional,
    /// No auth required
    None,
}

/// Load auth configuration from config file.
/// This should not fail, but if it does, it will return a default config.
pub(crate) static AUTH_CONFIG: LazyLock<AuthConfig> = LazyLock::new(|| {
    serde_json::from_str(AUTH_CONFIG_FILE).unwrap_or_else(|e| {
        error!(error=%e, "Failed to parse auth config file");
        AuthConfig::default()
    })
});

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            auth_rules: vec![],
            default_auth_level: AuthLevel::Required,
        }
    }
}

impl AuthConfig {
    /// Get auth level for a given method and path.
    pub(crate) fn get_auth_level(
        &self,
        method: &str,
        path: &str,
    ) -> AuthLevel {
        for rule in &self.auth_rules {
            if Self::matches_rule(method, path, rule) {
                return rule.auth_level.clone();
            }
        }
        self.default_auth_level.clone()
    }

    /// Check if a rule matches a given method and path.
    fn matches_rule(
        method: &str,
        path: &str,
        rule: &AuthRule,
    ) -> bool {
        // Case insensitive method match
        let method_matches = rule.method.eq_ignore_ascii_case(method);

        // Regex based path match
        let path_matches = match Regex::new(&rule.path_regex) {
            Ok(regex) => regex.is_match(path),
            Err(e) => {
                error!(error=%e,"Invalid regex in auth rule '{}'", rule.path_regex);
                false
            },
        };
        method_matches && path_matches
    }
}

impl From<AuthLevel> for http_gateway::api::AuthLevel {
    fn from(auth_level: AuthLevel) -> Self {
        match auth_level {
            AuthLevel::Required => http_gateway::api::AuthLevel::Required,
            AuthLevel::Optional => http_gateway::api::AuthLevel::Optional,
            AuthLevel::None => http_gateway::api::AuthLevel::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config() {
        // Ensure that the config loads successfully
        assert!(
            !AUTH_CONFIG.auth_rules.is_empty(),
            "Auth rules should not be empty"
        );
    }
}
