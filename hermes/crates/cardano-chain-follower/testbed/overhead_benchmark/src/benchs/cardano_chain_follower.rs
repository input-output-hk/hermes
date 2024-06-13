//! Benchmark implementation that uses the chain-follower-crate to read all the
//! blocks from the specified Mithril snapshot.

use cardano_chain_follower::{ChainUpdate, Follower, FollowerConfigBuilder, Network};
use pallas_traverse::MultiEraBlock;

use super::{monitor, snapshot_tip, BenchmarkParams};

/// Executes the benchmark.
pub async fn run(params: BenchmarkParams) -> anyhow::Result<()> {
    let mithril_snapshot_tip_data = snapshot_tip(&params.mithril_snapshot_path)?
        .ok_or(anyhow::anyhow!("Failed to find snapshot tip"))?;
    let mithril_snapshot_tip_block = MultiEraBlock::decode(&mithril_snapshot_tip_data)?;

    let monitor_task_handle = monitor::spawn_task(mithril_snapshot_tip_block.number());

    let config = FollowerConfigBuilder::default()
        .follow_from(cardano_chain_follower::PointOrTip::Point(
            cardano_chain_follower::Point::Origin,
        ))
        .mithril_snapshot_path(params.mithril_snapshot_path)
        .build();

    let mut follower = Follower::connect(
        "relays-new.cardano-mainnet.iohk.io:3001",
        Network::Mainnet,
        config,
    )
    .await?;

    loop {
        let update = follower.next().await?;

        match update {
            ChainUpdate::ImmutableBlockRollback(data)
            | ChainUpdate::BlockTip(data)
            | ChainUpdate::ImmutableBlock(data)
            | ChainUpdate::Block(raw_block_data) => {
                let block_data = raw_block_data.decode()?;

                monitor_task_handle.send_update(monitor::BenchmarkStats {
                    blocks_read: 1,
                    block_bytes_read: raw_block_data.as_ref().len() as u64,
                    current_block: block_data.number(),
                    current_slot: block_data.slot(),
                })?;

                if block_data.number() >= mithril_snapshot_tip_block.number() {
                    break;
                }
            },
            ChainUpdate::Rollback(_) => {
                anyhow::bail!("Unexpected rollback: benchmark should not receive rollback events");
            },
        }
    }

    monitor_task_handle.close().await;

    Ok(())
}
