//! Wasm module package.

mod config;
pub(crate) mod manifest;
mod metadata;
mod settings;
mod signature_payload;

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use config::{Config, ConfigSchema};
use metadata::Metadata;
use settings::SettingsSchema;
use signature_payload::SignaturePayloadBuilder;

use self::manifest::Manifest;
use super::{
    copy_dir_recursively_to_package, copy_resource_to_package,
    resources::{bytes_resource::BytesResource, ResourceTrait},
};
use crate::{errors::Errors, wasm};

/// Create WASM module package error.
#[derive(thiserror::Error, Debug)]
#[error("Failed to create WASM module package. Package at {0} could be already exists.")]
pub(crate) struct CreatePackageError(PathBuf);

/// Invalid file error.
#[derive(thiserror::Error, Debug)]
#[error("Invalid file at {0}:\n{1}")]
pub(crate) struct InvalidFileError(String, String);

/// Wasm module package.
#[derive(Debug)]
pub(crate) struct WasmModulePackage {
    /// hdf5 package instance.
    package: hdf5::File,
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
    pub(crate) fn build_from_manifest<P: AsRef<Path>>(
        manifest: &Manifest, output_path: P, package_name: Option<&str>, build_time: DateTime<Utc>,
    ) -> anyhow::Result<Self> {
        let package_name = package_name.unwrap_or(&manifest.name);
        let mut package_path = output_path.as_ref().join(package_name);
        package_path.set_extension(Self::FILE_EXTENSION);
        let package = hdf5::File::create(&package_path)
            .map_err(|_| CreatePackageError(package_path.clone()))?;

        let mut errors = Errors::new();

        Self::validate_and_write_metadata(manifest, build_time, package_name, &package)
            .unwrap_or_else(|err| errors.add_err(err));
        Self::validate_and_write_component(manifest, &package)
            .unwrap_or_else(|err| errors.add_err(err));
        Self::validate_and_write_config(manifest, &package)
            .unwrap_or_else(|err| errors.add_err(err));
        Self::validate_and_write_settings(manifest, &package)
            .unwrap_or_else(|err| errors.add_err(err));
        Self::write_share_dir(manifest, &package).unwrap_or_else(|err| {
            match err.downcast::<Errors>() {
                Ok(errs) => errors.merge(errs),
                Err(err) => errors.add_err(err),
            }
        });

        if !errors.is_empty() {
            std::fs::remove_file(package_path).unwrap_or_else(|err| errors.add_err(err.into()));
        }

        errors.return_result(Self { package })
    }

