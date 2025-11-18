//! Cardano Blockchain block implementation for WASM runtime.

use std::time::Duration;

use cardano_blockchain_types::{MultiEraBlock, Network, Point, Slot};
use cardano_chain_follower::{ChainFollower, Kind};

use crate::runtime_extensions::hermes::cardano::TOKIO_RUNTIME;

/// Timeout for blocking operations to prevent indefinite hangs during shutdown.
/// Set to 2 minutes to allow for slow network conditions and mithril downloads
/// while still preventing complete deadlocks.
const BLOCK_OPERATION_TIMEOUT: Duration = Duration::from_secs(120);

/// Get a block relative to `start` by `step`.
pub(crate) fn get_block_relative(
    network: Network,
    start: Option<u64>,
    step: i64,
) -> anyhow::Result<MultiEraBlock> {
    let point = if let Some(start_point) = start {
        calculate_point_from_step(start_point, step)?
    // If start is None, use the tip
    } else {
        Point::TIP
    };

    let handle = TOKIO_RUNTIME.handle();
    let block = handle
        .block_on(async {
            tokio::time::timeout(
                BLOCK_OPERATION_TIMEOUT,
                ChainFollower::get_block(network, point.clone()),
            )
            .await
        })
        .map_err(|_| anyhow::anyhow!("Timeout fetching block at point {point}"))?
        .ok_or_else(|| anyhow::anyhow!("Failed to fetch block at point {point}"))?;

    Ok(block.data)
}

/// Calculate point from start and step
fn calculate_point_from_step(
    start: u64,
    step: i64,
) -> anyhow::Result<Point> {
    let target = if step.is_negative() {
        start
            .checked_sub(step.unsigned_abs())
            .ok_or_else(|| anyhow::anyhow!("Step causes underflow"))?
    } else {
        start
            .checked_add(step.unsigned_abs())
            .ok_or_else(|| anyhow::anyhow!("Step causes overflow"))?
    };
    Ok(Point::fuzzy(target.into()))
}

/// Retrieves the current tips of the blockchain for the specified network.
pub(crate) fn get_tips(network: Network) -> anyhow::Result<(Slot, Slot)> {
    let handle = TOKIO_RUNTIME.handle();
    let (immutable_tip, live_tip) = handle
        .block_on(async {
            tokio::time::timeout(BLOCK_OPERATION_TIMEOUT, ChainFollower::get_tips(network)).await
        })
        .map_err(|_| anyhow::anyhow!("Timeout getting tips for network {network}"))?;
    Ok((immutable_tip.slot_or_default(), live_tip.slot_or_default()))
}

/// Checks if the block at the given slot is a rollback block or not.
pub(crate) fn get_is_rollback(
    network: Network,
    slot: Slot,
) -> anyhow::Result<Option<bool>> {
    let point = Point::fuzzy(slot);
    let handle = TOKIO_RUNTIME.handle();
    let block = handle
        .block_on(async {
            tokio::time::timeout(
                BLOCK_OPERATION_TIMEOUT,
                ChainFollower::get_block(network, point),
            )
            .await
        })
        .map_err(|_| anyhow::anyhow!("Timeout checking if block at slot {slot:?} is rollback"))?;
    match block {
        Some(block) => Ok(Some(block.kind == Kind::Rollback)),
        None => Ok(None),
    }
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use anyhow::Result;

    use super::*;

    #[test]
    fn test_positive_step() -> Result<()> {
        let start = 100;
        let step = 50;
        let point = calculate_point_from_step(start, step)?;
        assert_eq!(point.slot_or_default(), 150.into());
        Ok(())
    }

    #[test]
    fn test_negative_step() -> Result<()> {
        let start = 100;
        let step = -30;
        let point = calculate_point_from_step(start, step)?;
        assert_eq!(point.slot_or_default(), 70.into());
        Ok(())
    }

    #[test]
    fn test_zero_step() -> Result<()> {
        let start = 100;
        let step = 0;
        let point = calculate_point_from_step(start, step)?;
        assert_eq!(point.slot_or_default(), 100.into());
        Ok(())
    }

    #[test]
    fn test_underflow() {
        let start = 10;
        let step = -100;
        let result = calculate_point_from_step(start, step);
        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains("underflow"));
    }

    #[test]
    fn test_overflow() {
        let start = u64::MAX;
        let step = 10;
        let result = calculate_point_from_step(start, step);
        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains("overflow"));
    }
}
