use cardano_chain_follower::{ChainUpdate, Follower, FollowerConfigBuilder, Network};
use tracing::info;

use super::{monitor, BenchmarkParams};

pub async fn run(params: BenchmarkParams) -> anyhow::Result<()> {
    info!("Locating Mithril snapshot tip");
    let mithril_snapshot_tip =
        pallas_hardano::storage::immutable::get_tip(&params.mithril_snapshot_path)
            .map_err(|e| anyhow::anyhow!("Get tip error: {:?}", e))?
            .ok_or(anyhow::anyhow!("Failed to get Mithril snapshot tip"))?;

    let monitor_task_handle = monitor::spawn_task();

    info!("Starting chain follower");
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

    info!("Starting block iteration");
    loop {
        let update = follower.next().await?;

        match update {
            ChainUpdate::Block(raw_block_data) => {
                let block_data = raw_block_data.decode()?;
                let current_slot = block_data.slot();

                monitor_task_handle
                    .send_update(monitor::BenchmarkStats {
                        blocks_read: 1,
                        block_bytes_read: raw_block_data.as_ref().len() as u64,
                        current_block: block_data.number(),
                        current_slot,
                    })
                    .await?;

                if current_slot >= mithril_snapshot_tip.slot_or_default() {
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
