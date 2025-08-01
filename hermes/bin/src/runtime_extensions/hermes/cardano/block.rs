//! Cardano Blockchain block implementation for WASM runtime.

use cardano_blockchain_types::{MultiEraBlock, Network, Point, Slot};
use cardano_chain_follower::{ChainFollower, Kind};

/// Get a block relative to `start` by `step`.
pub(crate) fn get_block_relative(
    network: Network, start: Option<u64>, step: i64,
) -> anyhow::Result<MultiEraBlock> {
    let handle = std::thread::spawn(move || -> anyhow::Result<MultiEraBlock> {
        let point = if let Some(start_point) = start {
            calculate_point_from_step(start_point, step)?
        // If start is None, use the tip
        } else {
            Point::TIP
        };

        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .enable_io()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to create Tokio runtime: {e}"));
            },
        };

        let block = rt
            .block_on(ChainFollower::get_block(network, point.clone()))
            .ok_or_else(|| anyhow::anyhow!("Failed to fetch block at point {point}"))?;

        Ok(block.data)
    });

    handle
        .join()
        .map_err(|e| anyhow::anyhow!("Thread panicked while getting block: {e:?}"))?
}

/// Calculate point from start and step
fn calculate_point_from_step(start: u64, step: i64) -> anyhow::Result<Point> {
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
    let handle = std::thread::spawn(move || -> anyhow::Result<(Slot, Slot)> {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .enable_io()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to create Tokio runtime: {e}"));
            },
        };

        let (immutable_tip, live_tip) = rt.block_on(ChainFollower::get_tips(network));
        Ok((immutable_tip.slot_or_default(), live_tip.slot_or_default()))
    });

    handle
        .join()
        .map_err(|e| anyhow::anyhow!("Thread panicked while getting tips: {e:?}"))?
}

/// Checks if the block at the given slot is a rollback block or not.
pub(crate) fn get_is_rollback(network: Network, slot: Slot) -> anyhow::Result<Option<bool>> {
    let handle = std::thread::spawn(move || -> anyhow::Result<Option<bool>> {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .enable_io()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to create Tokio runtime: {e}"));
            },
        };

        let point = Point::fuzzy(slot);
        let block = rt.block_on(ChainFollower::get_block(network, point));
        match block {
            // Block found
            Some(block) => Ok(Some(block.kind == Kind::Rollback)),
            // Block not found
            None => Ok(None),
        }
    });

    handle
        .join()
        .map_err(|e| anyhow::anyhow!("Thread panicked while getting block rollback: {e:?}"))?
}

#[cfg(test)]
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
