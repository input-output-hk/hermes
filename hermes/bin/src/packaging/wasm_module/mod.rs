//! Hermes WASM module package.

mod config;
pub(crate) mod manifest;
mod metadata;
mod settings;
mod signature_payload;

use std::path::Path;

use chrono::{DateTime, Utc};
use config::{Config, ConfigSchema};
use manifest::Manifest;
use metadata::Metadata;
use settings::SettingsSchema;
use signature_payload::{SignaturePayload, SignaturePayloadBuilder};

use crate::{
    errors::Errors,
    packaging::{
        package::Package,
        resources::{bytes_resource::BytesResource, ResourceTrait},
        sign::{
            certificate::Certificate,
            keys::PrivateKey,
            signature::{Signature, SignaturePayloadEncoding},
        },
        FileError,
    },
    wasm,
};

/// Missing package file error.
#[derive(thiserror::Error, Debug)]
#[error("Missing package file {0}.")]
pub(crate) struct MissingPackageFileError(String);

/// Hermes WASM module package.
pub(crate) struct WasmModulePackage(Package);

impl WasmModulePackage {
    /// WASM module package signature file.
    const AUTHOR_COSE_FILE: &'static str = "author.cose";
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
        let package = Package::create(&package_path)?;

        let mut errors = Errors::new();

        validate_and_write_metadata(manifest, build_time, package_name, &package)
            .unwrap_or_else(errors.get_add_err_fn());
        validate_and_write_component(manifest, &package).unwrap_or_else(errors.get_add_err_fn());
        validate_and_write_config(manifest, &package).unwrap_or_else(errors.get_add_err_fn());
        validate_and_write_settings(manifest, &package).unwrap_or_else(errors.get_add_err_fn());
        write_share_dir(manifest, &package).unwrap_or_else(errors.get_add_err_fn());

        if !errors.is_empty() {
            std::fs::remove_file(package_path).unwrap_or_else(errors.get_add_err_fn());
        }

