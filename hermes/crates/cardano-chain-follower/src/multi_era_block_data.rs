//! Multi Era CBOR Encoded Block Data

use std::{cmp::Ordering, fmt::Display, sync::Arc};

use ouroboros::self_referencing;
use pallas::network::miniprotocols::Point;

use crate::{error::Error, stats::stats_invalid_block, Network};

/// Self-referencing CBOR encoded data of a multi-era block.
/// Note: The fields in the original struct can not be accessed directly
/// The builder creates accessor methods which are called
/// `borrow_raw_data()` and `borrow_block()`
#[self_referencing]
#[derive(Debug)]
struct SelfReferencedMultiEraBlock {
    /// The CBOR encoded data of a multi-era block.
    raw_data: Vec<u8>,

    /// The decoded multi-era block.
    /// References the `raw_data` field.
    #[borrows(raw_data)]
    #[covariant]
    block: pallas::ledger::traverse::MultiEraBlock<'this>,
}

/// Multi-era block - inner.
#[derive(Debug)]
#[allow(dead_code)]
pub struct MultiEraBlockInner {
    /// What blockchain was the block produced on.
    chain: Network,
    /// The Point on the blockchain this block can be found.
    point: Point,
    /// The previous point on the blockchain before this block.
    /// When the current point is Genesis, so is the previous.
    previous: Point,
    /// Is the block considered immutable, or could it be effected by rollback?
    immutable: bool,
    /// The decoded multi-era block.
    data: Option<SelfReferencedMultiEraBlock>,
}

/// A special point which means we do not know the point, and its NOT the origin.
/// Used for previous point when its truly unknown.
pub(crate) const UNKNOWN_POINT: Point = Point::Specific(0, Vec::new());

/// Multi-era block.
#[derive(Clone, Debug)]
pub struct MultiEraBlock(Arc<MultiEraBlockInner>);

impl MultiEraBlock {
    /// Creates a new `MultiEraBlockData` from the given bytes.
    ///
    /// # Errors
    ///
    /// If the given bytes cannot be decoded as a multi-era block, an error is returned.
    fn new_block(
        chain: Network, raw_data: Vec<u8>, previous: &Point, immutable: bool,
    ) -> anyhow::Result<Self, Error> {
        let builder = SelfReferencedMultiEraBlockTryBuilder {
            raw_data,
            block_builder: |raw_data| -> Result<_, Error> {
                pallas::ledger::traverse::MultiEraBlock::decode(raw_data)
                    .map_err(|err| Error::Codec(err.to_string()))
            },
        };
        let self_ref_block = builder.try_build()?;
        let decoded_block = self_ref_block.borrow_block();

        let slot = decoded_block.slot();

        let point = Point::new(slot, decoded_block.hash().to_vec());

        // Validate that the Block point is valid.
        match previous.clone() {
            Point::Specific(prev_slot, prev_hash) => {
                // The ONLY validation we can do on previous slot is that its less than current
                // slot.
                if slot < prev_slot {
                    return Err(Error::Codec(
                        "Previous slot is not less than current slot".to_string(),
                    ));
                }

                // Check that the previous block hash is consistent with the block itself.
                if let Some(prev_block_hash) = decoded_block.header().previous_hash() {
                    // Special case, when the previous block is actually UNKNOWN, we can't check it.
                    if *previous != UNKNOWN_POINT && (*prev_hash != *prev_block_hash) {
                        return Err(Error::Codec(
                            "Previous Block Hash mismatch with block".to_string(),
                        ));
                    }
                } else {
                    return Err(Error::Codec(
                        "Previous Slot Hash missing from block".to_string(),
                    ));
                }
            },
            Point::Origin => {
                if decoded_block.header().previous_hash().is_some() {
                    return Err(Error::Codec(
                        "Previous block must not be Origin, for any other block than Origin"
                            .to_string(),
                    ));
                }
            },
        }

        Ok(Self(Arc::new(MultiEraBlockInner {
            chain,
            point,
            previous: previous.clone(),
            immutable,
            data: Some(self_ref_block),
        })))
    }

    /// Creates a new `MultiEraBlockData` from the given bytes.
    ///
    /// # Errors
    ///
    /// If the given bytes cannot be decoded as a multi-era block, an error is returned.
    pub fn new(
        chain: Network, raw_data: Vec<u8>, previous: &Point, immutable: bool,
    ) -> anyhow::Result<Self, Error> {
        // This lets us reliably count any bad block arising from deserialization.
        let block = MultiEraBlock::new_block(chain, raw_data, previous, immutable);
        if block.is_err() {
            stats_invalid_block(chain, immutable);
        }
        block
    }

