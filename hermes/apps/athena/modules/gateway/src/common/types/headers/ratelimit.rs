//! `RateLimit` Header type.
//!
//! This is a passive type, produced automatically by the CORS middleware.

use std::sync::LazyLock;

use serde_json::Value;

use crate::common::types::string_types::impl_string_types;

/// Tite for the header in documentation.
const TITLE: &str = "RateLimit HTTP header.";

/// Description for the header in documentation.
const DESCRIPTION: &str = "Allows this server to advertise its quota policies and the current
service limits, thereby allowing clients to avoid being throttled.";

/// Example for the header in documentation.
const EXAMPLE: &str = r#""default";q=100;w=10"#;

// Access-Control-Allow-Origin Header String Type
impl_string_types!(RateLimitHeader, "string", "rate-limit");
