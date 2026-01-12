//! Hermes IPFS File Publishing and Pinning

use hermes_ipfs::{Cid, HermesIpfs};

/// Print helper
async fn print_cid_pinned(
    hermes_ipfs: &HermesIpfs,
    cid: &Cid,
) -> anyhow::Result<()> {
    let is_pinned = hermes_ipfs.is_pinned(cid).await?;
    println!("* Is CID pinned?: {is_pinned:?}");
    Ok(())
}

/// Example application.
#[tokio::main]
#[allow(clippy::println_empty_string)]
async fn main() -> anyhow::Result<()> {
    let hermes_ipfs = HermesIpfs::start().await?;
    println!("***************************************");
    println!("* Hermes IPFS node has started.");
    println!("***************************************");
    println!("");
    println!("***************************************");
    println!("* Adding file to IPFS:");
    println!("");
    let ipfs_file = b"This is a demo file that is stored in IPFS.".to_vec();
    let ipfs_path = hermes_ipfs.add_ipfs_file(ipfs_file).await?;
    println!("* IPFS file published at {ipfs_path}");
    let cid = ipfs_path.root().cid().ok_or(anyhow::anyhow!(
        "ERROR! Could not extract CID from IPFS path."
    ))?;
    println!("* CID: {cid}");
    println!("* CID Version: {:?}", cid.version());
    print_cid_pinned(&hermes_ipfs, cid).await?;
    println!("***************************************");
    println!("");
    println!("***************************************");
    println!("* CID Pinning:");
    println!("");
    if let Err(e) = hermes_ipfs.insert_pin(cid).await {
        if e.to_string().contains("already pinned recursively") {
            println!("{cid} is already pinned");
        } else {
            println!("AN ERROR OCCURRED: {e}");
        }
    }
    println!("* Removing pin.");
    hermes_ipfs.remove_pin(cid).await?;
    print_cid_pinned(&hermes_ipfs, cid).await?;
    println!("");
    println!("* Re-pinning CID:.");
    if let Err(e) = hermes_ipfs.insert_pin(cid).await {
        if e.to_string().contains("already pinned recursively") {
            println!("{cid} is already pinned");
        } else {
            println!("AN ERROR OCCURRED: {e}");
        }
    }
    print_cid_pinned(&hermes_ipfs, cid).await?;
    println!("***************************************");
    println!("");
    println!("***************************************");
    println!("* Get file from IPFS:");
    println!("");
    println!("* Retrieving from {ipfs_path}");
    let get_file_bytes = hermes_ipfs
        .get_ipfs_file_cbor(
            ipfs_path
                .root()
                .cid()
                .ok_or(anyhow::anyhow!("Could not get CID"))?,
        )
        .await?;
    println!("* Got file, {} bytes:", get_file_bytes.len());
    let get_file = String::from_utf8(get_file_bytes)?;
    println!("* FILE CONTENTS:");
    println!("");
    println!("{get_file}");
    println!("");
    println!("***************************************");
    println!("");
    hermes_ipfs.stop().await;
    println!("***************************************");
    println!("* Hermes IPFS node has stopped.");
    println!("***************************************");
    Ok(())
}
