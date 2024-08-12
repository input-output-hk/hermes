//! Multi Era CBOR Encoded Block Data
//!
//! Data about how the block/transactions can be encoded is found here:
//! <https://github.com/IntersectMBO/cardano-ledger/blob/78b32d585fd4a0340fb2b184959fb0d46f32c8d2/eras/conway/impl/cddl-files/conway.cddl>
//!
//! DO NOT USE the documentation/cddl definitions from the head of this repo because it currently
//! lacks most of the documentation needed to understand the format and is also incorrectly generated
//! and contains errors that will be difficult to discern.

use std::{cmp::Ordering, fmt::Display, sync::Arc};

use ouroboros::self_referencing;
use tracing::debug;

use crate::{
    error::Error,
    metadata,
    point::{ORIGIN_POINT, UNKNOWN_POINT},
    stats::stats_invalid_block,
    witness::TxWitness,
    Network, Point,
};

/// Self-referencing CBOR encoded data of a multi-era block.
/// Note: The fields in the original struct can not be accessed directly
/// The builder creates accessor methods which are called
/// `borrow_raw_data()` and `borrow_block()`
#[self_referencing]
#[derive(Debug)]
pub(crate) struct SelfReferencedMultiEraBlock {
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
    pub chain: Network,
    /// The Point on the blockchain this block can be found.
    point: Point,
    /// The previous point on the blockchain before this block.
    /// When the current point is Genesis, so is the previous.
    previous: Point,
    /// The decoded multi-era block.
    data: SelfReferencedMultiEraBlock,
    /// Decoded Metadata in the transactions in the block.
    metadata: metadata::DecodedTransaction,
    /// A map of public key hashes to the public key and transaction numbers they are in.
    #[allow(dead_code)]
    witness_map: Option<TxWitness>,
}

/// Multi-era block.
#[derive(Clone, Debug)]
pub struct MultiEraBlock {
    /// What fork is the block on?
    /// This is NOT part of the inner block, because it is not to be protected by the Arc.
    /// It can change at any time due to rollbacks detected on the live-chain.
    /// This means that any holder of a `MultiEraBlock` will have the actual fork their
    /// block was on when they read it, the live-chain code can modify the actual fork
    /// count at any time without that impacting consumers processing the data.
    /// The fork count itself is used so an asynchronous follower can properly work out
    /// how far to roll back on the live-chain in order to resynchronize, without
    /// keeping a full state of processed blocks.
    /// Followers, simply need to step backwards on the live chain until they find the
    /// previous block they followed, or reach a fork that is <= the fork of the
    /// previous block they followed. They can then safely re-follow from that earlier
    /// point, with full integrity. fork is 0 on any immutable block.
    /// It starts at 1 for live blocks, and is only incremented if the live-chain tip is
    /// purged because of a detected fork based on data received from the peer node.
    /// It does NOT count the strict number of forks reported by the peer node.
    fork: u64,
    /// The Immutable decoded data about the block itself.
    inner: Arc<MultiEraBlockInner>,
}

