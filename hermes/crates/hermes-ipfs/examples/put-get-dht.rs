//! Use Hermes IPFS to distribute content using DHT
#![allow(clippy::println_empty_string)]

use hermes_ipfs::HermesIpfs;

#[allow(clippy::indexing_slicing)]
/// Connect Node A, upload file and provide CID by adding to DHT
async fn start_bootstrapped_nodes() -> anyhow::Result<(HermesIpfs, HermesIpfs)> {
    let hermes_a = HermesIpfs::start().await?;
    println!("***************************************");
    println!("* Hermes IPFS node A has started.");
    let peer_id_a = hermes_a.identity(None).await?;
    println!("    Peer ID: {peer_id_a}");
    let addresses = hermes_a.listening_addresses().await?;
    let a_address = addresses[0].clone();
    let a_p2p = a_address.with(rust_ipfs::Protocol::P2p(peer_id_a));
    println!("    P2P addr: {a_p2p}");
    println!("***************************************");
    println!("* Hermes IPFS node B has started.");
    let hermes_b = HermesIpfs::start().await?;
    let peer_id_b = hermes_b.identity(None).await?;
    println!("    Peer ID: {peer_id_b}");
    let addresses = hermes_b.listening_addresses().await?;
    let b_address = addresses[0].clone();
    let b_p2p = b_address.with(rust_ipfs::Protocol::P2p(peer_id_b));
    println!("    P2P addr: {b_p2p}");
    println!("***************************************");
    println!("* Bootstrapping node A.");
    hermes_a.dht_mode(rust_ipfs::DhtMode::Server).await?;
    hermes_a.add_bootstrap(b_p2p).await?;
    hermes_a.bootstrap().await?;
    println!("***************************************");
    println!("* Bootstrapping node B.");
    hermes_b.dht_mode(rust_ipfs::DhtMode::Server).await?;
    hermes_b.add_bootstrap(a_p2p).await?;
    hermes_b.bootstrap().await?;
    println!("***************************************");
    Ok((hermes_a, hermes_b))
}

/// Example application.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // File to be uploaded
    let ipfs_file = b"DEMO FILE DISTRIBUTED WITH DHT".to_vec();
    // Start Node A, publish file, and make node provider for CID
    let (hermes_ipfs_a, hermes_ipfs_b) = start_bootstrapped_nodes().await?;
    println!("* Hermes IPFS node A is publishing 'my_key' to DHT.");
    hermes_ipfs_a.dht_put(b"my_key", ipfs_file).await?;
    println!("* Hermes IPFS node B is getting 'my_key' from DHT.");
    let data_retrieved = hermes_ipfs_b.dht_get(b"my_key").await?;
    let data = String::from_utf8(data_retrieved)?;
    println!("  Got data: {data:?}");
    // Stop the nodes and exit.
    hermes_ipfs_a.stop().await;
    println!("***************************************");
    println!("* Hermes IPFS node A has stopped.");
    println!("***************************************");
    hermes_ipfs_b.stop().await;
    println!("* Hermes IPFS node B has stopped.");
    println!("***************************************");
    Ok(())
}
