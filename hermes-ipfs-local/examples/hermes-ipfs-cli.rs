//! Hermes IPFS VFS compatibility

use clap::{Parser, Subcommand};
use connexa::dummy;
use hermes_ipfs::{HermesIpfs, HermesIpfsBuilder};
use lipsum::lipsum;
use rust_ipfs::IpfsPath;

/// CLI for a virtual filesystem.
#[derive(Debug, Parser)] // requires `derive` feature
#[command(name = "hermes-ipfs-cli")]
#[command(about = "Hermes IPFS Virtual File Manager", long_about = None)]
struct Cli {
    /// CLI commands
    #[command(subcommand)]
    command: Commands,
}

/// Commands for the CLI.
#[derive(Debug, Subcommand)]
enum Commands {
    /// List Files
    #[command(name = "ls")]
    ListFiles,
    /// Add a file with random content to IPFS
    #[command(name = "add")]
    AddFile,
    /// Print the contents from a file in IPFS
    #[command(name = "cat")]
    GetFile {
        /// IPFS Path
        ipfs_path_str: String,
    },
    /// Remove the file from being listed (will be garbage collected)
    #[command(name = "rm")]
    UnPinFile {
        /// IPFS Path
        ipfs_path_str: String,
    },
}

/// Example application.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let base_dir = dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let ipfs_data_path = base_dir.as_path().join("hermes/ipfs");
    let builder = HermesIpfsBuilder::<dummy::Behaviour>::new()
        .with_default()
        .set_default_listener()
        // TODO(saibatizoku): Re-Enable default transport config when libp2p Cert bug is fixed
        //.enable_secure_websocket()
        .set_disk_storage(ipfs_data_path);
    let hermes_node: HermesIpfs = builder.start().await?.into();
    match args.command {
        Commands::ListFiles => {
            println!("Listing files");
            let cids = hermes_node.list_pins().await?;
            for cid in &cids {
                println!("{}", IpfsPath::from(cid));
            }
        },
        Commands::AddFile => {
            println!("Adding file");
            let contents = lipsum(42);
            let ipfs_path = hermes_node.add_ipfs_file(contents.into_bytes()).await?;
            println!("Added file: {ipfs_path}");
        },
        Commands::GetFile { ipfs_path_str } => {
            println!("Getting file");
            let ipfs_path: IpfsPath = ipfs_path_str.parse()?;
            let get_file_bytes = hermes_node
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
            println!("{get_file}\n");
        },
        Commands::UnPinFile { ipfs_path_str } => {
            println!("Un-pinning file {ipfs_path_str}");
            let ipfs_path: IpfsPath = ipfs_path_str.parse()?;
            let cid = ipfs_path.root().cid().ok_or(anyhow::anyhow!(
                "ERROR! Could not extract CID from IPFS path."
            ))?;
            hermes_node.remove_pin(cid).await?;
        },
    }
    hermes_node.stop().await;
    Ok(())
}
