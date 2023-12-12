//! Cardano chain follower.
//!
//! # Example
//!
//! Following chain updates from the tip of the chain can be done like:
//!
//! ```rust,no_run
//! use cardano_chain_follower::{Follower, Network, PointOrTip};
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut follower = Follower::connect_n2n("node.address", Network::Preprod)
//!         .await
//!         .unwrap();
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
        facades::PeerClient,
        miniprotocols::{MAINNET_MAGIC, PREVIEW_MAGIC, PRE_PRODUCTION_MAGIC, TESTNET_MAGIC},
    },
};

const DEFAULT_BLOCK_BUFFER_SIZE: usize = 32;
const DEFAULT_MAX_AWAIT_RETRIES: u32 = 3;

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
    /// Chain rollback to the given block.
    Rollback(MultiEraBlockData),
}

/// Builder used to create [Config]s.
#[derive(Default)]
pub struct ConfigBuilder {
    block_buffer_size: Option<usize>,
    max_await_retries: Option<u32>,
}

impl ConfigBuilder {
    /// Sets the size of the block buffer used by the [Follower].
    ///
    /// # Arguments
    ///
    /// * `block_buffer_size`: Size of the block buffer.
    pub fn with_block_buffer_size(mut self, block_buffer_size: usize) -> Self {
        self.block_buffer_size = Some(block_buffer_size);
        self
    }

    /// Sets the maximum number of retries the [Follower] will execute when remote node sends
    /// an AWAIT message when the [Follower] is already in the MUST_REPLY state.
    ///
    /// # Argument
    ///
    /// * `max_await_retries`: Maxium number of retries.
    pub fn with_max_await_retries(mut self, max_await_retries: u32) -> Self {
        self.max_await_retries = Some(max_await_retries);
        self
    }

    /// Builds a [Config].
    pub fn build(self) -> Config {
        Config {
            block_buffer_size: self.block_buffer_size.unwrap_or(DEFAULT_BLOCK_BUFFER_SIZE),
            max_await_retries: self.max_await_retries.unwrap_or(DEFAULT_MAX_AWAIT_RETRIES),
        }
    }
}

/// Configuration for the Cardano chain follower.
pub struct Config {
    block_buffer_size: usize,
    max_await_retries: u32,
}

/// Cardano chain follower.
pub struct Follower {
    /// Client used to get chain data using node-to-node protocol.
    n2n_client: PeerClient,
    /// Follower configuration.
    config: Config,
}

impl Follower {
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
    pub async fn connect(address: &str, network: Network, config: Config) -> Result<Self> {
        let n2n_client = PeerClient::connect(address, network.into())
            .await
            .map_err(Box::new)?;

        Ok(Self { n2n_client, config })
    }

    /// Set the follower's chain read-pointer. Returns None if the point was
    /// not found on the chain.
    ///
    /// # Arguments
    ///
    /// * `at`: Point at which to set the read-pointer.
    ///
    /// # Errors
    ///
    /// Returns Err if something went wrong while communicating with the producer.
    pub async fn set_read_pointer<P>(&mut self, _at: P) -> Result<Option<Point>>
    where
        P: Into<PointOrTip>,
    {
        todo!()
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

/// Validate a multiera block.
///
/// This does not execute Plutus scripts nor validates ledger state.
/// It only checks that the block is correctly formatted for its era.
fn validate_multiera_block(_block: &MultiEraBlock) {
    // (fsgr): Not sure about hwo the validation will be done in here yet.
    todo!()
}
