//! Run cli commands module

use std::path::PathBuf;

use clap::Args;
use console::Emoji;

use crate::packaging::wasm_module::WasmModulePackage;

/// Package cli command
#[derive(Args)]
pub(crate) struct PackageCommand {
    /// Directory where placed all necessary files to package wasm module
    /// A full description of the package can be found <https://input-output-hk.github.io/hermes/architecture/08_concepts/hermes_packaging_requirements/wasm_modules/#overview-of-a-wasm-component-module>
    #[clap(long)]
    dir: PathBuf,
}

impl PackageCommand {
    /// Run cli command
    pub(crate) fn exec(self) -> anyhow::Result<()> {
        println!("{} Build wasm module package...", Emoji::new("📦", ""));

        WasmModulePackage::from_dir(self.dir)?;

        println!("{} Done", Emoji::new("✅", ""));

        Ok(())
    }
}
