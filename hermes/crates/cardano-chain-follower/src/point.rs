//! A Cardano Point on the Blockchain.
//!
//! Wrapped version of the Pallas primitive.
//! We only use this version unless talking to Pallas.

use std::cmp::Ordering;

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

    /// Create a new specific point.
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
    /* TODO: Fix this
    use pallas::network::miniprotocols::Point;

    use super::Point;

    #[test]
    fn test_comparisons() {
        let origin = Point::Point(Point::Origin);
        let origin2 = Point::Point(Point::Origin);
        let tip = PointOrTip::Tip;
        let tip2 = PointOrTip::Tip;
        let early_block = PointOrTip::Point(Point::Specific(100u64, vec![]));
        let late_block = PointOrTip::Point(Point::Specific(5000u64, vec![]));
        let late_block2 = PointOrTip::Point(Point::Specific(5000u64, vec![]));

        assert!(origin == origin2);
        assert!(origin < early_block);
        assert!(origin <= early_block);
        assert!(origin != early_block);
        assert!(origin < late_block);
        assert!(origin <= late_block);
        assert!(origin != late_block);
        assert!(origin < tip);
        assert!(origin <= tip);
        assert!(origin != tip);

        assert!(tip > origin);
        assert!(tip >= origin);
        assert!(tip != origin);
        assert!(tip > early_block);
        assert!(tip >= late_block);
        assert!(tip != late_block);
        assert!(tip == tip2);

        assert!(early_block > origin);
        assert!(early_block >= origin);
        assert!(early_block != origin);
        assert!(early_block < late_block);
        assert!(early_block <= late_block);
        assert!(early_block != late_block);
        assert!(early_block < tip);
        assert!(early_block <= tip);
        assert!(early_block != tip);

        assert!(late_block == late_block2);
        assert!(late_block > origin);
        assert!(late_block >= origin);
        assert!(late_block != origin);
        assert!(late_block > early_block);
        assert!(late_block >= early_block);
        assert!(late_block != early_block);
        assert!(late_block < tip);
        assert!(late_block <= tip);
        assert!(late_block != tip);
    }
    */
}
