//! WASM module package command

use std::path::PathBuf;

use clap::Args;
use console::Emoji;

use crate::packaging::wasm_module::{manifest::Manifest, WasmModulePackage};

/// WASM module packaging
#[derive(Args)]
pub(crate) struct PackageCommand {
    /// Defines the location of all the src artifacts needed to build the package. This
    /// file must conform to the manifests JSON schema. An example manifest of this JSON
    /// schema is here.
    manifest_path: PathBuf,

    /// By default the module will be created in the same directory where manifest placed.
    /// This option allows the path of the generated module to be set, it can be absolute
    /// or relative to the manifest directory.
    output_path: Option<PathBuf>,
}

impl PackageCommand {
    /// Run cli command
    pub(crate) fn exec(self) -> anyhow::Result<()> {
        println!("{} Build wasm module package...", Emoji::new("📦", ""));

        let manifest_dir = self
            .manifest_path
            .parent()
            .ok_or(anyhow::anyhow!("Cannot get manifest working directory"))?;
        let output_path = self
            .output_path
            .map(|output_path| {
                if output_path.is_relative() {
                    manifest_dir.join(&output_path)
                } else {
                    output_path
                }
            })
            .unwrap_or(manifest_dir.into());

        let manifest = Manifest::from_file(&self.manifest_path)?;
        WasmModulePackage::from_manifest(manifest, output_path)?;

        println!("{} Done", Emoji::new("✅", ""));
        Ok(())
    }
}