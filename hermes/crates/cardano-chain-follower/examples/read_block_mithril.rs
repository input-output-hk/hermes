//! This example shows how to use the chain reader to read arbitrary blocks
//! from Mithril snapshot files.

use std::{error::Error, path::PathBuf};

use cardano_chain_follower::{Network, Point, Reader};
use tracing::level_filters::LevelFilter;
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

    let mut reader = Reader::connect(
        "preprod-node.play.dev.cardano.org:3001",
        Network::Preprod,
        Some(
            PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
                .join("examples/snapshot_data"),
        ),
    )
    .await?;

    let data = reader
        .read_block(Point::Specific(
            49_075_418,
            hex::decode("bdb5ce7788850c30342794f252b1d955086862e8f7cb90a32a8f560b693ca78a")?,
        ))
        .await?;

    let block = data.decode()?;

    let total_fee = block
        .txs()
        .iter()
        .map(|tx| tx.fee().unwrap_or_default())
        .sum::<u64>();

    println!("Block number: {}", block.number());
    println!("Total fee: {total_fee}");

    Ok(())
}
