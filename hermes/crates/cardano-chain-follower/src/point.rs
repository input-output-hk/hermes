//! A Cardano Point on the Blockchain.
//!
//! Wrapped version of the Pallas primitive.
//! We only use this version unless talking to Pallas.

use std::{
    cmp::Ordering,
    fmt::{Debug, Display, Formatter},
};

use pallas::crypto::hash::Hash;

/// A point in the chain or the tip.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Point(pallas::network::miniprotocols::Point);

/// A special point which means we do not know the point, and its NOT the origin.
/// Used for previous point when its truly unknown.
pub(crate) const UNKNOWN_POINT: Point = Point(pallas::network::miniprotocols::Point::Specific(
    u64::MIN,
    Vec::new(),
));

/// A special point which means we do not know the point, however it's the TIP, whatever that
/// happens to be NOW.
/// Used for Point we are interested in should be the TIP of the blockchain.
pub const TIP_POINT: Point = Point(pallas::network::miniprotocols::Point::Specific(
    u64::MAX,
    Vec::new(),
));

/// A special point which means we do not know the point, however it's the ORIGIN, whatever that
/// happens to be.
/// Used for Point we are interested in should be the ORIGIN of the blockchain.
pub const ORIGIN_POINT: Point = Point(pallas::network::miniprotocols::Point::Origin);

impl Point {
    /// Create a new specific point.
    #[must_use]
    pub fn new(slot: u64, hash: Vec<u8>) -> Self {
        Self(pallas::network::miniprotocols::Point::Specific(slot, hash))
    }

    /// Create a new specific point where hash is unknown.
    #[must_use]
    pub fn fuzzy(slot: u64) -> Self {
        Self(pallas::network::miniprotocols::Point::Specific(
            slot,
            Vec::new(),
        ))
    }

    /// Compare the Points hash with a known hash from a block.
    #[must_use]
    pub fn cmp_hash(&self, hash: &Option<Hash<32>>) -> bool {
        match hash {
            Some(cmp_hash) => match self.0 {
                pallas::network::miniprotocols::Point::Specific(_, ref hash) => {
                    **hash == **cmp_hash
                },
                pallas::network::miniprotocols::Point::Origin => false,
            },
            None => match self.0 {
                pallas::network::miniprotocols::Point::Specific(_, ref hash) => hash.is_empty(),
                pallas::network::miniprotocols::Point::Origin => true,
            },
        }
    }

    /// Get the slot, or a default if its the Origin.
    #[must_use]
    pub fn slot_or_default(&self) -> u64 {
        self.0.slot_or_default()
    }

    /// Get the slot, or a default if its the Origin.
    #[must_use]
    pub fn hash_or_default(&self) -> Vec<u8> {
        match &self.0 {
            pallas::network::miniprotocols::Point::Specific(_, hash) => hash.clone(),
            pallas::network::miniprotocols::Point::Origin => Vec::new(),
        }
    }

    /// Strict Equality.
    ///
    /// This checks BOTH the Slot# and Hash are identical.
    #[must_use]
    pub fn strict_eq(&self, b: &Self) -> bool {
        self.0 == b.0
    }
}

impl Display for Point {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        if *self == ORIGIN_POINT {
            return write!(f, "Point @ Origin");
        } else if *self == TIP_POINT {
            return write!(f, "Point @ Tip");
        } else if *self == UNKNOWN_POINT {
            return write!(f, "Point @ Unknown");
        }

        let slot = self.slot_or_default();
        let hash = self.hash_or_default();
        if hash.is_empty() {
            return write!(f, "Point @ Probe:{slot}");
        }
        write!(f, "Point @ {slot}:{}", hex::encode(hash))
    }
}

impl From<pallas::network::miniprotocols::Point> for Point {
    fn from(point: pallas::network::miniprotocols::Point) -> Self {
        Self(point)
    }
}

impl From<Point> for pallas::network::miniprotocols::Point {
    fn from(point: Point) -> pallas::network::miniprotocols::Point {
        point.0
    }
}

impl PartialOrd for Point {
    /// Compare two `LiveBlocks` by their points.
    /// Only checks the Slot#.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Point {
    /// Compare two `PointOrTips` by their points.
    /// Only checks the Slot#.
    fn cmp(&self, other: &Self) -> Ordering {
        cmp_point(&self.0, &other.0)
    }
}

