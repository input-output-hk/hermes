//! Access-Control-Allow-Origin Header type.
//!
//! This is a passive type, produced automatically by the CORS middleware.

use serde_json::Value;
use std::sync::LazyLock;

use crate::common::types::string_types::impl_string_types;

/// Tite for the header in documentation.
const TITLE: &str = "Access-Control-Allow-Origin header.";

/// Description for the header in documentation.
const DESCRIPTION: &str = "Valid formats:

* `Access-Control-Allow-Origin: *`
* `Access-Control-Allow-Origin: <origin>`
* `Access-Control-Allow-Origin: null`
";

/// Example for the header in documentation.
const EXAMPLE: &str = "*";

// Access-Control-Allow-Origin Header String Type
impl_string_types!(AccessControlAllowOriginHeader, "string", "origin");
