//! Use Hermes IPFS to distribute content using DHT

use hermes_ipfs::{AddIpfsFile, HermesIpfs};

/// Example application.
#[tokio::main]
#[allow(clippy::println_empty_string)]
async fn main() -> anyhow::Result<()> {
    let hermes_ipfs_a = HermesIpfs::start().await?;
    let node_a = hermes_ipfs_a.node();
    println!("***************************************");
    println!("* Hermes IPFS node A has started.");
    println!("");
    let peer_info = hermes_ipfs_a.identity(None).await?;
    let peer_id_a = peer_info.peer_id;
    println!("* Peer ID: {peer_id_a}");
    let addresses = node_a.listening_addresses().await?;
    for addr in addresses {
        println!("    * {addr}");
    }
    println!("***************************************");
    println!("");
    println!("***************************************");
    println!("* Adding file to IPFS:");
    println!("");
    let ipfs_file = b"This is a demo content distributed via IPFS.".to_vec();
    let ipfs_path = hermes_ipfs_a
        .add_ipfs_file(AddIpfsFile::Stream((None, ipfs_file)))
        .await?;
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
    println!("* Providing {cid} as peer {peer_id_a}");
    println!("***************************************");
    println!("");
    let hermes_ipfs_b = HermesIpfs::start().await?;
    let node_b = hermes_ipfs_b.node();
    println!("***************************************");
    println!("* Hermes IPFS node B has started.");
    println!("");
    let peer_info = hermes_ipfs_b.identity(None).await?;
    let peer_id_b = peer_info.peer_id;
    //node_b.connect(peer_id_a).await?;
    println!("* Peer ID: {peer_id_b}");
    println!("* Listening addresses:");
    let addresses = node_b.listening_addresses().await?;
    for addr in addresses {
        println!("    * {addr}");
    }
    hermes_ipfs_a.stop().await;
    println!("***************************************");
    println!("* Hermes IPFS node has stopped.");
    println!("***************************************");
    Ok(())
}