// Allows us to compare a SnapshotID against u64 (Just the Immutable File Number).
impl PartialEq<u64> for Point {
    // Equality ONLY checks the Immutable File Number, not the path.
    // This is because the Filename is already the ImmutableFileNumber
    fn eq(&self, other: &u64) -> bool {
        self.0.slot_or_default() == *other
    }
}

impl PartialOrd<u64> for Point {
    // Equality ONLY checks the Immutable File Number, not the path.
    // This is because the Filename is already the ImmutableFileNumber
    fn partial_cmp(&self, other: &u64) -> Option<Ordering> {
        self.0.slot_or_default().partial_cmp(other)
    }
}

impl Default for Point {
    fn default() -> Self {
        UNKNOWN_POINT
    }
}

/// Compare Points, because Pallas does not impl `Ord` for Point.
pub(crate) fn cmp_point(
    a: &pallas::network::miniprotocols::Point, b: &pallas::network::miniprotocols::Point,
) -> Ordering {
    match a {
        pallas::network::miniprotocols::Point::Origin => match b {
            pallas::network::miniprotocols::Point::Origin => Ordering::Equal,
            pallas::network::miniprotocols::Point::Specific(..) => Ordering::Less,
        },
        pallas::network::miniprotocols::Point::Specific(slot, _) => match b {
            pallas::network::miniprotocols::Point::Origin => Ordering::Greater,
            pallas::network::miniprotocols::Point::Specific(other_slot, _) => slot.cmp(other_slot),
        },
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    use pallas::crypto::hash::Hash;

    #[test]
    fn test_create_points() {
        let point1 = Point::new(100u64, vec![]);
        let fuzzy1 = Point::fuzzy(100u64);

        assert!(point1 == fuzzy1)
    }

    #[test]
    fn test_cmp_hash_simple() {
        let origin1 = ORIGIN_POINT;
        let point1 = Point::new(100u64, vec![8; 32]);

        assert_eq!(origin1.cmp_hash(&Some(Hash::new([0; 32]))), false);
        assert_eq!(origin1.cmp_hash(&None), true);

        assert_eq!(point1.cmp_hash(&Some(Hash::new([8; 32]))), true);
        assert_eq!(point1.cmp_hash(&None), false);
    }

    #[test]
    fn test_get_hash_simple() {
        let point1 = Point::new(100u64, vec![8; 32]);

        assert_eq!(point1.hash_or_default(), vec![8; 32])
    }

    #[test]
    fn test_identical_compare() {
        let point1 = Point::new(100u64, vec![8; 32]);
        let point2 = Point::new(100u64, vec![8; 32]);
        let point3 = Point::new(999u64, vec![8; 32]);

        assert_eq!(point1.strict_eq(&point2), true);
        assert_eq!(point1.strict_eq(&point3), false);
    }

    #[test]
    fn test_comparisons() {
        let origin1 = ORIGIN_POINT;
        let origin2 = ORIGIN_POINT;
        let tip1 = TIP_POINT;
        let tip2 = TIP_POINT;
        let early_block = Point::new(100u64, vec![]);
        let late_block1 = Point::new(5000u64, vec![]);
        let late_block2 = Point::new(5000u64, vec![]);

        assert!(origin1 == origin2);
        assert!(origin1 < early_block);
        assert!(origin1 <= early_block);
        assert!(origin1 != early_block);
        assert!(origin1 < late_block1);
        assert!(origin1 <= late_block1);
        assert!(origin1 != late_block1);
        assert!(origin1 < tip1);
        assert!(origin1 <= tip1);
        assert!(origin1 != tip1);

        assert!(tip1 > origin1);
        assert!(tip1 >= origin1);
        assert!(tip1 != origin1);
        assert!(tip1 > early_block);
        assert!(tip1 >= late_block1);
        assert!(tip1 != late_block1);
        assert!(tip1 == tip2);

        assert!(early_block > origin1);
        assert!(early_block >= origin1);
        assert!(early_block != origin1);
        assert!(early_block < late_block1);
        assert!(early_block <= late_block1);
        assert!(early_block != late_block1);
        assert!(early_block < tip1);
        assert!(early_block <= tip1);
        assert!(early_block != tip1);

        assert!(late_block1 == late_block2);
        assert!(late_block1 > origin1);
        assert!(late_block1 >= origin1);
        assert!(late_block1 != origin1);
        assert!(late_block1 > early_block);
        assert!(late_block1 >= early_block);
        assert!(late_block1 != early_block);
        assert!(late_block1 < tip1);
        assert!(late_block1 <= tip1);
        assert!(late_block1 != tip1);
    }
}