        errors.return_result(Self(package))
    }

    /// Open an existing WASM module package.
    pub(crate) fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let package = Package::open(path)?;
        Ok(Self(package))
    }

    /// Validate package with its signature and other contents.
    pub(crate) fn validate(&self) -> anyhow::Result<()> {
        let mut errors = Errors::new();

        self.get_metadata()
            .map_or_else(errors.get_add_err_fn(), |_| ());
        self.get_component()
            .map_or_else(errors.get_add_err_fn(), |_| ());
        self.get_config_with_schema()
            .map_or_else(errors.get_add_err_fn(), |_| ());
        self.get_settings_schema()
            .map_or_else(errors.get_add_err_fn(), |_| ());

        self.verify_sign().unwrap_or_else(errors.get_add_err_fn());

        errors.return_result(())
    }

    /// Verify package signature if it exists.
    fn verify_sign(&self) -> anyhow::Result<()> {
        if let Some(signature) = self.get_signature()? {
            let expected_payload = self.get_signature_payload()?;
            let signature_payload = signature.payload();
            anyhow::ensure!(
                &expected_payload == signature_payload,
                "Signature payload mismatch.\nExpected: {}\nGot: {}",
                expected_payload.to_json().to_string(),
                signature_payload.to_json().to_string()
            );
            signature.verify()?;
        }
        Ok(())
    }

    /// Sign the package and store signature inside it.
    /// If signature already exists it will be extended with a new signature.
    pub(crate) fn sign(
        &self, private_key: &PrivateKey, certificate: &Certificate,
    ) -> anyhow::Result<()> {
        let mut signature = if let Some(existing_signature) = self.get_signature()? {
            self.0.remove_file(Self::AUTHOR_COSE_FILE)?;
            existing_signature
        } else {
            let signature_payload = self.get_signature_payload()?;
            Signature::new(signature_payload)
        };

        signature.add_sign(private_key, certificate)?;

        let signature_bytes = signature.to_bytes()?;
        let signature_resource =
            BytesResource::new(Self::AUTHOR_COSE_FILE.to_string(), signature_bytes);

        self.0
            .copy_file(&signature_resource, Self::AUTHOR_COSE_FILE)
    }

    /// Build and return `SignaturePayload`.
    fn get_signature_payload(&self) -> anyhow::Result<SignaturePayload> {
        let metadata_hash = self
            .0
            .get_file_hash(Self::METADATA_FILE)?
            .ok_or(MissingPackageFileError(Self::METADATA_FILE.to_string()))?;
        let component_hash = self
            .0
            .get_file_hash(Self::COMPONENT_FILE)?
            .ok_or(MissingPackageFileError(Self::COMPONENT_FILE.to_string()))?;

        let mut signature_payload_builder =
            SignaturePayloadBuilder::new(metadata_hash.clone(), component_hash.clone());

        if let Some(config_hash) = self.0.get_file_hash(Self::CONFIG_FILE)? {
            signature_payload_builder.with_config_file(config_hash);
        }
        if let Some(config_schema_hash) = self.0.get_file_hash(Self::CONFIG_SCHEMA_FILE)? {
            signature_payload_builder.with_config_schema(config_schema_hash);
        }
        if let Some(setting_schema_hash) = self.0.get_file_hash(Self::SETTINGS_SCHEMA_FILE)? {
            signature_payload_builder.with_settings_schema(setting_schema_hash);
        }
        if let Some(share_hash) = self.0.get_dir_hash(Self::SHARE_DIR)? {
            signature_payload_builder.with_share(share_hash);
        }

        Ok(signature_payload_builder.build())
    }

    /// Get `Metadata` object from package.
    pub(crate) fn get_metadata(&self) -> anyhow::Result<Metadata> {
        self.0
            .get_file_reader(Self::METADATA_FILE)?
            .map(Metadata::from_reader)
            .ok_or(MissingPackageFileError(Self::METADATA_FILE.to_string()))?
    }

    /// Get `wasm::module::Module` object from package.
    pub(crate) fn get_component(&self) -> anyhow::Result<wasm::module::Module> {
        self.0
            .get_file_reader(Self::COMPONENT_FILE)?
            .map(wasm::module::Module::from_reader)
            .ok_or(MissingPackageFileError(Self::COMPONENT_FILE.to_string()))?
    }

    /// Get `Signature` object from package.
    pub(crate) fn get_signature(&self) -> anyhow::Result<Option<Signature<SignaturePayload>>> {
        self.0
            .get_file_reader(Self::AUTHOR_COSE_FILE)?
            .map(Signature::<SignaturePayload>::from_reader)
            .transpose()
    }

    /// Get `ConfigSchema` object from package.
    pub(crate) fn get_config_schema(&self) -> anyhow::Result<Option<ConfigSchema>> {
        self.0
            .get_file_reader(Self::CONFIG_SCHEMA_FILE)?
            .map(ConfigSchema::from_reader)
            .transpose()
    }

    /// Get `Config` and `ConfigSchema` objects from package if present.
    /// To obtain a valid `Config` object it is needed to get `ConfigSchema` first.
    pub(crate) fn get_config_with_schema(
        &self,
    ) -> anyhow::Result<(Option<Config>, Option<ConfigSchema>)> {
        let Some(config_schema) = self.get_config_schema()? else {
            return Ok((None, None));
        };

        if let Some(reader) = self.0.get_file_reader(Self::CONFIG_FILE)? {
            let config_file = Config::from_reader(reader, config_schema.validator())?;
            Ok((Some(config_file), Some(config_schema)))
        } else {
            Ok((None, Some(config_schema)))
        }
    }

    /// Get `SettingsSchema` object from package if present.
    pub(crate) fn get_settings_schema(&self) -> anyhow::Result<Option<SettingsSchema>> {
        self.0
            .get_file_reader(Self::SETTINGS_SCHEMA_FILE)?
            .map(SettingsSchema::from_reader)
            .transpose()
    }
}

