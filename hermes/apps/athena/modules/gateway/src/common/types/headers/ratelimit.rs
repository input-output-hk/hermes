//! `RateLimit` Header type.
//!
//! This is a passive type, produced automatically by the CORS middleware.

use crate::common::types::string_types::impl_string_types;

// Access-Control-Allow-Origin Header String Type
impl_string_types!(RateLimitHeader, "string", "rate-limit");
