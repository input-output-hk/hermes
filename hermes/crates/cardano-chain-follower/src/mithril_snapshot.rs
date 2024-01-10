use std::path::PathBuf;

use pallas::network::miniprotocols::Point;

use crate::{Error, MultiEraBlockData, Result};

pub(crate) struct MithrilSnapshot {
    pub path: PathBuf,
    pub tip: Point,
}

impl MithrilSnapshot {
    pub fn from_path(path: PathBuf) -> Option<Self> {
        tracing::debug!("Using Mithril snapshot");

        let tip = pallas::storage::hardano::immutable::get_tip(&path)
            .unwrap()
            .unwrap();
        tracing::debug!(point = ?tip, "Mithril snapshot tip");

        Some(Self { path, tip })
    }

    pub fn try_read_block(&self, point: Point) -> Result<Option<MultiEraBlockData>> {
        if !self.contains_point(&point) {
            tracing::trace!(
                slot = point.slot_or_default(),
                "Point not in Mithril snapshot"
            );
            return Ok(None);
        }

        // Used in tracing
        let point_slot = point.slot_or_default();

        let mut block_data_iter =
            pallas::storage::hardano::immutable::read_blocks_from_point(&self.path, point)
                .map_err(|_| Error::MithrilSnapshot)?;

        match block_data_iter.next() {
            Some(res) => {
                let block_data = res.map_err(|_| Error::MithrilSnapshot)?;

                tracing::trace!(slot = point_slot, "Block read from Mithril snapshot");
                Ok(Some(MultiEraBlockData(block_data)))
            },
            None => Ok(None),
        }
    }

    pub fn try_read_block_range(
        &self, from: Point, to: Point,
    ) -> Result<Option<(Point, Vec<MultiEraBlockData>)>> {
        tracing::trace!(
            from_slot = from.slot_or_default(),
            to_slot = to.slot_or_default(),
            "Trying to read block range from Mithril snapshot"
        );

        if !self.contains_point(&from) {
            tracing::trace!(
                from_slot = from.slot_or_default(),
                to_slot = to.slot_or_default(),
                "Range has no points in Mithril snapshot"
            );
            return Ok(None);
        }

        let blocks_iter =
            pallas::storage::hardano::immutable::read_blocks_from_point(&self.path, from)
                .map_err(|_| Error::MithrilSnapshot)?;

        let mut block_data_vec = Vec::new();
        for result in blocks_iter {
            let block_data = MultiEraBlockData(result.map_err(|_| Error::MithrilSnapshot)?);

            // TODO(fsgr): Should we check the hash as well?
            //             Maybe throw an error if we don't get the block we were expecting at that
            //             slot?
            if block_data.decode()?.slot() > to.slot_or_default() {
                break;
            }

            block_data_vec.push(block_data);
        }

        // Get the point from last block read.
        // Pop here to get an owned value (we'll insert it back later).
        match block_data_vec.pop() {
            Some(last_block_data) => {
                let last_block = last_block_data.decode()?;
                let last_block_point = Point::new(last_block.slot(), last_block.hash().to_vec());

                // Push the last block data back
                block_data_vec.push(last_block_data);

                tracing::trace!(
                    last_block_slot = last_block_point.slot_or_default(),
                    "Block range read from Mithril snapshot"
                );
                Ok(Some((last_block_point, block_data_vec)))
            },
            None => Ok(None),
        }
    }

    pub fn contains_point(&self, point: &Point) -> bool {
        point.slot_or_default() <= self.tip.slot_or_default()
    }
}