/// Validate metadata.json file and write it to the package.
/// Also updates `Metadata` object by setting `build_date` and `name` properties.
fn validate_and_write_metadata(
    manifest: &Manifest, build_date: DateTime<Utc>, name: &str, package: &Package,
) -> anyhow::Result<()> {
    let resource = &manifest.metadata;
    let metadata_reader = resource.get_reader()?;

    let mut metadata = Metadata::from_reader(metadata_reader)
        .map_err(|err| FileError::from_string(resource.location(), Some(err)))?;
    metadata.set_build_date(build_date);
    metadata.set_name(name);

    let resource = BytesResource::new(resource.name()?, metadata.to_bytes()?);
    package.copy_file(&resource, WasmModulePackage::METADATA_FILE)?;
    Ok(())
}

/// Validate WASM component file and write it to the package.
fn validate_and_write_component(manifest: &Manifest, package: &Package) -> anyhow::Result<()> {
    let resource = &manifest.component;

    let component_reader = resource.get_reader()?;

    wasm::module::Module::from_reader(component_reader)
        .map_err(|err| FileError::from_string(resource.location(), Some(err)))?;

    package.copy_file(resource, WasmModulePackage::COMPONENT_FILE)?;
    Ok(())
}

/// Validate config file and config schema and write them to the package.
fn validate_and_write_config(manifest: &Manifest, package: &Package) -> anyhow::Result<()> {
    if let Some(config) = &manifest.config {
        let config_schema_reader = config.schema.get_reader()?;
        let config_schema = ConfigSchema::from_reader(config_schema_reader)
            .map_err(|err| FileError::from_string(config.schema.location(), Some(err)))?;

        let resource = BytesResource::new(config.schema.name()?, config_schema.to_bytes()?);
        package.copy_file(&resource, WasmModulePackage::CONFIG_SCHEMA_FILE)?;

        if let Some(config_file) = &config.file {
            let config_reader = config_file.get_reader()?;

            let config = Config::from_reader(config_reader, config_schema.validator())
                .map_err(|err| FileError::from_string(config.schema.location(), Some(err)))?;

            let resource = BytesResource::new(config_file.name()?, config.to_bytes()?);
            package.copy_file(&resource, WasmModulePackage::CONFIG_FILE)?;
        }
    }
    Ok(())
}

/// Validate settings schema file and it to the package.
fn validate_and_write_settings(manifest: &Manifest, package: &Package) -> anyhow::Result<()> {
    if let Some(settings) = &manifest.settings {
        let setting_schema_reader = settings.schema.get_reader()?;
        let settings_schema = SettingsSchema::from_reader(setting_schema_reader)
            .map_err(|err| FileError::from_string(settings.schema.location(), Some(err)))?;

        let resource = BytesResource::new(settings.schema.name()?, settings_schema.to_bytes()?);
        package.copy_file(&resource, WasmModulePackage::SETTINGS_SCHEMA_FILE)?;
    }
    Ok(())
}