    /// Open an existing WASM module package.
    pub(crate) fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let package = hdf5::File::open_rw(&path).map_err(|_| {
            anyhow::anyhow!(
                "Cannot load a WASM module package at {0}.",
                path.as_ref().display()
            )
        })?;
        Ok(Self { package })
    }

    /// Sign the package and store signature inside it.
    pub(crate) fn sign(&self) -> anyhow::Result<()> {
        let metadata = self.get_metadata()?;
        let metadata_hash = metadata.hash()?;

        let mut signature_payload_builder =
            SignaturePayloadBuilder::new(metadata_hash.clone(), metadata_hash.clone());
        if let Some(setting_schema) = self.get_settings_schema()? {
            signature_payload_builder.with_settings_schema(setting_schema.hash()?);
        }
        let (config, config_schema) = self.get_config_with_schema()?;
        if let Some(config) = config {
            signature_payload_builder.with_config_file(config.hash()?);
        }
        if let Some(config_schema) = config_schema {
            signature_payload_builder.with_config_schema(config_schema.hash()?);
        }
        signature_payload_builder.with_share(metadata_hash);

        let _signature_payload = signature_payload_builder.build();

        Ok(())
    }

    /// Get `Metadata` object from package.
    pub(crate) fn get_metadata(&self) -> anyhow::Result<Metadata> {
        let ds = self.package.dataset(Self::METADATA_FILE)?;
        let reader = ds.as_byte_reader()?;
        Metadata::from_reader(reader)
    }

    /// Get `ConfigSchema` object from package.
    pub(crate) fn get_config_schema(&self) -> anyhow::Result<Option<ConfigSchema>> {
        let ds = self.package.dataset(Self::CONFIG_SCHEMA_FILE)?;
        let Ok(reader) = ds.as_byte_reader() else {
            return Ok(None);
        };
        Ok(Some(ConfigSchema::from_reader(reader)?))
    }

    /// Get `Config` and `ConfigSchema` objects from package if present.
    /// To obtain a valid `Config` object it is needed to get `ConfigSchema` first.
    pub(crate) fn get_config_with_schema(
        &self,
    ) -> anyhow::Result<(Option<Config>, Option<ConfigSchema>)> {
        let Some(config_schema) = self.get_config_schema()? else {
            return Ok((None, None));
        };

        let ds = self.package.dataset(Self::CONFIG_FILE)?;
        let Ok(reader) = ds.as_byte_reader() else {
            return Ok((None, Some(config_schema)));
        };
        let config = Config::from_reader(reader, config_schema.validator())?;
        Ok((Some(config), Some(config_schema)))
    }

    /// Get `SettingsSchema` object from package if present.
    pub(crate) fn get_settings_schema(&self) -> anyhow::Result<Option<SettingsSchema>> {
        let ds = self.package.dataset(Self::SETTINGS_SCHEMA_FILE)?;
        let Ok(reader) = ds.as_byte_reader() else {
            return Ok(None);
        };
        Ok(Some(SettingsSchema::from_reader(reader)?))
    }

    /// Validate metadata.json file and write it to the package.
    /// Also updates `Metadata` object by setting `build_date` and `name` properties.
    fn validate_and_write_metadata(
        manifest: &Manifest, build_date: DateTime<Utc>, name: &str, package: &hdf5::File,
    ) -> anyhow::Result<()> {
        let resource = &manifest.metadata;
        let metadata_reader = resource
            .get_reader()
            .map_err(|err| InvalidFileError(resource.location(), err.to_string()))?;

        let mut metadata = Metadata::from_reader(metadata_reader)
            .map_err(|err| InvalidFileError(resource.location(), err.to_string()))?;
        metadata.set_build_date(build_date);
        metadata.set_name(name);

        let resource = BytesResource::new(resource.name()?, metadata.to_bytes()?);
        copy_resource_to_package(&resource, Self::METADATA_FILE, package)?;
        Ok(())
    }

    /// Validate WASM component file and write it to the package.
    fn validate_and_write_component(
        manifest: &Manifest, package: &hdf5::File,
    ) -> anyhow::Result<()> {
        let resource = &manifest.component;

        let component_reader = resource
            .get_reader()
            .map_err(|err| InvalidFileError(resource.location(), err.to_string()))?;

        wasm::module::Module::from_reader(component_reader)
            .map_err(|err| InvalidFileError(resource.location(), err.to_string()))?;

        copy_resource_to_package(resource, Self::COMPONENT_FILE, package)?;
        Ok(())
    }

    /// Validate config file and config schema and write them to the package.
    fn validate_and_write_config(manifest: &Manifest, package: &hdf5::File) -> anyhow::Result<()> {
        if let Some(config) = &manifest.config {
            let config_schema_reader = config
                .schema
                .get_reader()
                .map_err(|err| InvalidFileError(config.schema.location(), err.to_string()))?;
            let config_schema = ConfigSchema::from_reader(config_schema_reader)
                .map_err(|err| InvalidFileError(config.schema.location(), err.to_string()))?;

            let resource = BytesResource::new(config.schema.name()?, config_schema.to_bytes()?);
            copy_resource_to_package(&resource, Self::CONFIG_SCHEMA_FILE, package)?;

            if let Some(config_file) = &config.file {
                let config_reader = config_file
                    .get_reader()
                    .map_err(|err| InvalidFileError(config_file.location(), err.to_string()))?;

                let config = Config::from_reader(config_reader, config_schema.validator())
                    .map_err(|err| InvalidFileError(config_file.location(), err.to_string()))?;

                let resource = BytesResource::new(config_file.name()?, config.to_bytes()?);
                copy_resource_to_package(&resource, Self::CONFIG_FILE, package)?;
            }
        }
        Ok(())
    }

    /// Validate settings schema file and it to the package.
    fn validate_and_write_settings(
        manifest: &Manifest, package: &hdf5::File,
    ) -> anyhow::Result<()> {
        if let Some(settings) = &manifest.settings {
            let setting_schema_reader = settings
                .schema
                .get_reader()
                .map_err(|err| InvalidFileError(settings.schema.location(), err.to_string()))?;
            let settings_schema = SettingsSchema::from_reader(setting_schema_reader)
                .map_err(|err| InvalidFileError(settings.schema.location(), err.to_string()))?;

            let resource = BytesResource::new(settings.schema.name()?, settings_schema.to_bytes()?);
            copy_resource_to_package(&resource, Self::SETTINGS_SCHEMA_FILE, package)?;
        }
        Ok(())
    }

    /// Write share dir to the package.
    fn write_share_dir(manifest: &Manifest, package: &hdf5::File) -> anyhow::Result<()> {
        if let Some(share_dir) = &manifest.share {
            copy_dir_recursively_to_package(share_dir, Self::SHARE_DIR, package)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use temp_dir::TempDir;

    use super::*;
    use crate::packaging::resources::{fs_resource::FsResource, Resource};

    fn prepare_package_dir(
        module_name: String, dir: &TempDir, metadata: &[u8], component: &[u8], config: &[u8],
        config_schema: &[u8], settings_schema: &[u8],
    ) -> Manifest {
        let config_path = dir.path().join("config.json");
        let config_schema_path = dir.path().join("config.schema.json");
        let metadata_path = dir.path().join("metadata.json");
        let component_path = dir.path().join("module.wasm");
        let settings_schema_path = dir.path().join("settings.schema.json");

        std::fs::write(&metadata_path, metadata).expect("Cannot create metadata.json file");
        std::fs::write(&component_path, component).expect("Cannot create module.wasm file");
        std::fs::write(&config_path, config).expect("Cannot create config.json file");
        std::fs::write(&config_schema_path, config_schema)
            .expect("Cannot create config.schema.json file");
        std::fs::write(&settings_schema_path, settings_schema)
            .expect("Cannot create settings.schema.json file");

        Manifest {
            name: module_name,
            metadata: Resource::Fs(FsResource::new(metadata_path)),
            component: Resource::Fs(FsResource::new(component_path)),
            config: manifest::ManifestConfig {
                file: Some(Resource::Fs(FsResource::new(config_path))),
                schema: Resource::Fs(FsResource::new(config_schema_path)),
            }
            .into(),
            settings: manifest::ManifestSettings {
                schema: Resource::Fs(FsResource::new(settings_schema_path)),
            }
            .into(),
            share: None,
        }
    }

    #[test]
    fn from_dir_test() {
        let dir = TempDir::new().expect("cannot create temp dir");

        let mut metadata = Metadata::from_reader(
            serde_json::json!(
                {
                    "$schema": "https://raw.githubusercontent.com/input-output-hk/hermes/main/hermes/schemas/hermes_module_metadata.schema.json",
                    "name": "Test module",
                    "version": "V1.0.0",
                    "description": "Some description",
                    "src": ["https://github.com/input-output-hk/hermes"],
                    "copyright": ["Copyright Ⓒ 2024, IOG Singapore."],
                    "license": [{"spdx": "MIT"}]
                }
            ).to_string().as_bytes()
        ).expect("Invalid metadata");
        let config_schema = ConfigSchema::from_reader(serde_json::json!({}).to_string().as_bytes())
            .expect("Invalid config schema");

        let config = Config::from_reader(
            serde_json::json!({}).to_string().as_bytes(),
            config_schema.validator(),
        )
        .expect("Invalid config");

        let settings_schema =
            SettingsSchema::from_reader(serde_json::json!({}).to_string().as_bytes())
                .expect("Invalid settings schema");
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

        let manifest = prepare_package_dir(
            "module".to_string(),
            &dir,
            metadata
                .to_bytes()
                .expect("cannot decode metadata to bytes")
                .as_slice(),
            component.to_string().as_bytes(),
            config
                .to_bytes()
                .expect("cannot decode config to bytes")
                .as_slice(),
            config_schema
                .to_bytes()
                .expect("cannot decode config schema to bytes")
                .as_slice(),
            settings_schema
                .to_bytes()
                .expect("cannot decode settings schema to bytes")
                .as_slice(),
        );

        let build_time = DateTime::default();
        let package =
            WasmModulePackage::build_from_manifest(&manifest, dir.path(), None, build_time)
                .expect("Cannot create module package");

        // check metadata JSON file
        metadata.set_name(&manifest.name);
        metadata.set_build_date(build_time);

        let package_metadata = package
            .get_metadata()
            .expect("Cannot get metadata from package");
        assert_eq!(metadata, package_metadata);

        // check config and config schema JSON files
        let (package_config, package_config_schema) = package
            .get_config_with_schema()
            .expect("Cannot get config from package");
        assert_eq!(config, package_config.expect("Missing config in package"));
        assert_eq!(
            config_schema,
            package_config_schema.expect("Missing config schema in package")
        );

        // check settings schema JSON file
        let package_settings_schema = package
            .get_settings_schema()
            .expect("Cannot get settings schema from package");
        assert_eq!(
            settings_schema,
            package_settings_schema.expect("Missing settings schema in package")
        );
    }
}
