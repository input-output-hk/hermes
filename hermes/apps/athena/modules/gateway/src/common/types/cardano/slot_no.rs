//! Slot Number on the blockchain.

use anyhow::bail;
use cardano_blockchain_types::Slot;
use num_bigint::BigInt;

/// Slot number
#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy, PartialOrd, Ord)]
pub(crate) struct SlotNo(u64);

impl SlotNo {
    /// Maximum.
    pub(crate) const MAXIMUM: SlotNo = SlotNo(u64::MAX / 2);
    /// Minimum.
    pub(crate) const MINIMUM: SlotNo = SlotNo(0);

    /// Is the Slot Number valid?
    fn is_valid(value: u64) -> bool {
        (Self::MINIMUM.0..=Self::MAXIMUM.0).contains(&value)
    }

    /// Generic conversion of `Option<T>` to `Option<SlotNo>`.
    pub(crate) fn into_option<T: Into<SlotNo>>(value: Option<T>) -> Option<SlotNo> {
        value.map(std::convert::Into::into)
    }
}

impl Default for SlotNo {
    /// Explicit default implementation of `SlotNo` which is `0`.
    fn default() -> Self {
        Self(0)
    }
}

impl From<SlotNo> for BigInt {
    fn from(val: SlotNo) -> Self {
        BigInt::from(val.0)
    }
}

impl TryFrom<u64> for SlotNo {
    type Error = anyhow::Error;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if !Self::is_valid(value) {
            bail!("Invalid Slot Number");
        }
        Ok(Self(value))
    }
}

impl TryFrom<i64> for SlotNo {
    type Error = anyhow::Error;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        u64::try_from(value).map(TryInto::try_into)?
    }
}

impl From<SlotNo> for u64 {
    fn from(value: SlotNo) -> Self {
        value.0
    }
}

impl From<Slot> for SlotNo {
    fn from(value: Slot) -> Self {
        Self(value.into())
    }
}

impl From<SlotNo> for Slot {
    fn from(value: SlotNo) -> Self {
        value.0.into()
    }
}