impl MultiEraBlock {
    /// Creates a new `MultiEraBlockData` from the given bytes.
    ///
    /// # Errors
    ///
    /// If the given bytes cannot be decoded as a multi-era block, an error is returned.
    fn new_block(
        chain: Network, raw_data: Vec<u8>, previous: &Point, fork: u64,
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

        let witness_map = TxWitness::new(&decoded_block.txs()).ok();

        let slot = decoded_block.slot();

        let point = Point::new(slot, decoded_block.hash().to_vec());

        // Validate that the Block point is valid.
        if *previous == ORIGIN_POINT {
            if decoded_block.header().previous_hash().is_some() {
                // or forcibly capture the backtrace regardless of environment variable
                // configuration
                debug!(
                    "Bad Previous Block: {}",
                    std::backtrace::Backtrace::force_capture()
                );

                return Err(Error::Codec(
                    "Previous block must not be Origin, for any other block than Origin"
                        .to_string(),
                ));
            }
        } else {
            if *previous >= slot {
                return Err(Error::Codec(format!(
                    "Previous slot is not less than current slot:{slot}"
                )));
            }

            // Special case, when the previous block is actually UNKNOWN, we can't check it.
            if *previous != UNKNOWN_POINT
                // Otherwise, we make sure the hash chain is intact
                && !previous.cmp_hash(&decoded_block.header().previous_hash())
            {
                debug!("{}, {:?}", previous, decoded_block.header().previous_hash());

                return Err(Error::Codec(
                    "Previous Block Hash mismatch with block".to_string(),
                ));
            }
        }

        let metadata = metadata::DecodedTransaction::new(decoded_block);

        Ok(Self {
            fork,
            inner: Arc::new(MultiEraBlockInner {
                chain,
                point,
                previous: previous.clone(),
                data: self_ref_block,
                metadata,
                witness_map,
            }),
        })
    }

    /// Creates a new `MultiEraBlockData` from the given bytes.
    ///
    /// # Errors
    ///
    /// If the given bytes cannot be decoded as a multi-era block, an error is returned.
    pub fn new(
        chain: Network, raw_data: Vec<u8>, previous: &Point, fork: u64,
    ) -> anyhow::Result<Self, Error> {
        // This lets us reliably count any bad block arising from deserialization.
        let block = MultiEraBlock::new_block(chain, raw_data, previous, fork);
        if block.is_err() {
            stats_invalid_block(chain, fork == 0);
        }
        block
    }

    /// Remake the block on a new fork.
    pub fn set_fork(&mut self, fork: u64) {
        self.fork = fork;
    }

    /// Decodes the data into a multi-era block.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn decode(&self) -> &pallas::ledger::traverse::MultiEraBlock {
        self.inner.data.borrow_block()
    }

    /// Decodes the data into a multi-era block.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn raw(&self) -> &Vec<u8> {
        self.inner.data.borrow_raw_data()
    }

    /// Returns the block point of this block.
    #[must_use]
    pub fn point(&self) -> Point {
        self.inner.point.clone()
    }

    /// Returns the block point of the previous block.
    #[must_use]
    pub fn previous(&self) -> Point {
        self.inner.previous.clone()
    }

    /// Is the block data immutable on-chain.
    #[must_use]
    pub fn immutable(&self) -> bool {
        self.fork == 0
    }

    /// Is the block data immutable on-chain.
    #[must_use]
    pub fn fork(&self) -> u64 {
        self.fork
    }

    /// What chain was the block from
    #[must_use]
    pub fn chain(&self) -> Network {
        self.inner.chain
    }

    /// Returns the witness map for the block.
    #[allow(dead_code)]
    pub(crate) fn witness_map(&self) -> Option<&TxWitness> {
        self.inner.witness_map.as_ref()
    }
}

