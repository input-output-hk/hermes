//! Nonce

use std::sync::LazyLock;

use serde_json::Value;

use super::slot_no;

/// Title.
const TITLE: &str = "Nonce";
/// Description.
const DESCRIPTION: &str = "The current slot at the time a transaction was posted.
Used to ensure out of order inclusion on-chain can be detected.

*Note: Because a Nonce should never be greater than the slot of the transaction it is found in,
excessively large nonces are capped to the transactions slot number.*";
/// Example.
pub(crate) const EXAMPLE: u64 = slot_no::EXAMPLE;
/// Minimum.
const MINIMUM: u64 = 0;
/// Maximum.
const MAXIMUM: u64 = u64::MAX;

/// Value of a Nonce.
#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub(crate) struct Nonce(u64);

/// Is the Nonce valid?
fn is_valid(value: u64) -> bool {
    (MINIMUM..=MAXIMUM).contains(&value)
}

impl From<u64> for Nonce {
    fn from(value: u64) -> Self {
        Self(value)
    }
}
