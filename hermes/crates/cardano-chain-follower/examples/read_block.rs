//! This example shows how to use the chain reader to download arbitrary blocks
//! from the chain.

use std::error::Error;

use cardano_chain_follower::{Network, Point, Reader};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut reader =
        Reader::connect("relays-new.cardano-mainnet.iohk.io:3001", Network::Mainnet).await?;

    let data = reader
        .read_block(Point::Specific(
            110_908_236,
            hex::decode("ad3798a1db2b6097c71f35609399e4b2ff834f0f45939803d563bf9d660df2f2")?,
        ))
        .await?;

    let block = data.decode()?;

    let total_fee = block
        .txs()
        .iter()
        .map(|tx| tx.fee().unwrap_or_default())
        .sum::<u64>();

    println!("Total fee: {total_fee}");

    Ok(())
}
