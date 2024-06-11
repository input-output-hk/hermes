//! WASM module sign command

use std::path::PathBuf;

use clap::Args;
use console::Emoji;

/// WASM module package signing
#[derive(Args)]
pub(crate) struct SignCommand {
    /// Defines the location of the builded WASM module package.
    package: PathBuf,

    /// Defines the location of the ED2559 private key associated with the signing key.
    private_key: PathBuf,

    /// Defines the location of the x.509 certificate associated with the signing key.
    cert: PathBuf,
}

impl SignCommand {
    /// Run cli command
    #[allow(clippy::unnecessary_wraps, clippy::unused_self)]
    pub(crate) fn exec(self) -> anyhow::Result<()> {
        println!("{} Sign wasm module package...", Emoji::new("📝", ""));

        println!("{} Done", Emoji::new("✅", ""));
        Ok(())
    }
}