/// Write share dir to the package.
fn write_share_dir(manifest: &Manifest, package: &Package) -> anyhow::Result<()> {
    if let Some(share_dir) = &manifest.share {
        package.copy_dir_recursively(share_dir, WasmModulePackage::SHARE_DIR)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use temp_dir::TempDir;

    use super::*;
    use crate::packaging::{
        resources::{fs_resource::FsResource, Resource},
        sign::{
            certificate::{self, tests::certificate_str},
            keys::tests::private_key_str,
        },
    };

    fn prepare_default_package_files() -> (Metadata, Vec<u8>, Config, ConfigSchema, SettingsSchema)
    {
        let metadata = Metadata::from_reader(
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
        (
            metadata,
            component.as_bytes().to_vec(),
            config,
            config_schema,
            settings_schema,
        )
    }

    fn prepare_package_dir(
        module_name: String, dir: &TempDir, metadata: &Metadata, component: &[u8], config: &Config,
        config_schema: &ConfigSchema, settings_schema: &SettingsSchema,
    ) -> Manifest {
        let config_path = dir.path().join("config.json");
        let config_schema_path = dir.path().join("config.schema.json");
        let metadata_path = dir.path().join("metadata.json");
        let component_path = dir.path().join("module.wasm");
        let settings_schema_path = dir.path().join("settings.schema.json");

        std::fs::write(
            &metadata_path,
            metadata
                .to_bytes()
                .expect("cannot decode metadata to bytes")
                .as_slice(),
        )
        .expect("Cannot create metadata.json file");
        std::fs::write(&component_path, component).expect("Cannot create module.wasm file");
        std::fs::write(
            &config_path,
            config
                .to_bytes()
                .expect("cannot decode config to bytes")
                .as_slice(),
        )
        .expect("Cannot create config.json file");
        std::fs::write(
            &config_schema_path,
            config_schema
                .to_bytes()
                .expect("cannot decode config schema to bytes")
                .as_slice(),
        )
        .expect("Cannot create config.schema.json file");
        std::fs::write(
            &settings_schema_path,
            settings_schema
                .to_bytes()
                .expect("cannot decode settings schema to bytes")
                .as_slice(),
        )
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

        let (mut metadata, component, config, config_schema, settings_schema) =
            prepare_default_package_files();

        let manifest = prepare_package_dir(
            "module".to_string(),
            &dir,
            &metadata,
            component.as_slice(),
            &config,
            &config_schema,
            &settings_schema,
        );

        let build_time = DateTime::default();
        let package =
            WasmModulePackage::build_from_manifest(&manifest, dir.path(), None, build_time)
                .expect("Cannot create module package");

        assert!(package.validate().is_ok());

        // check metadata JSON file
        metadata.set_name(&manifest.name);
        metadata.set_build_date(build_time);

        let package_metadata = package
            .get_metadata()
            .expect("Cannot get metadata from package");
        assert_eq!(metadata, package_metadata);

        // check component WASM file
        assert!(package.get_component().is_ok());

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

    #[test]
    fn sign_test() {
        let dir = TempDir::new().expect("cannot create temp dir");

        let (mut metadata, component, config, config_schema, settings_schema) =
            prepare_default_package_files();

        let manifest = prepare_package_dir(
            "module".to_string(),
            &dir,
            &metadata,
            component.as_slice(),
            &config,
            &config_schema,
            &settings_schema,
        );

        let build_time = DateTime::default();
        let package =
            WasmModulePackage::build_from_manifest(&manifest, dir.path(), None, build_time)
                .expect("Cannot create module package");

        assert!(package.validate().is_ok());

        assert!(package.get_signature().expect("Package error").is_none());

        let private_key =
            PrivateKey::from_str(&private_key_str()).expect("Cannot create private key");
        let certificate =
            Certificate::from_str(&certificate_str()).expect("Cannot create certificate");
        package
            .sign(&private_key, &certificate)
            .expect("Cannot sign package");
        package
            .sign(&private_key, &certificate)
            .expect("Cannot sign package twice");

        assert!(package.get_signature().expect("Package error").is_some());

        assert!(
            package.validate().is_err(),
            "Missing certificate in the storage."
        );

        certificate::storage::add_certificate(certificate)
            .expect("Failed to add certificate to the storage.");
        assert!(package.validate().is_ok());

        // corrupt payload with the modifying metadata.json file
        metadata.set_name("New name");
        package
            .0
            .remove_file(WasmModulePackage::METADATA_FILE)
            .expect("Failed to remove file");
        package
            .0
            .copy_file(
                &BytesResource::new(
                    WasmModulePackage::METADATA_FILE.to_string(),
                    metadata.to_bytes().expect("Failed to decode metadata."),
                ),
                WasmModulePackage::METADATA_FILE,
            )
            .expect("Failed to copy resource to the package.");

        assert!(package.validate().is_err(), "Corrupted signature payload.");
    }
}