impl Display for MultiEraBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fork = self.fork;
        let block_data = &self.inner.data;
        let block = block_data.borrow_block();
        let block_number = block.number();
        let slot = block.slot();
        let size = block.size();
        let txns = block.tx_count();
        let aux_data = block.has_aux_data();

        let fork = if self.immutable() {
            "Immutable".to_string()
        } else {
            format!("Fork: {fork}")
        };

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
        write!(f, "{block_era} block : {}, Previous {} : Slot# {slot} : {fork} : Block# {block_number} : Size {size} : Txns {txns} : AuxData? {aux_data}",
    self.point(), self.previous())?;
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
        self.inner.point.cmp(&other.inner.point)
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
        Some(self.inner.point.cmp(other))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::ops::Add;

    use anyhow::Ok;

    use crate::{point::ORIGIN_POINT, MultiEraBlock, Network, Point};

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

    /// Alonzo Test Block data
    pub(crate) fn alonzo_block() -> Vec<u8> {
        hex::decode(include_str!("./../test_data/allegra.block"))
            .expect("Failed to decode hex block.")
    }

    /// Babbage Test Block data
    pub(crate) fn babbage_block() -> Vec<u8> {
        hex::decode(include_str!("./../test_data/babbage.block"))
            .expect("Failed to decode hex block.")
    }

    /// An array of test blocks
    fn test_blocks() -> Vec<TestRecord> {
        vec![
            TestRecord {
                raw: byron_block(),
                previous: ORIGIN_POINT,
            },
            TestRecord {
                raw: shelley_block(),
                previous: ORIGIN_POINT,
            },
            TestRecord {
                raw: mary_block(),
                previous: ORIGIN_POINT,
            },
            TestRecord {
                raw: allegra_block(),
                previous: ORIGIN_POINT,
            },
            TestRecord {
                raw: alonzo_block(),
                previous: ORIGIN_POINT,
            },
        ]
    }

    // Gets sorted by slot number from highest to lowest
    fn sorted_test_blocks() -> Vec<Vec<u8>> {
        vec![
            mary_block(),    // 27388606
            allegra_block(), // 18748707
            alonzo_block(),  // 18748707
            shelley_block(), // 7948610
            byron_block(),   // 3241381
        ]
    }

    /// Previous Point slot is >= blocks point, but hash is correct (should fail)
    #[test]
    fn test_multi_era_block_point_compare_1() -> anyhow::Result<()> {
        for (i, test_block) in test_blocks().into_iter().enumerate() {
            let pallas_block =
                pallas::ledger::traverse::MultiEraBlock::decode(test_block.raw.as_slice())?;

            let previous_point = Point::new(
                pallas_block.slot().add(i as u64),
                pallas_block
                    .header()
                    .previous_hash()
                    .expect("cannot get previous hash")
                    .to_vec(),
            );

            let block =
                MultiEraBlock::new(Network::Preprod, test_block.raw.clone(), &previous_point, 1);

            assert!(block.is_err());
        }

        Ok(())
    }

    /// Previous Point slot is < blocks point, but hash is different. (should fail).
    #[test]
    fn test_multi_era_block_point_compare_2() -> anyhow::Result<()> {
        for test_block in test_blocks() {
            let pallas_block =
                pallas::ledger::traverse::MultiEraBlock::decode(test_block.raw.as_slice())?;

            let previous_point = Point::new(pallas_block.slot() - 1, vec![0; 32]);

            let block =
                MultiEraBlock::new(Network::Preprod, test_block.raw.clone(), &previous_point, 1);

            assert!(block.is_err());
        }

        Ok(())
    }

    /// Previous Point slot is < blocks point, and hash is also correct. (should pass).
    #[test]
    fn test_multi_era_block_point_compare_3() -> anyhow::Result<()> {
        for test_block in test_blocks() {
            let pallas_block =
                pallas::ledger::traverse::MultiEraBlock::decode(test_block.raw.as_slice())?;

            let previous_point = Point::new(
                pallas_block.slot() - 1,
                pallas_block
                    .header()
                    .previous_hash()
                    .expect("cannot get previous hash")
                    .to_vec(),
            );

            let block =
                MultiEraBlock::new(Network::Preprod, test_block.raw.clone(), &previous_point, 1)?;

            assert_eq!(block.decode().hash(), pallas_block.hash());
        }

        Ok(())
    }

    fn mk_test_blocks() -> Vec<MultiEraBlock> {
        let raw_blocks = sorted_test_blocks();
        raw_blocks
            .iter()
            .map(|block| {
                let prev_point = pallas::ledger::traverse::MultiEraBlock::decode(block.as_slice())
                    .map(|block| {
                        Point::new(
                            block.slot() - 1,
                            block
                                .header()
                                .previous_hash()
                                .expect("cannot get previous hash")
                                .to_vec(),
                        )
                    })
                    .expect("cannot create point");

                MultiEraBlock::new(Network::Preprod, block.clone(), &prev_point, 1)
                    .expect("cannot create multi-era block")
            })
            .collect()
    }

    fn mk_test_points() -> Vec<Point> {
        let raw_blocks = sorted_test_blocks();
        raw_blocks
            .iter()
            .map(|block| {
                pallas::ledger::traverse::MultiEraBlock::decode(block.as_slice())
                    .map(|block| {
                        Point::new(
                            block.slot(),
                            block
                                .header()
                                .previous_hash()
                                .expect("cannot get previous hash")
                                .to_vec(),
                        )
                    })
                    .expect("cannot create point")
            })
            .collect()
    }

    /// Compares between blocks using comparison operators
    #[test]
    fn test_multi_era_block_point_compare_4() -> anyhow::Result<()> {
        let multi_era_blocks = mk_test_blocks();

        let mary_block = multi_era_blocks.first().expect("cannot get block");
        let allegra_block = multi_era_blocks.get(1).expect("cannot get block");
        let alonzo_block = multi_era_blocks.get(2).expect("cannot get block");
        let shelley_block = multi_era_blocks.get(3).expect("cannot get block");
        let byron_block = multi_era_blocks.get(4).expect("cannot get block");

        assert!(mary_block > allegra_block);
        assert!(mary_block >= allegra_block);
        assert!(mary_block != allegra_block);
        assert!(mary_block > alonzo_block);
        assert!(mary_block >= alonzo_block);
        assert!(mary_block != alonzo_block);
        assert!(mary_block > shelley_block);
        assert!(mary_block >= shelley_block);
        assert!(mary_block != shelley_block);
        assert!(mary_block > byron_block);
        assert!(mary_block >= byron_block);

        assert!(allegra_block < mary_block);
        assert!(allegra_block <= mary_block);
        assert!(allegra_block != mary_block);
        assert!(allegra_block == alonzo_block);
        assert!(allegra_block >= alonzo_block);
        assert!(allegra_block <= alonzo_block);
        assert!(allegra_block > shelley_block);
        assert!(allegra_block >= shelley_block);
        assert!(allegra_block != shelley_block);
        assert!(allegra_block > byron_block);
        assert!(allegra_block >= byron_block);
        assert!(allegra_block != byron_block);

        assert!(alonzo_block < mary_block);
        assert!(alonzo_block <= mary_block);
        assert!(alonzo_block != mary_block);
        assert!(alonzo_block == allegra_block);
        assert!(alonzo_block >= allegra_block);
        assert!(alonzo_block <= allegra_block);
        assert!(alonzo_block > shelley_block);
        assert!(alonzo_block >= shelley_block);
        assert!(alonzo_block != shelley_block);
        assert!(alonzo_block > byron_block);
        assert!(alonzo_block >= byron_block);
        assert!(alonzo_block != byron_block);

        assert!(shelley_block < mary_block);
        assert!(shelley_block <= mary_block);
        assert!(shelley_block != mary_block);
        assert!(shelley_block < allegra_block);
        assert!(shelley_block <= allegra_block);
        assert!(shelley_block != allegra_block);
        assert!(shelley_block < alonzo_block);
        assert!(shelley_block <= alonzo_block);
        assert!(shelley_block != alonzo_block);
        assert!(shelley_block > byron_block);
        assert!(shelley_block >= byron_block);
        assert!(shelley_block != byron_block);

        assert!(byron_block < mary_block);
        assert!(byron_block <= mary_block);
        assert!(byron_block != mary_block);
        assert!(byron_block < allegra_block);
        assert!(byron_block <= allegra_block);
        assert!(byron_block != allegra_block);
        assert!(byron_block < alonzo_block);
        assert!(byron_block <= alonzo_block);
        assert!(byron_block != alonzo_block);
        assert!(byron_block < shelley_block);
        assert!(byron_block <= shelley_block);
        assert!(byron_block != shelley_block);

        Ok(())
    }

    /// Compares between blocks and points using comparison operators
    #[test]
    fn test_multi_era_block_point_compare_5() -> anyhow::Result<()> {
        let points = mk_test_points();
        let blocks = mk_test_blocks();

        let mary_block = blocks.first().expect("cannot get block");
        let allegra_block = blocks.get(1).expect("cannot get block");
        let alonzo_block = blocks.get(2).expect("cannot get block");
        let shelley_block = blocks.get(3).expect("cannot get block");
        let byron_block = blocks.get(4).expect("cannot get block");

        let mary_point = points.first().expect("cannot get point");
        let allegra_point = points.get(1).expect("cannot get point");
        let alonzo_point = points.get(2).expect("cannot get point");
        let shelley_point = points.get(3).expect("cannot get point");
        let byron_point = points.get(4).expect("cannot get point");

        assert!(mary_block > allegra_point);
        assert!(mary_block >= allegra_point);
        assert!(mary_block != allegra_point);
        assert!(mary_block > alonzo_point);
        assert!(mary_block >= alonzo_point);
        assert!(mary_block != alonzo_point);
        assert!(mary_block > shelley_point);
        assert!(mary_block >= shelley_point);
        assert!(mary_block != shelley_point);
        assert!(mary_block > byron_point);
        assert!(mary_block >= byron_point);

        assert!(allegra_block < mary_point);
        assert!(allegra_block <= mary_point);
        assert!(allegra_block != mary_point);
        assert!(allegra_block == alonzo_point);
        assert!(allegra_block >= alonzo_point);
        assert!(allegra_block <= alonzo_point);
        assert!(allegra_block > shelley_point);
        assert!(allegra_block >= shelley_point);
        assert!(allegra_block != shelley_point);
        assert!(allegra_block > byron_point);
        assert!(allegra_block >= byron_point);
        assert!(allegra_block != byron_point);

        assert!(alonzo_block < mary_point);
        assert!(alonzo_block <= mary_point);
        assert!(alonzo_block != mary_point);
        assert!(alonzo_block == allegra_point);
        assert!(alonzo_block >= allegra_point);
        assert!(alonzo_block <= allegra_point);
        assert!(alonzo_block > shelley_point);
        assert!(alonzo_block >= shelley_point);
        assert!(alonzo_block != shelley_point);
        assert!(alonzo_block > byron_point);
        assert!(alonzo_block >= byron_point);
        assert!(alonzo_block != byron_point);

        assert!(shelley_block < mary_point);
        assert!(shelley_block <= mary_point);
        assert!(shelley_block != mary_point);
        assert!(shelley_block < allegra_point);
        assert!(shelley_block <= allegra_point);
        assert!(shelley_block != allegra_point);
        assert!(shelley_block < alonzo_point);
        assert!(shelley_block <= alonzo_point);
        assert!(shelley_block != alonzo_point);
        assert!(shelley_block > byron_point);
        assert!(shelley_block >= byron_point);
        assert!(shelley_block != byron_point);

        assert!(byron_block < mary_point);
        assert!(byron_block <= mary_point);
        assert!(byron_block != mary_point);
        assert!(byron_block < allegra_point);
        assert!(byron_block <= allegra_point);
        assert!(byron_block != allegra_point);
        assert!(byron_block < alonzo_point);
        assert!(byron_block <= alonzo_point);
        assert!(byron_block != alonzo_point);
        assert!(byron_block < shelley_point);
        assert!(byron_block <= shelley_point);
        assert!(byron_block != shelley_point);

        Ok(())
    }

    #[test]
    fn test_multi_era_block_with_origin_point() {
        for test_block in test_blocks() {
            let block = MultiEraBlock::new(
                Network::Preprod,
                test_block.raw.clone(),
                &test_block.previous,
                1,
            );

            assert!(block.is_err());
        }
    }
}