    /// Creates a special Probing `MultiEraBlock` from a point.
    ///
    /// Probe blocks can ONLY be used to search for blocks in the Live Chain.
    /// Trying to read their data will Panic.
    pub(crate) fn probe(chain: Network, point: &Point) -> Self {
        Self(Arc::new(MultiEraBlockInner {
            chain,
            point: point.clone(),
            previous: point.clone(),
            immutable: false,
            data: None,
        }))
    }

    /// Decodes the data into a multi-era block.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn decode(&self) -> &pallas::ledger::traverse::MultiEraBlock {
        // We checked the block before, during construction, so it is safe to unwrap.
        #[allow(clippy::unwrap_used)]
        self.0.data.as_ref().unwrap().borrow_block()
    }

    /// Decodes the data into a multi-era block.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn raw(&self) -> &Vec<u8> {
        // We checked the block before, during construction, so it is safe to unwrap.
        #[allow(clippy::unwrap_used)]
        self.0.data.as_ref().unwrap().borrow_raw_data()
    }

    /// Returns the block point of this block.
    #[must_use]
    pub fn point(&self) -> Point {
        // We checked the block before, during construction, so it is safe to unwrap.
        self.0.point.clone()
    }

    /// Returns the block point of the previous block.
    #[must_use]
    pub fn previous(&self) -> Point {
        // We checked the block before, during construction, so it is safe to unwrap.
        self.0.previous.clone()
    }

    /// Is the block data immutable on-chain.
    #[must_use]
    pub fn immutable(&self) -> bool {
        // We checked the block before, during construction, so it is safe to unwrap.
        self.0.immutable
    }
}

impl Display for MultiEraBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(block_data) = self.0.data.as_ref() {
            let block = block_data.borrow_block();
            let block_number = block.number();
            let slot = block.slot();
            let size = block.size();
            let txns = block.tx_count();
            let aux_data = block.has_aux_data();

            let block_era = match block {
                pallas::ledger::traverse::MultiEraBlock::EpochBoundary(_) => {
                    "Byron Epoch Boundary".to_string()
                },
                pallas::ledger::traverse::MultiEraBlock::AlonzoCompatible(_, era) => {
                    format!("{era}")
                },
                pallas::ledger::traverse::MultiEraBlock::Babbage(_) => "Babbage".to_string(),
                pallas::ledger::traverse::MultiEraBlock::Byron(_) => "Byron".to_string(),
                pallas::ledger::traverse::MultiEraBlock::Conway(_) => "Conway".to_string(),
                _ => "Unknown".to_string(),
            };
            write!(f, "{block_era} block : Slot# {slot} : Block# {block_number} : Size {size} : Txns {txns} : AuxData? {aux_data}")?;
        } else {
            write!(f, "PROBE BLOCK @ {:?}", self.0.point)?;
        }
        Ok(())
    }
}

impl PartialEq for MultiEraBlock {
    /// Compare two `MultiEraBlock` by their current points.
    /// Ignores the Hash, we only check for equality of the Slot#.
    fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other) == Some(Ordering::Equal)
    }
}

impl Eq for MultiEraBlock {}

impl PartialOrd for MultiEraBlock {
    /// Compare two `MultiEraBlock` by their points.
    /// Only checks the Slot#.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MultiEraBlock {
    /// Compare two `LiveBlocks` by their points.
    /// Only checks the Slot#.
    fn cmp(&self, other: &Self) -> Ordering {
        cmp_point(&self.0.point, &other.0.point)
    }
}

// Allows us to compare a MultiEraBlock against a Point directly (Just the slot#).
impl PartialEq<Point> for MultiEraBlock {
    // Equality ONLY checks the Slot#
    fn eq(&self, other: &Point) -> bool {
        Some(Ordering::Equal) == self.partial_cmp(other)
    }
}

impl PartialOrd<Point> for MultiEraBlock {
    /// Compare a `MultiEraBlock` to a `Point` by their points.
    /// Only checks the Slot#.
    fn partial_cmp(&self, other: &Point) -> Option<Ordering> {
        Some(cmp_point(&self.0.point, other))
    }
}

/// Compare Points, because Pallas does not impl `Ord` for Point.
pub(crate) fn cmp_point(a: &Point, b: &Point) -> Ordering {
    match a {
        Point::Origin => match b {
            Point::Origin => Ordering::Equal,
            Point::Specific(..) => Ordering::Less,
        },
        Point::Specific(slot, _) => match b {
            Point::Origin => Ordering::Greater,
            Point::Specific(other_slot, _) => slot.cmp(other_slot),
        },
    }
}

