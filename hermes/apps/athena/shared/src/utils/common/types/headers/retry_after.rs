//! Retry After header type
//!
//! This is an active header which expects to be provided in a response.

use std::fmt::Display;

use chrono::{DateTime, Utc};

/// Parameter which describes the possible choices for a Retry-After header field.
#[derive(Debug)]
#[allow(dead_code)] // Its OK if all these variants are not used.
pub enum RetryAfterHeader {
    /// Http Date
    Date(DateTime<Utc>),
    /// Interval in seconds.
    Seconds(u64),
}

/// Parameter which lets us set the retry header, or use some default.
/// Needed, because its valid to exclude the retry header specifically.
/// This is also due to the way Poem handles optional headers.
#[derive(Debug)]
#[allow(dead_code)] // Its OK if all these variants are not used.
pub enum RetryAfterOption {
    /// Use a default Retry After header value
    Default,
    /// Don't include the Retry After header value in the response.
    None,
    /// Use a specific Retry After header value
    Some(RetryAfterHeader),
}

impl Display for RetryAfterHeader {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            RetryAfterHeader::Date(date_time) => {
                let http_date = date_time.format("%a, %d %b %Y %T GMT").to_string();
                write!(f, "{http_date}")
            },
            RetryAfterHeader::Seconds(secs) => write!(f, "{secs}"),
        }
    }
}

impl Default for RetryAfterHeader {
    fn default() -> Self {
        Self::Seconds(300)
    }
}
