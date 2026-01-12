//! Use Hermes IPFS to distribute content using DHT
#![allow(clippy::println_empty_string)]

use hermes_ipfs::{HermesIpfs, IpfsPath};

/// Connect Node A, upload file and provide CID by adding to DHT
async fn connect_node_a_upload_and_provide(
    file_content: Vec<u8>
) -> anyhow::Result<(HermesIpfs, IpfsPath)> {
    let hermes_ipfs = HermesIpfs::start().await?;
    println!("***************************************");
    println!("* Hermes IPFS node A has started.");
    println!("");
    let peer_id_a = hermes_ipfs.identity(None).await?;
    let addresses = hermes_ipfs.listening_addresses().await?;
    println!("* Peer ID: {}", peer_id_a.peer_id);
    for addr in addresses {
        println!("    * {addr}");
    }
    println!("***************************************");
    println!("");
    println!("***************************************");
    println!("* Adding file to IPFS:");
    println!("");
    let ipfs_path = hermes_ipfs.add_ipfs_file(file_content).await?;
    println!("* IPFS file published at {ipfs_path}");
    let cid = ipfs_path.root().cid().ok_or(anyhow::anyhow!(
        "ERROR! Could not extract CID from IPFS path."
    ))?;
    println!("* CID: {cid}");
    println!("* CID Version: {:?}", cid.version());
    println!("***************************************");
    println!("");
    println!("***************************************");
    println!("* Providing content to DHT:");
    println!("");
    println!("* Providing {cid} as peer {}", peer_id_a.peer_id);
    println!("***************************************");
    println!("");
    Ok((hermes_ipfs, ipfs_path))
}

/// Connect Node A, upload file and provide CID by adding to DHT
async fn connect_node_b_to_node_a(node_a: &HermesIpfs) -> anyhow::Result<HermesIpfs> {
    let hermes_ipfs_b = HermesIpfs::start().await?;
    println!("***************************************");
    println!("* Hermes IPFS node B has started.");
    println!("");
    let peer_id_b = hermes_ipfs_b.identity(None).await?;
    // node_b.connect(peer_id_a).await?;
    println!("* Peer ID: {}", peer_id_b.peer_id);
    println!("* Listening addresses:");
    let addresses = hermes_ipfs_b.listening_addresses().await?;
    for addr in addresses {
        println!("    * {addr}");
    }
    println!("***************************************");
    println!("");
    println!("***************************************");
    println!("* Connecting Node B to Node A:");
    println!("");
    println!("* Adding peer listening addresses from Node A:");
    let node_a_addresses = node_a.listening_addresses().await?;
    let peer_a = node_a.identity(None).await?;
    for addr in node_a_addresses {
        hermes_ipfs_b.add_peer(peer_a.peer_id, addr.clone()).await?;
        println!("    * {addr} - CONNECTED");
    }
    println!("***************************************");
    println!("");
    Ok(hermes_ipfs_b)
}

/// Example application.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // File to be uploaded
    let ipfs_file = b"DEMO FILE DISTRIBUTED WITH IPFS".to_vec();
    // Start Node A, publish file, and make node provider for CID
    let (hermes_ipfs_a, ipfs_path) = connect_node_a_upload_and_provide(ipfs_file.clone()).await?;
    // Start Node B, add listening addresses from Node A, and
    // connect to Node A's peer ID.
    let hermes_ipfs_b = connect_node_b_to_node_a(&hermes_ipfs_a).await?;

    println!("***************************************");
    println!("* Get content from IPFS path {ipfs_path}");
    println!("");
    // Fetch the content from the `ipfs_path`.
    let fetched_bytes = hermes_ipfs_b
        .get_ipfs_file_cbor(
            ipfs_path
                .root()
                .cid()
                .ok_or(anyhow::anyhow!("Could not get CID"))?,
        )
        .await?;
    assert_eq!(ipfs_file, fetched_bytes);
    let fetched_file = String::from_utf8(fetched_bytes)?;
    println!("* Fetched: {fetched_file:?}");
    println!("***************************************");
    println!("");
    // Stop the nodes and exit.
    hermes_ipfs_a.stop().await;
    println!("***************************************");
    println!("* Hermes IPFS node A has stopped.");
    println!("***************************************");
    hermes_ipfs_b.stop().await;
    println!("");
    println!("***************************************");
    println!("* Hermes IPFS node B has stopped.");
    println!("***************************************");
    Ok(())
}
