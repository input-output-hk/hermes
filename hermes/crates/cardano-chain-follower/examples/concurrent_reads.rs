//! This example shows how to use the chain follower to download arbitrary blocks
//! from the chain concurrently.

use std::error::Error;

use cardano_chain_follower::{FollowerConfigBuilder, Network, Point};
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let follower = FollowerConfigBuilder::default_for(Network::Mainnet)
        .build()
        .connect()
        .await?;

    let points = vec![
        Point::Specific(
            110_908_236,
            hex::decode("ad3798a1db2b6097c71f35609399e4b2ff834f0f45939803d563bf9d660df2f2")?,
        ),
        Point::Specific(
            110_908_582,
            hex::decode("16e97a73e866280582ee1201a5e1815993978eede956af1869b0733bedc131f2")?,
        ),
    ];
    let mut point_count = points.len();

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    for p in points {
        let slot_no = p.slot_or_default();
        let r = follower.read_block(p);
        let r_tx = tx.clone();

        tokio::spawn(async move {
            tracing::info!(slot_no, "Reading block");
            let result = r.await;
            drop(r_tx.send(result));
        });
    }

    while let Some(result) = rx.recv().await {
        let block_data = result?;
        let block = block_data.decode()?;

        let total_fee = block
            .txs()
            .iter()
            .map(|tx| tx.fee().unwrap_or_default())
            .sum::<u64>();

        info!(
            "Block {} (slot {}) => total fee: {total_fee}",
            block.number(),
            block.slot()
        );

        point_count -= 1;
        if point_count == 0 {
            break;
        }
    }

    Ok(())
}
