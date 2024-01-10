//! Cardano chain reader module.

use std::path::PathBuf;

use pallas::network::{facades::PeerClient, miniprotocols::Point};

use crate::{
    mithril_snapshot::MithrilSnapshot, Error, MultiEraBlockData, Network, PointOrTip, Result,
};

/// Cardano chain Reader.
pub struct Reader {
    /// Connection used by the reader to read blocks.
    client: PeerClient,
    /// Mithril snapshot, if configured.
    mithril_snapshot: Option<MithrilSnapshot>,
}

impl Reader {
    /// Connects the Reader to a producer using the node-to-node protocol.
    ///
    /// # Arguments
    ///
    /// * `address`: Address of the node to connect to.
    /// * `network`: The [Network] the client is assuming it's connecting to.
    ///
    /// # Errors
    ///
    /// Returns Err if the connection could not be established.
    pub async fn connect(
        address: &str, network: Network, mithril_snapshot_path: Option<PathBuf>,
    ) -> Result<Self> {
        let client = PeerClient::connect(address, network.into())
            .await
            .map_err(Error::Client)?;

        let mithril_snapshot_read_state = if let Some(path) = mithril_snapshot_path {
            MithrilSnapshot::from_path(path)
        } else {
            None
        };

        Ok(Self {
            client,
            mithril_snapshot: mithril_snapshot_read_state,
        })
    }

    /// Reads a single block from the chain.
    ///
    /// # Arguments
    ///
    /// * `at`: The point at which to read the block.
    ///
    /// # Errors
    ///
    /// Returns Err if the block was not found or if some communication error ocurred.
    pub async fn read_block<P>(&mut self, at: P) -> Result<MultiEraBlockData>
    where
        P: Into<PointOrTip>,
    {
        match at.into() {
            PointOrTip::Tip => {
                let point = self.resolve_tip().await?;
                self.read_block_from_network(point).await
            },

            PointOrTip::Point(point) => {
                let snapshot_res = self
                    .mithril_snapshot
                    .as_ref()
                    .and_then(|snapshot| snapshot.try_read_block(point.clone()).ok())
                    .flatten();

                match snapshot_res {
                    Some(block_data) => Ok(block_data),
                    None => self.read_block_from_network(point).await,
                }
            },
        }
    }

    /// Reads a range of blocks from the chain.
    ///
    /// # Arguments
    ///
    /// * `from`: The point at which to start reading block from.
    /// * `to`: The point up to which the blocks will be read.
    ///
    /// # Errors
    ///
    /// Returns Err if the block range was not found or if some communication error
    /// ocurred.
    pub async fn read_block_range<P>(
        &mut self, from: Point, to: P,
    ) -> Result<Vec<MultiEraBlockData>>
    where
        P: Into<PointOrTip>,
    {
        match to.into() {
            PointOrTip::Tip => {
                let to_point = self.resolve_tip().await?;
                self.read_block_range_from_network(from, to_point).await
            },
            PointOrTip::Point(to) => {
                let snapshot_res = self
                    .mithril_snapshot
                    .as_ref()
                    .and_then(|snapshot| {
                        snapshot.try_read_block_range(from.clone(), to.clone()).ok()
                    })
                    .flatten();

                match snapshot_res {
                    Some((last_point_read, mut block_data_vec)) => {
                        // If we couldn't get all the blocks from the snapshot,
                        // try fetching the remaining ones from the network.
                        tracing::debug!(
                            slot = last_point_read.slot_or_default(),
                            "Last point read"
                        );
                        if last_point_read.slot_or_default() < to.slot_or_default() {
                            let network_blocks = self
                                .read_block_range_from_network(last_point_read, to)
                                .await?;

                            // Discard 1st point as it's already been read from
                            // the snapshot
                            let mut network_blocks_iter = network_blocks.into_iter();
                            drop(network_blocks_iter.next());

                            block_data_vec.extend(network_blocks_iter);
                        }

                        Ok(block_data_vec)
                    },
                    None => self.read_block_range_from_network(from, to).await,
                }
            },
        }
    }

    /// Finds the tip point.
    #[inline]
    async fn resolve_tip(&mut self) -> Result<Point> {
        self.client
            .chainsync()
            .intersect_tip()
            .await
            .map_err(Error::Chainsync)
    }

    /// Reads a block from the network using the N2N client.
    async fn read_block_from_network(&mut self, point: Point) -> Result<MultiEraBlockData> {
        // Used in tracing
        let slot = point.slot_or_default();

        let block_data = self
            .client
            .blockfetch()
            .fetch_single(point)
            .await
            .map_err(Error::Blockfetch)?;

        tracing::debug!(slot, "Block read from n2n");
        Ok(MultiEraBlockData(block_data))
    }

    /// Reads a range of blocks from the network using the N2N client.
    async fn read_block_range_from_network(
        &mut self, from: Point, to: Point,
    ) -> Result<Vec<MultiEraBlockData>> {
        // Used in tracing
        let from_slot = from.slot_or_default();
        let to_slot = to.slot_or_default();

        let data_vec = self
            .client
            .blockfetch()
            .fetch_range((from, to))
            .await
            .map_err(Error::Blockfetch)?
            .into_iter()
            .map(MultiEraBlockData)
            .collect();

        tracing::debug!(from_slot, to_slot, "Block range read from n2n");

        Ok(data_vec)
    }
}
