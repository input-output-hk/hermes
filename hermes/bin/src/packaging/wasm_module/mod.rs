//! Wasm module package.

pub(crate) mod manifest;
mod metadata;

use std::{
    io::Read,
    path::{Path, PathBuf},
};

use chrono::Utc;
use manifest::{Config, Settings};
use metadata::Metadata;

use self::manifest::Manifest;
use super::{
    copy_dir_recursively_to_package, copy_resource_to_package, resources::Resource,
    schema_validation::SchemaValidator,
};
use crate::{errors::Errors, wasm};

/// Create WASM module package error.
#[derive(thiserror::Error, Debug)]
#[error("Failed to create WASM module package. Package at {0} could be already exists.")]
pub(crate) struct CreatePackageError(PathBuf);

/// Invalid file error.
#[derive(thiserror::Error, Debug)]
#[error("Invalid file at {0}:\n {1}")]
pub(crate) struct InvalidFileError(Resource, String);

/// Wasm module package.
#[derive(Debug)]
pub(crate) struct WasmModulePackage {
    /// hdf5 package instance
    _package: hdf5::File,
}

impl WasmModulePackage {
    /// WASM module package component file.
    const COMPONENT_FILE: &'static str = "module.wasm";
    /// WASM module package config file.
    const CONFIG_FILE: &'static str = "config.json";
    /// WASM module package config schema file.
    const CONFIG_SCHEMA_FILE: &'static str = "config.schema.json";
    /// WASM module package file extension.
    const FILE_EXTENSION: &'static str = "hmod";
    /// WASM module package metadata file.
    const METADATA_FILE: &'static str = "metadata.json";
    /// WASM module package settings schema file.
    const SETTINGS_SCHEMA_FILE: &'static str = "settings.schema.json";
    /// WASM module package share directory.
    const SHARE_DIR: &'static str = "share";

    /// Create a new WASM module package from a manifest file.
    pub(crate) fn from_manifest<P: AsRef<Path>>(
        manifest: Manifest, output_path: P, package_name: Option<&str>,
    ) -> anyhow::Result<Self> {
        let package_name = package_name.unwrap_or(&manifest.name);
        let mut package_path = output_path.as_ref().join(package_name);
        package_path.set_extension(Self::FILE_EXTENSION);
        let package = hdf5::File::create(&package_path)
            .map_err(|_| CreatePackageError(package_path.clone()))?;

        let mut errors = Errors::new();

        Self::validate(&manifest, &mut errors);

        let mut metadata = Metadata::from_resource(&manifest.metadata)?;
        metadata.set_build_date(Utc::now());

        Self::copy_data_to_package(&manifest, &package, &mut errors);

        if !errors.is_empty() {
            std::fs::remove_file(package_path).unwrap_or_else(|err| errors.add_err(err.into()));
        }

        errors.return_result(Self { _package: package })
    }

    fn validate(manifest: &Manifest, errors: &mut Errors) {
        Self::validate_component(&manifest.component).unwrap_or_else(|err| errors.add_err(err));
        if let Some(config) = &manifest.config {
            Self::validate_config(config).unwrap_or_else(|err| errors.add_err(err));
        }
        if let Some(settings) = &manifest.settings {
            Self::validate_settings(settings).unwrap_or_else(|err| errors.add_err(err));
        }
    }

    fn validate_component(component: &Resource) -> anyhow::Result<()> {
        let mut component_reader = component
            .get_reader()
            .map_err(|err| InvalidFileError(component.clone(), err.to_string()))?;

        let mut module_bytes = Vec::new();
        component_reader
            .read_to_end(&mut module_bytes)
            .map_err(|err| InvalidFileError(component.clone(), err.to_string()))?;

        wasm::module::Module::new(&module_bytes)
            .map_err(|err| InvalidFileError(component.clone(), err.to_string()))?;
        Ok(())
    }

