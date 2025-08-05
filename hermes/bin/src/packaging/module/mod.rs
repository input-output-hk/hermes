//! Hermes WASM module package.

mod author_payload;
mod config;
mod config_info;
mod manifest;
mod settings;
#[cfg(test)]
pub(crate) mod tests;

pub(crate) use author_payload::{SignaturePayload, SignaturePayloadBuilder};
use chrono::{DateTime, Utc};
pub(crate) use config::{Config, ConfigSchema};
pub(crate) use config_info::ConfigInfo;
pub(crate) use manifest::{Manifest, ManifestConfig};
pub(crate) use settings::SettingsSchema;

use super::{
    metadata::{Metadata, MetadataSchema},
    package::Package,
    sign::{
        certificate::Certificate,
        keys::PrivateKey,
        signature::{Signature, SignaturePayloadEncoding},
    },
    FileError, MissingPackageFileError,
};
use crate::{
    errors::Errors,
    hdf5::{
        resources::{bytes::BytesResource, ResourceTrait},
        Dir, File, Path,
    },
    wasm::module::Module,
};

/// Hermes WASM module package.
pub(crate) struct ModulePackage(Package);

impl MetadataSchema for ModulePackage {
    const METADATA_SCHEMA: &'static str =
        include_str!("../../../../schemas/hermes_module_metadata.schema.json");
}

impl ModulePackage {
    /// Module package signature file path.
    const AUTHOR_COSE_FILE: &'static str = "author.cose";
    /// Module package WASM component file path.
    const COMPONENT_FILE: &'static str = "module.wasm";
    /// Module package config file path.
    const CONFIG_FILE: &'static str = "config.json";
    /// Module package config schema file path.
    const CONFIG_SCHEMA_FILE: &'static str = "config.schema.json";
    /// Module package file extension.
    pub(crate) const FILE_EXTENSION: &'static str = "hmod";
    /// Module package metadata file path.
    const METADATA_FILE: &'static str = "metadata.json";
    /// Module package settings schema file path.
    const SETTINGS_SCHEMA_FILE: &'static str = "settings.schema.json";
    /// Module package share directory path.
    const SHARE_DIR: &'static str = "share";

    /// Create a new module package from a manifest file.
    pub(crate) fn build_from_manifest<P: AsRef<std::path::Path>>(
        manifest: &Manifest,
        output_path: P,
        package_name: Option<&str>,
        build_date: DateTime<Utc>,
    ) -> anyhow::Result<Self> {
        let package_name = package_name.unwrap_or(&manifest.name);
        let mut package_path = output_path.as_ref().join(package_name);
        package_path.set_extension(Self::FILE_EXTENSION);
        let package = Package::create(&package_path)?;

        let mut errors = Errors::new();
        Self::validate_and_write_from_manifest(
            manifest,
            &package,
            build_date,
            package_name,
            &mut errors,
        );
        if !errors.is_empty() {
            std::fs::remove_file(package_path)?;
        }

        errors.return_result(Self(package))
    }

    /// Open an existing WASM module package.
    pub(crate) fn from_file<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Self> {
        let package = Package::open(path)?;
        Ok(Self(package))
    }

    /// Create `ModulePackage` from a `Package`.
    pub(crate) fn from_package(package: Package) -> Self {
        Self(package)
    }

    /// Validate package with its signature and other contents.
    /// If `untrusted` flag is `true` the signature will not be verified.
    pub(crate) fn validate(
        &self,
        untrusted: bool,
    ) -> anyhow::Result<()> {
        let mut errors = Errors::new();

        self.get_metadata()
            .map_or_else(errors.get_add_err_fn(), |_| ());
        self.get_component()
            .map_or_else(errors.get_add_err_fn(), |_| ());
        self.get_config_info()
            .map_or_else(errors.get_add_err_fn(), |_| ());
        self.get_settings_schema()
            .map_or_else(errors.get_add_err_fn(), |_| ());

        if !untrusted {
            self.verify_sign().unwrap_or_else(errors.get_add_err_fn());
        }

        errors.return_result(())
    }