#[cfg(test)]
mod tests {
    use pallas::network::miniprotocols::Point;

    use crate::{MultiEraBlock, Network};

    struct TestRecord {
        raw: Vec<u8>,
        previous: Point,
    }

    /// Byron Test Block data
    fn byron_block() -> Vec<u8> {
        hex::decode(include_str!("./../test_data/byron.block"))
            .expect("Failed to decode hex block.")
    }

    /// Shelley Test Block data
    fn shelley_block() -> Vec<u8> {
        hex::decode(include_str!("./../test_data/shelley.block"))
            .expect("Failed to decode hex block.")
    }

    /// Mary Test Block data
    fn mary_block() -> Vec<u8> {
        hex::decode(include_str!("./../test_data/mary.block")).expect("Failed to decode hex block.")
    }

    /// Allegra Test Block data
    fn allegra_block() -> Vec<u8> {
        hex::decode(include_str!("./../test_data/allegra.block"))
            .expect("Failed to decode hex block.")
    }

    /// Allegra Test Block data
    fn alonzo_block() -> Vec<u8> {
        hex::decode(include_str!("./../test_data/allegra.block"))
            .expect("Failed to decode hex block.")
    }

    /// An array of test blocks
    fn test_blocks() -> Vec<TestRecord> {
        vec![
            TestRecord {
                raw: byron_block(),
                previous: Point::Origin,
            },
            TestRecord {
                raw: shelley_block(),
                previous: Point::Origin,
            },
            TestRecord {
                raw: mary_block(),
                previous: Point::Origin,
            },
            TestRecord {
                raw: allegra_block(),
                previous: Point::Origin,
            },
            TestRecord {
                raw: alonzo_block(),
                previous: Point::Origin,
            },
        ]
    }

    #[test]
    fn multi_era_block_test() {
        for test_block in test_blocks() {
            let block_bytes = hex::decode(test_block.raw).expect("Failed to decode hex block.");
            let block = MultiEraBlock::new(
                Network::Preprod,
                block_bytes.clone(),
                &test_block.previous,
                false,
            )
            .expect("Failed to decode block.");
            let pallas_block =
                pallas::ledger::traverse::MultiEraBlock::decode(block_bytes.as_slice())
                    .expect("Failed to decode pallas block.");

            assert_eq!(block.decode().hash(), pallas_block.hash());
        }
    }

    // #[test]
    // #[allow(clippy::unwrap_used)]
    // fn test_comparisons() {
    // let origin1 = LiveBlock::new(Point::Origin, MultiEraBlock::new(vec![]).unwrap());
    // let origin2 = LiveBlock::new(Point::Origin, MultiEraBlock::new(vec![]).unwrap());
    // let early_block = LiveBlock::new(
    // Point::Specific(100u64, vec![]),
    // MultiEraBlock::new(vec![1, 2, 3]).unwrap(),
    // );
    // let early_block2 = LiveBlock::new(
    // Point::Specific(100u64, vec![]),
    // MultiEraBlock::new(vec![4, 5, 6]).unwrap(),
    // );
    // let late_block = LiveBlock::new(
    // Point::Specific(10000u64, vec![]),
    // MultiEraBlock::new(vec![1, 2, 3]).unwrap(),
    // );
    // let late_block2 = LiveBlock::new(
    // Point::Specific(10000u64, vec![]),
    // MultiEraBlock::new(vec![4, 5, 6]).unwrap(),
    // );
    //
    // assert!(origin1 == origin2);
    // assert!(origin2 == origin1);
    //
    // assert!(origin1 < early_block);
    // assert!(origin2 < late_block);
    //
    // assert!(origin1 <= early_block);
    // assert!(origin2 <= late_block);
    //
    // assert!(early_block > origin1);
    // assert!(late_block > origin2);
    //
    // assert!(early_block >= origin1);
    // assert!(late_block >= origin2);
    //
    // assert!(early_block < late_block);
    // assert!(late_block > early_block);
    //
    // assert!(early_block <= late_block);
    // assert!(late_block >= early_block);
    //
    // assert!(early_block == early_block2);
    // assert!(late_block == late_block2);
    //
    // assert!(origin1 != early_block);
    // assert!(origin2 != late_block);
    // assert!(early_block != late_block);
    // }
}
