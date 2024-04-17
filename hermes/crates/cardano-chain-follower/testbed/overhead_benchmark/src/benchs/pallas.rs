use pallas_hardano::storage::immutable::Point;
use pallas_traverse::MultiEraBlock;
use tracing::info;

use super::{monitor, snapshot_tip, BenchmarkParams};

pub async fn run(params: &BenchmarkParams) -> anyhow::Result<()> {
    info!("Locating Mithril snapshot tip");
    let mithril_snapshot_tip_data = snapshot_tip(&params.mithril_snapshot_path)?
        .ok_or(anyhow::anyhow!("Failed to find snapshot tip"))?;
    let mithril_snapshot_tip_block = MultiEraBlock::decode(&mithril_snapshot_tip_data)?;

    let monitor_task_handle = monitor::spawn_task(mithril_snapshot_tip_block.number());

    info!("Opening Mithril snapshot for reading");
    let iter = pallas_hardano::storage::immutable::read_blocks_from_point(
        &params.mithril_snapshot_path,
        Point::Origin,
    )
    .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    info!("Starting block iteration");
    for result in iter {
        let raw_block_data = result?;
        let block_data = MultiEraBlock::decode(&raw_block_data)?;

        monitor_task_handle
            .send_update(monitor::BenchmarkStats {
                blocks_read: 1,
                block_bytes_read: raw_block_data.len() as u64,
                current_block: block_data.number(),
                current_slot: block_data.slot(),
            })
            .await?;
    }

    monitor_task_handle.close().await;

    Ok(())
}
