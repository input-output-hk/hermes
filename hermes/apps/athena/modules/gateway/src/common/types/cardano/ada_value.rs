//! ADA coins value on the blockchain.

use std::fmt::Display;

use anyhow::bail;
use num_bigint::BigInt;

/// Title.
const TITLE: &str = "Cardano Blockchain ADA coins value";
/// Description.
const DESCRIPTION: &str = "The ADA coins value of a Cardano Block on the chain.";
/// Example.
pub(crate) const EXAMPLE: u64 = 1_234_567;
/// Minimum.
const MINIMUM: u64 = 0;
/// Maximum.
const MAXIMUM: u64 = u64::MAX;

/// ADA coins value on the blockchain.
#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy, PartialOrd, Ord, Default)]
pub(crate) struct AdaValue(u64);

impl Display for AdaValue {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AdaValue {
    /// Performs saturating addition.
    pub(crate) fn saturating_add(
        self,
        v: Self,
    ) -> Self {
        self.0.checked_add(v.0).map_or_else(
            || {
                tracing::error!("Ada value overflow: {self} + {v}",);
                Self(u64::MAX)
            },
            Self,
        )
    }
}

/// Is the Slot Number valid?
fn is_valid(_value: u64) -> bool {
    true
}

impl From<AdaValue> for BigInt {
    fn from(val: AdaValue) -> Self {
        BigInt::from(val.0)
    }
}

impl TryFrom<num_bigint::BigInt> for AdaValue {
    type Error = anyhow::Error;

    fn try_from(value: num_bigint::BigInt) -> Result<Self, Self::Error> {
        let value: u64 = value.try_into()?;
        if !is_valid(value) {
            bail!("Invalid ADA Value");
        }
        Ok(Self(value))
    }
}
