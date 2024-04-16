use pallas::ledger::traverse::MultiEraBlock;
use tracing::info;

use super::{monitor, BenchmarkParams};

pub async fn run(params: &BenchmarkParams) -> anyhow::Result<()> {
    let monitor_task_handle = monitor::spawn_task();

    info!("Opening Mithril snapshot for reading");
    let iter = pallas_hardano::storage::immutable::read_blocks_from_point(
        &params.mithril_snapshot_path,
        pallas::network::miniprotocols::Point::Origin,
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
