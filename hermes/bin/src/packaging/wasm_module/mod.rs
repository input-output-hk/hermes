//! Wasm module package.

pub(crate) mod manifest;

use std::path::Path;

use self::manifest::Manifest;
use super::{copy_dir_recursively_to_package, copy_file_from_dir_to_package};
use crate::errors::Errors;

/// Wasm module package.
#[derive(Debug)]
pub(crate) struct WasmModulePackage {
    /// hdf5 package instance
    _package: hdf5::File,
}

impl WasmModulePackage {
    /// Create a new WASM module package from a manifest file.
    pub(crate) fn from_manifest<P: AsRef<Path>>(
        manifest: Manifest, output_path: P,
    ) -> anyhow::Result<Self> {
        let mut errors = Errors::new();

        let package = hdf5::File::create(&output_path)?;

        copy_file_from_dir_to_package(manifest.metadata, &package)
            .unwrap_or_else(|err| errors.add_err(err));

        copy_file_from_dir_to_package(manifest.component, &package)
            .unwrap_or_else(|err| errors.add_err(err));

        if let Some(config) = manifest.config {
            copy_file_from_dir_to_package(config, &package)
                .unwrap_or_else(|err| errors.add_err(err));
        }

        if let Some(config_schema) = manifest.config_schema {
            copy_file_from_dir_to_package(config_schema, &package)
                .unwrap_or_else(|err| errors.add_err(err));
        }

        if let Some(settings_schema) = manifest.settings_schema {
            copy_file_from_dir_to_package(settings_schema, &package)
                .unwrap_or_else(|err| errors.add_err(err));
        }

        if let Some(share_path) = manifest.share {
            copy_dir_recursively_to_package(share_path, &package).unwrap_or_else(|err| {
                match err.downcast::<Errors>() {
                    Ok(errs) => errors.merge(errs),
                    Err(err) => errors.add_err(err),
                }
            });
        }

        if !errors.is_empty() {
            std::fs::remove_file(output_path).unwrap_or_else(|err| errors.add_err(err.into()));
        }

        errors.return_result(Self { _package: package })
    }
}

#[cfg(test)]
mod tests {
    use temp_dir::TempDir;

    use super::*;

    #[test]
    fn from_dir_test() {
        let dir = TempDir::new().expect("cannot create temp dir");

        let config_path = dir.path().join("config.json");
        let config_schema_path = dir.path().join("config.schema.json");
        let metadata_path = dir.path().join("metadata.json");
        let component_path = dir.path().join("module.wasm");
        let settings_schema_path = dir.path().join("settings.schema.json");

        std::fs::File::create(&config_path).expect("Cannot create config.json file");
        std::fs::File::create(&config_schema_path).expect("Cannot create config.schema.json file");
        std::fs::File::create(&metadata_path).expect("Cannot create metadata.json file");
        std::fs::File::create(&component_path).expect("Cannot create module.wasm file");
        std::fs::File::create(&settings_schema_path)
            .expect("Cannot create settings.schema.json file");

        let manifest = Manifest {
            metadata: metadata_path,
            component: component_path,
            config: Some(config_path),
            config_schema: Some(config_schema_path),
            settings_schema: Some(settings_schema_path),
            share: None,
        };
        let package_path = dir.path().join("module.hdf5");
        WasmModulePackage::from_manifest(manifest, package_path)
            .expect("Cannot create module package");
    }
}
