//! Run cli commands module

use std::path::PathBuf;

use clap::Args;
use console::Emoji;

use crate::packaging::wasm_module::{manifest::Manifest, WasmModulePackage};

/// Package cli command
#[derive(Args)]
pub(crate) struct PackageCommand {
    /// Directory where placed all necessary files to package wasm module
    /// A full description of the package can be found <https://input-output-hk.github.io/hermes/architecture/08_concepts/hermes_packaging_requirements/wasm_modules/#overview-of-a-wasm-component-module>
    manifest_path: PathBuf,
}

impl PackageCommand {
    /// Run cli command
    pub(crate) fn exec(self) -> anyhow::Result<()> {
        println!("{} Build wasm module package...", Emoji::new("📦", ""));

        let manifest_dir = self
            .manifest_path
            .parent()
            .ok_or(anyhow::anyhow!("Cannot get manifest working directory"))?;
        let manifest = Manifest::from_file(&self.manifest_path)?;
        WasmModulePackage::from_manifest(manifest, manifest_dir)?;

        println!("{} Done", Emoji::new("✅", ""));
        Ok(())
    }
}
