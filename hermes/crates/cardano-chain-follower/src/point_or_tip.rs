//! A Cardano Point on the Blockchain, or Tip.

use std::cmp::Ordering;

pub use pallas::network::miniprotocols::Point;

use crate::multi_era_block_data::cmp_point;

/// A point in the chain or the tip.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum PointOrTip {
    /// Represents a specific point of the chain.
    Point(Point),
    /// Represents the tip of the chain.
    Tip,
}

impl From<Point> for PointOrTip {
    fn from(point: Point) -> Self {
        Self::Point(point)
    }
}

impl PartialOrd for PointOrTip {
    /// Compare two `LiveBlocks` by their points.
    /// Only checks the Slot#.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// Allows us to compare a PointOrTip against a Point directly (Just the slot#).
impl PartialEq<Point> for PointOrTip {
    // Equality ONLY checks the Slot#
    fn eq(&self, other: &Point) -> bool {
        Some(Ordering::Equal) == self.partial_cmp(other)
    }
}

impl PartialOrd<Point> for PointOrTip {
    /// Compare a `PointOrTip` to a `Point` by their points.
    /// Only checks the Slot#.
    fn partial_cmp(&self, other: &Point) -> Option<Ordering> {
        self.partial_cmp(&PointOrTip::Point(other.clone()))
    }
}

impl Ord for PointOrTip {
    /// Compare two `PointOrTips` by their points.
    /// Only checks the Slot#.
    fn cmp(&self, other: &Self) -> Ordering {
        match self {
            Self::Point(a) => {
                match other {
                    Self::Point(b) => cmp_point(a, b),
                    Self::Tip => Ordering::Less,
                }
            },
            Self::Tip => {
                match other {
                    Self::Point(_) => Ordering::Greater,
                    Self::Tip => Ordering::Equal,
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use pallas::network::miniprotocols::Point;

    use super::PointOrTip;

    #[test]
    fn test_comparisons() {
        let origin = PointOrTip::Point(Point::Origin);
        let origin2 = PointOrTip::Point(Point::Origin);
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
}
