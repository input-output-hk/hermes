//! Value of a Cardano Native Asset.

use std::fmt::Display;

/// Value of a Cardano Native Asset (may not be zero)
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) struct AssetValue(i128);

impl Display for AssetValue {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AssetValue {
    /// Performs saturating addition.
    pub(crate) fn saturating_add(
        &self,
        v: &Self,
    ) -> Self {
        self.0.checked_add(v.0).map_or_else(
            || {
                tracing::error!("Asset value overflow: {self} + {v}",);
                Self(i128::MAX)
            },
            Self,
        )
    }
}

// Really no need for this to be fallible.
// Its not possible for it to be outside the range of an i128, and if it is.
// Just saturate.
impl From<&num_bigint::BigInt> for AssetValue {
    fn from(value: &num_bigint::BigInt) -> Self {
        let sign = value.sign();
        match TryInto::<i128>::try_into(value) {
            Ok(v) => Self(v),
            Err(_) => match sign {
                num_bigint::Sign::Minus => Self(i128::MIN),
                _ => Self(i128::MAX),
            },
        }
    }
}
