//! Wasm module package.

pub(crate) mod manifest;

use std::path::{Path, PathBuf};

use self::manifest::Manifest;
use super::{copy_dir_recursively_to_package, copy_file_from_dir_to_package};
use crate::errors::Errors;

/// Create WASM module package error.
#[derive(thiserror::Error, Debug)]
#[error("Failed to create WASM module package. Package at {0} could be already exists.")]
pub(crate) struct CreatePackageError(PathBuf);

/// Wasm module package.
#[derive(Debug)]
pub(crate) struct WasmModulePackage {
    /// hdf5 package instance
    _package: hdf5::File,
}

impl WasmModulePackage {
    /// WASM module package file extension.
    const FILE_EXTENSION: &'static str = "hmod";

    /// Create a new WASM module package from a manifest file.
    pub(crate) fn from_manifest<P: AsRef<Path>>(
        manifest: Manifest, output_path: P, package_name: Option<String>,
    ) -> anyhow::Result<Self> {
        let mut errors = Errors::new();

        let package_name = package_name.unwrap_or(manifest.name);
        let mut package_path = output_path.as_ref().join(package_name);
        package_path.set_extension(Self::FILE_EXTENSION);
        let package = hdf5::File::create(&package_path)
            .map_err(|_| CreatePackageError(package_path.clone()))?;

        copy_file_from_dir_to_package(manifest.metadata, &package)
            .unwrap_or_else(|err| errors.add_err(err));

        copy_file_from_dir_to_package(manifest.component, &package)
            .unwrap_or_else(|err| errors.add_err(err));

        if let Some(config) = manifest.config {
            if let Some(config_file) = config.file {
                copy_file_from_dir_to_package(config_file, &package)
                    .unwrap_or_else(|err| errors.add_err(err));
            }
            copy_file_from_dir_to_package(config.schema, &package)
                .unwrap_or_else(|err| errors.add_err(err));
        }

        if let Some(settings) = manifest.settings {
            copy_file_from_dir_to_package(settings.schema, &package)
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
            std::fs::remove_file(package_path).unwrap_or_else(|err| errors.add_err(err.into()));
        }

        errors.return_result(Self { _package: package })
    }
}

#[cfg(test)]
mod tests {
    use manifest::{Config, Settings};
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
            name: "module".to_string(),
            metadata: metadata_path,
            component: component_path,
            config: Config {
                file: config_path.into(),
                schema: config_schema_path,
            }
            .into(),
            settings: Settings {
                schema: settings_schema_path,
            }
            .into(),
            share: None,
        };
        WasmModulePackage::from_manifest(manifest, dir.path(), None)
            .expect("Cannot create module package");
    }
}
