//! Cardano chain follower.

// TODO: remove this once we implement the API.
#![allow(dead_code, clippy::unused_async, clippy::no_effect_underscore_binding)]
// (fsgr): This should be removed. I only added it because, for some reason,
//         the tower crate is failing to compile in my machine (didn't test anywhere else)
//         if it's compiled with this flag.
#![deny(missing_docs)]

pub use pallas::network::miniprotocols::Point;
use pallas::{
    ledger::traverse::MultiEraBlock,
    network::miniprotocols::{MAINNET_MAGIC, PREVIEW_MAGIC, PRE_PRODUCTION_MAGIC, TESTNET_MAGIC},
};

/// Default [`Follower`] block buffer size.
const DEFAULT_CHAIN_UPDATE_BUFFER_SIZE: usize = 32;
/// Default [`Follower`] max await retries.
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

/// Cardano chain Reader.
pub struct Reader {}

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
    pub async fn connect(_address: &str, _network: Network) -> Result<Self> {
        todo!()
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
    pub async fn read_block<P>(&mut self, _at: P) -> Result<MultiEraBlockData>
    where P: Into<PointOrTip> {
        todo!()
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
        &mut self, _from: Point, _to: P,
    ) -> Result<Vec<MultiEraBlockData>>
    where P: Into<PointOrTip> {
        todo!()
    }
}

/// Enum of chain updates received by the follower.
pub enum ChainUpdate {
    /// New block inserted on chain.
    Block(MultiEraBlockData),
    /// Chain rollback to the given block.
    Rollback(MultiEraBlockData),
}

/// Builder used to create [`FollowerConfig`]s.
pub struct FollowerConfigBuilder {
    /// Block buffer size option.
    chain_update_buffer_size: usize,
    /// Maximum await retries option.
    max_await_retries: u32,
    /// Where to start following from.
    follow_from: PointOrTip,
}

impl Default for FollowerConfigBuilder {
    fn default() -> Self {
        Self {
            chain_update_buffer_size: DEFAULT_CHAIN_UPDATE_BUFFER_SIZE,
            max_await_retries: DEFAULT_MAX_AWAIT_RETRIES,
            follow_from: PointOrTip::Tip,
        }
    }
}

impl FollowerConfigBuilder {
    /// Sets the size of the chain updates buffer used by the [`Follower`].
    ///
    /// # Arguments
    ///
    /// * `chain_update_buffer_size`: Size of the chain updates buffer.
    #[must_use]
    pub fn chain_update_buffer_size(mut self, block_buffer_size: usize) -> Self {
        self.chain_update_buffer_size = block_buffer_size;
        self
    }

    /// Sets the maximum number of retries the [`Follower`] will execute when remote node
    /// sends an AWAIT message when the [`Follower`] is already in the "must reply"
    /// state.
    ///
    /// # Arguments
    ///
    /// * `max_await_retries`: Maximum number of retries.
    #[must_use]
    pub fn max_await_retries(mut self, max_await_retries: u32) -> Self {
        self.max_await_retries = max_await_retries;
        self
    }

    /// Sets the point at which the follower will start following from.
    ///
    /// # Arguments
    ///
    /// * `from`: Sync starting point.
    #[must_use]
    pub fn follow_from<P>(mut self, from: P) -> Self
    where P: Into<PointOrTip> {
        self.follow_from = from.into();
        self
    }

    /// Builds a [`FollowerConfig`].
    #[must_use]
    pub fn build(self) -> FollowerConfig {
        FollowerConfig {
            chain_update_buffer_size: self.chain_update_buffer_size,
            max_await_retries: self.max_await_retries,
            follow_from: self.follow_from,
        }
    }
}

/// Configuration for the Cardano chain follower.
pub struct FollowerConfig {
    /// Configured chain update buffer size.
    pub chain_update_buffer_size: usize,
    /// Configured maximum await retry count.
    pub max_await_retries: u32,
    /// Where to start following from.
    pub follow_from: PointOrTip,
}

/// Cardano chain follower.
pub struct Follower {}

impl Follower {
    /// Connects the follower to a producer using the node-to-node protocol.
    ///
    /// # Arguments
    ///
    /// * `address`: Address of the node to connect to.
    /// * `network`: The [Network] the client is assuming it's connecting to.
    /// * `config`: Follower's configuration (see [`FollowerConfigBuilder`]).
    ///
    /// # Errors
    ///
    /// Returns Err if the connection could not be established.
    pub async fn connect(
        _address: &str, _network: Network, _config: FollowerConfig,
    ) -> Result<Self> {
        todo!()
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
    where P: Into<PointOrTip> {
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

/// Validate a multi-era block.
///
/// This does not execute Plutus scripts nor validates ledger state.
/// It only checks that the block is correctly formatted for its era.
fn validate_multiera_block(_block: &MultiEraBlock) {
    // (fsgr): Not sure about hwo the validation will be done in here yet.
    todo!()
}
