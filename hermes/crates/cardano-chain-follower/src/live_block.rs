//! A Live Block from the blockchain.
//!
//! Live Blocks are any that are not yet Immutable.

use std::cmp::Ordering;

use pallas::network::miniprotocols::Point;

use crate::MultiEraBlockData;

/// A Live Block from the blockchain.
#[derive(Clone)]
pub(crate) struct LiveBlock {
    /// The Blocks location on the Blockchain
    pub(crate) point: Point,
    /// The data of the block itself.
    pub(crate) data: MultiEraBlockData,
}

impl LiveBlock {
    /// Create a new `LiveBlock`.
    pub fn new(point: Point, data: MultiEraBlockData) -> Self {
        Self { point, data }
    }

    /// Creates a `LiveBlock` without any data for probing
    pub(crate) fn probe(point: &Point) -> Self {
        Self {
            point: point.clone(),
            data: MultiEraBlockData::default(),
        }
    }
}

impl PartialEq for LiveBlock {
    /// Compare two `LiveBlocks` by their points.
    /// Ignores the Hash, we only check for equality of the Slot#.
    fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other) == Some(Ordering::Equal)
    }
}

impl Eq for LiveBlock {}

impl PartialOrd for LiveBlock {
    /// Compare two `LiveBlocks` by their points.
    /// Only checks the Slot#.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LiveBlock {
    /// Compare two `LiveBlocks` by their points.
    /// Only checks the Slot#.
    fn cmp(&self, other: &Self) -> Ordering {
        cmp_point(&self.point, &other.point)
    }
}

/// Compare Points, because Pallas does not impl `Ord` for Point.
pub(crate) fn cmp_point(a: &Point, b: &Point) -> Ordering {
    match a {
        Point::Origin => {
            match b {
                Point::Origin => Ordering::Equal,
                Point::Specific(..) => Ordering::Less,
            }
        },
        Point::Specific(slot, _) => {
            match b {
                Point::Origin => Ordering::Greater,
                Point::Specific(other_slot, _) => slot.cmp(other_slot),
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use pallas::network::miniprotocols::Point;

    use super::LiveBlock;
    use crate::MultiEraBlockData;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_comparisons() {
        let origin1 = LiveBlock::new(Point::Origin, MultiEraBlockData::new(vec![]).unwrap());
        let origin2 = LiveBlock::new(Point::Origin, MultiEraBlockData::new(vec![]).unwrap());
        let early_block = LiveBlock::new(
            Point::Specific(100u64, vec![]),
            MultiEraBlockData::new(vec![1, 2, 3]).unwrap(),
        );
        let early_block2 = LiveBlock::new(
            Point::Specific(100u64, vec![]),
            MultiEraBlockData::new(vec![4, 5, 6]).unwrap(),
        );
        let late_block = LiveBlock::new(
            Point::Specific(10000u64, vec![]),
            MultiEraBlockData::new(vec![1, 2, 3]).unwrap(),
        );
        let late_block2 = LiveBlock::new(
            Point::Specific(10000u64, vec![]),
            MultiEraBlockData::new(vec![4, 5, 6]).unwrap(),
        );

        assert!(origin1 == origin2);
        assert!(origin2 == origin1);

        assert!(origin1 < early_block);
        assert!(origin2 < late_block);

        assert!(origin1 <= early_block);
        assert!(origin2 <= late_block);

        assert!(early_block > origin1);
        assert!(late_block > origin2);

        assert!(early_block >= origin1);
        assert!(late_block >= origin2);

        assert!(early_block < late_block);
        assert!(late_block > early_block);

        assert!(early_block <= late_block);
        assert!(late_block >= early_block);

        assert!(early_block == early_block2);
        assert!(late_block == late_block2);

        assert!(origin1 != early_block);
        assert!(origin2 != late_block);
        assert!(early_block != late_block);
    }
}