    /// Verify package signature.
    fn verify_sign(&self) -> anyhow::Result<()> {
        if let Some(signature) = self.get_signature()? {
            let expected_payload = self.get_signature_payload()?;
            let signature_payload = signature.payload();
            anyhow::ensure!(
                &expected_payload == signature_payload,
                "Module package signature payload mismatch.\nExpected: {}\nGot: {}",
                expected_payload.to_json().to_string(),
                signature_payload.to_json().to_string()
            );
            signature.verify()?;
            Ok(())
        } else {
            Err(MissingPackageFileError(Self::AUTHOR_COSE_FILE.to_string()).into())
        }
    }

    /// Sign the package and store signature inside it.
    /// If signature already exists it will be extended with a new signature.
    pub(crate) fn sign(
        &self,
        private_key: &PrivateKey,
        certificate: &Certificate,
    ) -> anyhow::Result<()> {
        let mut signature = if let Some(existing_signature) = self.get_signature()? {
            self.0.remove_file(Self::AUTHOR_COSE_FILE.into())?;
            existing_signature
        } else {
            let payload = self.get_signature_payload()?;
            Signature::new(payload)
        };

        signature.add_sign(private_key, certificate)?;

        let signature_bytes = signature.to_bytes()?;
        let signature_resource =
            BytesResource::new(Self::AUTHOR_COSE_FILE.to_string(), signature_bytes);

        self.0
            .copy_resource_file(&signature_resource, Self::AUTHOR_COSE_FILE.into())
    }

    /// Build and return `SignaturePayload`.
    fn get_signature_payload(&self) -> anyhow::Result<SignaturePayload> {
        let metadata_hash = self
            .0
            .calculate_file_hash(Self::METADATA_FILE.into())?
            .ok_or(MissingPackageFileError(Self::METADATA_FILE.to_string()))?;
        let component_hash = self
            .0
            .calculate_file_hash(Self::COMPONENT_FILE.into())?
            .ok_or(MissingPackageFileError(Self::COMPONENT_FILE.to_string()))?;

        let mut signature_payload_builder =
            SignaturePayloadBuilder::new(metadata_hash.clone(), component_hash.clone());

        if let Some(config_hash) = self.0.calculate_file_hash(Self::CONFIG_FILE.into())? {
            signature_payload_builder.with_config_file(config_hash);
        }
        if let Some(config_schema_hash) = self
            .0
            .calculate_file_hash(Self::CONFIG_SCHEMA_FILE.into())?
        {
            signature_payload_builder.with_config_schema(config_schema_hash);
        }
        if let Some(setting_schema_hash) = self
            .0
            .calculate_file_hash(Self::SETTINGS_SCHEMA_FILE.into())?
        {
            signature_payload_builder.with_settings_schema(setting_schema_hash);
        }
        if let Some(share_hash) = self.0.calculate_dir_hash(&Self::SHARE_DIR.into())? {
            signature_payload_builder.with_share(share_hash);
        }

        Ok(signature_payload_builder.build())
    }

    /// Get metadata `File` object from package.
    pub(super) fn get_metadata_file(&self) -> anyhow::Result<File> {
        self.0
            .get_file(Self::METADATA_FILE.into())
            .map_err(|_| MissingPackageFileError(Self::METADATA_FILE.to_string()).into())
    }

    /// Get `Metadata` object from package.
    pub(crate) fn get_metadata(&self) -> anyhow::Result<Metadata<Self>> {
        self.get_metadata_file().map(Metadata::from_reader)?
    }

    /// Get component `File` object from package.
    pub(super) fn get_component_file(&self) -> anyhow::Result<File> {
        self.0
            .get_file(Self::COMPONENT_FILE.into())
            .map_err(|_| MissingPackageFileError(Self::METADATA_FILE.to_string()).into())
    }

    /// Get `wasm::module::Module` object from package.
    pub(crate) fn get_component(&self) -> anyhow::Result<Module> {
        self.get_component_file().map(Module::from_reader)?
    }

