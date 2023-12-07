//! Cardano chain follower.
//!
//! # Follower
//!
//! The follower can be used to follow the chain and to fetch a single block or a range of
//! blocks.
//!
//! Follower will maintain the state of the read-pointer (the point at which the chain is
//! being read by the chainsync miniprotocol).
//!
//! The read-pointer state will not be modified when fetching blocks.
//!
//! # Client
//!
//! The client can be used to fetch a single block or a range of blocks and
//! is used by the follower to follow the chain.
//!
//! # Example
//!
//! Following chain updates from the tip of the chain can be done like:
//!
//! ```rust,no_run
//! use cardano_chain_follower::{Client, Follower, Network, PointOrTip};
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut follower = Follower::new(
//!         Client::connect_n2n("node.address", Network::Preprod)
//!             .await
//!             .unwrap(),
//!     );
//!
//!     follower.set_read_pointer(PointOrTip::Tip).await.unwrap();
//!
//!     let _update = follower.next().await.unwrap();
//! }
//! ```

// TODO: remove this once we implement the API.
#![allow(dead_code, clippy::unused_async)]
// (fsgr): This should be removed. I only added it because, for some reason,
//         the tower crate is failing to compile in my machine (didn't test anywhere else)
//         if it's compiled with this flag.
#![deny(missing_docs)]

pub use pallas::network::miniprotocols::Point;
use pallas::{
    ledger::traverse::MultiEraBlock,
    network::{
        facades::{NodeClient, PeerClient},
        miniprotocols::{
            chainsync::NextResponse, MAINNET_MAGIC, PREVIEW_MAGIC, PRE_PRODUCTION_MAGIC,
            TESTNET_MAGIC,
        },
    },
};

/// Crate error type.
///
/// We are using a boxed error here until we have some implementation of the
/// the crate's API.
///
/// In the future this will probably be something as:
///
/// ```ignore
/// use thiserror::Error;
///
/// #[derive(Debug, Error)]
/// pub enum Error {
/// ...
/// }
/// ```
pub type Error = Box<dyn std::error::Error>;

/// Crate result type.
pub type Result<T> = std::result::Result<T, Error>;

/// Enum of possible types of clients used by the follower.
pub enum Client {
    /// Pallas node-to-node client.
    N2n(PeerClient),
    /// Pallas node-to-client client.
    N2c(NodeClient),
}

impl Client {
    /// Connects the follower to a producer using the node-to-node protocol.
    ///
    /// # Arguments
    ///
    /// * `address`: Address of the node to connect to.
    /// * `network`: The [Network] the client is assuming it's connecting to.
    ///
    /// # Errors
    ///
    /// Returns Err if the connection could not be estabilished.
    pub async fn connect_n2n(address: &str, network: Network) -> Result<Self> {
        let client = PeerClient::connect(address, network.into())
            .await
            .map_err(Box::new)?;

        Ok(Client::N2n(client))
    }

    /// Connects the follower to a producer using the node-to-client protocol.
    ///
    /// # Arguments
    ///
    /// * `path`: Path to the UDS to use for node communication.
    /// * `network`: The [Network] the client is assuming it's connecting to.
    ///
    /// # Errors
    ///
    /// Returns Err if the connection could not be estabilished.
    #[cfg(unix)]
    pub async fn connect_n2c<P>(path: P, network: Network) -> Result<Self>
    where P: AsRef<std::path::Path> {
        let client = NodeClient::connect(path, network.into())
            .await
            .map_err(Box::new)?;

        Ok(Client::N2c(client))
    }

    /// Fetches a single block from the producer.
    ///
    /// # Arguments
    ///
    /// * `at`: Point at which to fetch the block from.
    ///
    /// # Errors
    ///
    /// Returns Err if the block could not be fetched (e.g. producer communication
    /// failed).
    pub async fn fetch_block(&mut self, _at: Point) -> Result<MultiEraBlockData> {
        todo!()
    }

    /// Fetches a range of blocks from the producer.
    ///
    /// # Arguments
    ///
    /// * `from`: Point defining the start of the range.
    /// * `to`: Point defining the end of the range.
    ///
    /// # Errors
    ///
    /// Returns Err if the block range could not be fetched (e.g. producer communication
    /// failed).
    pub async fn fetch_block_range(
        &mut self, _from: Point, _to: Point,
    ) -> Result<Vec<MultiEraBlockData>> {
        todo!()
    }

    // (fsgr): These will be used by the Follower to get the chain updates.

    async fn set_read_pointer(&mut self, _at: PointOrTip) -> Result<Point> {
        todo!()
    }

    async fn next(&mut self) -> Result<NextResponse<MultiEraBlockData>> {
        todo!()
    }
}

/// A point in the chain or the tip.
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

/// CBOR encoded data of a multi-era block.
pub struct MultiEraBlockData(Vec<u8>);

impl MultiEraBlockData {
    /// Decodes the data into a multi-era block.
    ///
    /// # Errors
    ///
    /// Returns Err if the block's era couldn't be decided or if the encoded data is
    /// invalid.
    pub fn decode(&self) -> Result<MultiEraBlock> {
        let block = MultiEraBlock::decode(&self.0).map_err(Box::new)?;

        Ok(block)
    }
}

/// Enum of possible Cardano networks.
pub enum Network {
    /// Cardano mainnet network.
    Mainnet,
    /// Cardano pre-production network.
    Preprod,
    /// Cardano preview network.
    Preview,
    /// Cardano testnet network.
    Testnet,
}

impl From<Network> for u64 {
    fn from(network: Network) -> Self {
        match network {
            Network::Mainnet => MAINNET_MAGIC,
            Network::Preprod => PRE_PRODUCTION_MAGIC,
            Network::Preview => PREVIEW_MAGIC,
            Network::Testnet => TESTNET_MAGIC,
        }
    }
}

/// Enum of chain updates received by the follower.
pub enum ChainUpdate {
    /// New block inserted on chain.
    Block(MultiEraBlockData),
    /// Chain rollback to the given point.
    Rollback(PointOrTip),
}

/// Cardano chain follower.
pub struct Follower {
    /// Client used to get chain data.
    client: Client,
    /// Point at which the follower is reading the chain at.
    read_pointer: Point,
}

impl Follower {
    /// Creates a new follower.
    ///
    /// # Arguments
    ///
    /// * `client`: The client the follower will use.
    pub fn new(client: Client) -> Self {
        Self {
            client,
            read_pointer: Point::Origin,
        }
    }

    /// Returns the follower's client.
    #[must_use]
    pub fn client(&mut self) -> &mut Client {
        &mut self.client
    }

    /// Set the follower's chain read-pointer.
    ///
    /// # Arguments
    ///
    /// * `at`: Point at which to set the read-pointer.
    ///
    /// # Errors
    ///
    /// Returns Err if something went wrong while communicating with the producer.
    pub async fn set_read_pointer<P>(&mut self, at: P) -> Result<()>
    where P: Into<PointOrTip> {
        self.read_pointer = self.client.set_read_pointer(at.into()).await?;

        Ok(())
    }

    /// Receive the next chain update from the producer.
    ///
    /// # Errors
    ///
    /// Returns Err if any producer communication errors occurred.
    pub async fn next(&mut self) -> Result<ChainUpdate> {
        todo!()
    }
}
