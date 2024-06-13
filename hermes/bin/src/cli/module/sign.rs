//! WASM module sign command

use std::path::PathBuf;

use clap::Args;
use console::Emoji;

use crate::{
    packaging::wasm_module::WasmModulePackage,
    sign::{certificate::Certificate, keys::PrivateKey},
};

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
    pub(crate) fn exec(self) -> anyhow::Result<()> {
        println!("{} Sign wasm module package...", Emoji::new("📝", ""));

        let _private_key = PrivateKey::from_file(self.private_key)?;
        let _cert = Certificate::from_file(self.cert)?;
        let _package = WasmModulePackage::from_file(self.package)?;

        println!("{} Done", Emoji::new("✅", ""));
        Ok(())
    }
}