    /// Get `Signature` object from package.
    pub(crate) fn get_signature(&self) -> anyhow::Result<Option<Signature<SignaturePayload>>> {
        self.0
            .get_file(Self::AUTHOR_COSE_FILE.into())
            .ok()
            .map(Signature::<SignaturePayload>::from_reader)
            .transpose()
    }

    /// Get config schema `File` object from package.
    pub(super) fn get_config_schema_file(&self) -> Option<File> {
        self.0.get_file(Self::CONFIG_SCHEMA_FILE.into()).ok()
    }

    /// Get `ConfigSchema` object from package.
    pub(crate) fn get_config_schema(&self) -> anyhow::Result<Option<ConfigSchema>> {
        self.get_config_schema_file()
            .map(ConfigSchema::from_reader)
            .transpose()
    }

    /// Get config `File` object from package.
    pub(super) fn get_config_file(&self) -> Option<File> {
        self.0.get_file(Self::CONFIG_FILE.into()).ok()
    }

    /// Get `Config` and `ConfigSchema` objects from package if present.
    pub(crate) fn get_config_info(&self) -> anyhow::Result<Option<ConfigInfo>> {
        let Some(config_schema) = self.get_config_schema()? else {
            return Ok(None);
        };

        if let Some(file) = self.get_config_file() {
            let config_file = Config::from_reader(file, config_schema.validator())?;
            Ok(Some(ConfigInfo {
                schema: config_schema,
                val: Some(config_file),
            }))
        } else {
            Ok(Some(ConfigInfo {
                schema: config_schema,
                val: None,
            }))
        }
    }

    /// Get settings schema `File` object from package if present.
    pub(super) fn get_settings_schema_file(&self) -> Option<File> {
        self.0.get_file(Self::SETTINGS_SCHEMA_FILE.into()).ok()
    }

    /// Get `SettingsSchema` object from package if present.
    pub(crate) fn get_settings_schema(&self) -> anyhow::Result<Option<SettingsSchema>> {
        self.get_settings_schema_file()
            .map(SettingsSchema::from_reader)
            .transpose()
    }

    /// Get share dir from package if present.
    pub(super) fn get_share_dir(&self) -> Option<Dir> {
        self.0.get_dir(&Self::SHARE_DIR.into()).ok()
    }

    /// Copy all content of the `ModulePackage` to the provided `Dir`.
    pub(crate) fn copy_to_dir(
        &self,
        dir: &Dir,
        path: &Path,
    ) -> anyhow::Result<()> {
        dir.copy_dir(&self.0, path)
    }

    /// Validate and write all content of the `Manifest` to the provided `package`.
    fn validate_and_write_from_manifest(
        manifest: &Manifest,
        package: &Package,
        build_date: DateTime<Utc>,
        package_name: &str,
        errors: &mut Errors,
    ) {
        validate_and_write_metadata(
            &manifest.metadata.build(),
            build_date,
            package_name,
            package,
            Self::METADATA_FILE.into(),
        )
        .unwrap_or_else(errors.get_add_err_fn());

        validate_and_write_component(
            &manifest.component.build(),
            package,
            Self::COMPONENT_FILE.into(),
        )
        .unwrap_or_else(errors.get_add_err_fn());

        if let Some(config) = &manifest.config {
            validate_and_write_config(
                config,
                package,
                Self::CONFIG_SCHEMA_FILE.into(),
                Self::CONFIG_FILE.into(),
            )
            .unwrap_or_else(errors.get_add_err_fn());
        }

        if let Some(settings) = &manifest.settings {
            validate_and_write_settings_schema(
                &settings.schema.build(),
                package,
                Self::SETTINGS_SCHEMA_FILE.into(),
            )
            .unwrap_or_else(errors.get_add_err_fn());
        }

        if let Some(share_dir) = &manifest.share {
            write_share_dir(&share_dir.build(), package, Self::SHARE_DIR.into())
                .unwrap_or_else(errors.get_add_err_fn());
        }
    }
}