    fn validate_config(config: &Config) -> anyhow::Result<()> {
        let config_schema_reader = config
            .schema
            .get_reader()
            .map_err(|err| InvalidFileError(config.schema.clone(), err.to_string()))?;
        let config_schema = SchemaValidator::from_reader(config_schema_reader)
            .map_err(|err| InvalidFileError(config.schema.clone(), err.to_string()))?;

        if let Some(config_file) = &config.file {
            let config_schema_reader = config_file
                .get_reader()
                .map_err(|err| InvalidFileError(config_file.clone(), err.to_string()))?;

            config_schema
                .deserialize_and_validate::<_, serde_json::Value>(config_schema_reader)
                .map_err(|err| InvalidFileError(config_file.clone(), err.to_string()))?;
        }
        Ok(())
    }

    fn validate_settings(settings: &Settings) -> anyhow::Result<()> {
        let setting_schema_reader = settings
            .schema
            .get_reader()
            .map_err(|err| InvalidFileError(settings.schema.clone(), err.to_string()))?;
        SchemaValidator::from_reader(setting_schema_reader)
            .map_err(|err| InvalidFileError(settings.schema.clone(), err.to_string()))?;

        Ok(())
    }

    /// Copy data from manifest to package.
    fn copy_data_to_package(manifest: &Manifest, package: &hdf5::File, errors: &mut Errors) {
        copy_resource_to_package(&manifest.metadata, Self::METADATA_FILE, package)
            .unwrap_or_else(|err| errors.add_err(err));

        copy_resource_to_package(&manifest.component, Self::COMPONENT_FILE, package)
            .unwrap_or_else(|err| errors.add_err(err));

        if let Some(config) = &manifest.config {
            if let Some(config_file) = &config.file {
                copy_resource_to_package(config_file, Self::CONFIG_FILE, package)
                    .unwrap_or_else(|err| errors.add_err(err));
            }
            copy_resource_to_package(&config.schema, Self::CONFIG_SCHEMA_FILE, package)
                .unwrap_or_else(|err| errors.add_err(err));
        }

        if let Some(settings) = &manifest.settings {
            copy_resource_to_package(&settings.schema, Self::SETTINGS_SCHEMA_FILE, package)
                .unwrap_or_else(|err| errors.add_err(err));
        }

        if let Some(share_path) = &manifest.share {
            copy_dir_recursively_to_package(share_path, Self::SHARE_DIR, package).unwrap_or_else(
                |err| {
                    match err.downcast::<Errors>() {
                        Ok(errs) => errors.merge(errs),
                        Err(err) => errors.add_err(err),
                    }
                },
            );
        }
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

        let config = serde_json::json!({});
        let config_schema = serde_json::json!({});
        let settings_schema = serde_json::json!({});
        let component = r#"
            (component
                (core module $Module
                    (export "foo" (func $foo))
                    (func $foo (result i32)
                        i32.const 1
                    )
                )
                (core instance $module (instantiate (module $Module)))
                (func $foo (result s32) (canon lift (core func $module "foo")))
                (export "foo" (func $foo))
            )"#;

        std::fs::write(&config_path, config.to_string().as_bytes())
            .expect("Cannot create config.json file");
        std::fs::write(&config_schema_path, config_schema.to_string().as_bytes())
            .expect("Cannot create config.schema.json file");
        std::fs::write(&metadata_path, [1, 2, 3]).expect("Cannot create metadata.json file");
        std::fs::write(&component_path, component.to_string().as_bytes())
            .expect("Cannot create module.wasm file");
        std::fs::write(
            &settings_schema_path,
            settings_schema.to_string().as_bytes(),
        )
        .expect("Cannot create settings.schema.json file");

        let manifest = Manifest {
            name: "module".to_string(),
            metadata: metadata_path.into(),
            component: component_path.into(),
            config: Config {
                file: Some(config_path.into()),
                schema: config_schema_path.into(),
            }
            .into(),
            settings: Settings {
                schema: settings_schema_path.into(),
            }
            .into(),
            share: None,
        };
        WasmModulePackage::from_manifest(&manifest, dir.path(), None)
            .expect("Cannot create module package");
    }
}