/// Validate metadata.json file and write it to the package to the provided dir path.
/// Also updates `Metadata` object by setting `build_date` and `name` properties.
fn validate_and_write_metadata(
    resource: &impl ResourceTrait,
    build_date: DateTime<Utc>,
    name: &str,
    dir: &Dir,
    path: Path,
) -> anyhow::Result<()> {
    let metadata_reader = resource.get_reader()?;

    let mut metadata = Metadata::<ModulePackage>::from_reader(metadata_reader)
        .map_err(|err| FileError::from_string(resource.to_string(), Some(err)))?;
    metadata.set_build_date(build_date);
    metadata.set_name(name);

    let resource = BytesResource::new(resource.name()?, metadata.to_bytes()?);
    dir.copy_resource_file(&resource, path)?;
    Ok(())
}

/// Validate WASM component file and write it to the package to the provided dir path.
fn validate_and_write_component(
    resource: &impl ResourceTrait,
    dir: &Dir,
    path: Path,
) -> anyhow::Result<()> {
    let component_reader = resource.get_reader()?;

    Module::from_reader(component_reader)
        .map_err(|err| FileError::from_string(resource.to_string(), Some(err)))?;

    dir.copy_resource_file(resource, path)?;
    Ok(())
}

/// Validate config schema and config file and write them to the package.
fn validate_and_write_config(
    manifest: &ManifestConfig,
    dir: &Dir,
    config_schema_path: Path,
    config_file_path: Path,
) -> anyhow::Result<()> {
    let config_schema =
        validate_and_write_config_schema(&manifest.schema.build(), dir, config_schema_path)?;
    if let Some(config_file) = &manifest.file {
        validate_and_write_config_file(
            &config_file.build(),
            &config_schema,
            dir,
            config_file_path,
        )?;
    }
    Ok(())
}

/// Validate config schema and write it to the package to the provided dir path.
fn validate_and_write_config_schema(
    resource: &impl ResourceTrait,
    dir: &Dir,
    path: Path,
) -> anyhow::Result<ConfigSchema> {
    let config_schema_reader = resource.get_reader()?;
    let config_schema = ConfigSchema::from_reader(config_schema_reader)
        .map_err(|err| FileError::from_string(resource.to_string(), Some(err)))?;

    let resource = BytesResource::new(resource.name()?, config_schema.to_bytes()?);
    dir.copy_resource_file(&resource, path)?;

    Ok(config_schema)
}

/// Validate config file and write it to the package.
pub(crate) fn validate_and_write_config_file(
    resource: &impl ResourceTrait,
    config_schema: &ConfigSchema,
    dir: &Dir,
    path: Path,
) -> anyhow::Result<()> {
    let config_reader = resource.get_reader()?;

    let config = Config::from_reader(config_reader, config_schema.validator())
        .map_err(|err| FileError::from_string(resource.to_string(), Some(err)))?;

    let resource = BytesResource::new(resource.name()?, config.to_bytes()?);
    dir.copy_resource_file(&resource, path)?;
    Ok(())
}

/// Validate settings schema file and it to the package.
fn validate_and_write_settings_schema(
    resource: &impl ResourceTrait,
    dir: &Dir,
    path: Path,
) -> anyhow::Result<()> {
    let setting_schema_reader = resource.get_reader()?;
    let settings_schema = SettingsSchema::from_reader(setting_schema_reader)
        .map_err(|err| FileError::from_string(resource.to_string(), Some(err)))?;

    let resource = BytesResource::new(resource.name()?, settings_schema.to_bytes()?);
    dir.copy_resource_file(&resource, path)?;
    Ok(())
}

/// Write share dir to the package.
pub(crate) fn write_share_dir(
    resource: &impl ResourceTrait,
    dir: &Dir,
    path: Path,
) -> anyhow::Result<()> {
    let share_dir = dir.create_dir(path)?;
    share_dir.copy_resource_dir(resource, &Path::default())?;
    Ok(())
}
