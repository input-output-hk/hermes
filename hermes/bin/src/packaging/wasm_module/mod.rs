//! Hermes WASM module package.

mod author_payload;
mod config;
pub(crate) mod manifest;
mod settings;

use author_payload::{SignaturePayload, SignaturePayloadBuilder};
use chrono::{DateTime, Utc};
use config::{Config, ConfigSchema};
use manifest::{Manifest, ManifestConfig};
use settings::SettingsSchema;

use crate::{
    errors::Errors,
    hdf5::{
        resources::{bytes::BytesResource, ResourceTrait},
        Dir, Path,
    },
    packaging::{
        metadata::{Metadata, MetadataSchema},
        package::Package,
        sign::{
            certificate::Certificate,
            keys::PrivateKey,
            signature::{Signature, SignaturePayloadEncoding},
        },
        FileError, MissingPackageFileError,
    },
    wasm,
};

/// Hermes WASM module package.
pub(crate) struct WasmModulePackage(Package);

impl MetadataSchema for WasmModulePackage {
    const METADATA_SCHEMA: &'static str =
        include_str!("../../../../schemas/hermes_module_metadata.schema.json");
}

impl WasmModulePackage {
    /// WASM module package signature file path.
    const AUTHOR_COSE_FILE: &'static str = "author.cose";
    /// WASM module package component file path.
    const COMPONENT_FILE: &'static str = "module.wasm";
    /// WASM module package config file path.
    pub(crate) const CONFIG_FILE: &'static str = "config.json";
    /// WASM module package config schema file path.
    const CONFIG_SCHEMA_FILE: &'static str = "config.schema.json";
    /// WASM module package file extension.
    pub(crate) const FILE_EXTENSION: &'static str = "hmod";
    /// WASM module package metadata file path.
    const METADATA_FILE: &'static str = "metadata.json";
    /// WASM module package settings schema file path.
    const SETTINGS_SCHEMA_FILE: &'static str = "settings.schema.json";
    /// WASM module package share directory path.
    pub(crate) const SHARE_DIR: &'static str = "share";

    /// Create a new WASM module package from a manifest file.
    pub(crate) fn build_from_manifest<P: AsRef<std::path::Path>>(
        manifest: &Manifest, output_path: P, package_name: Option<&str>, build_date: DateTime<Utc>,
    ) -> anyhow::Result<Self> {
        let package_name = package_name.unwrap_or(&manifest.name);
        let mut package_path = output_path.as_ref().join(package_name);
        package_path.set_extension(Self::FILE_EXTENSION);
        let package = Package::create(&package_path)?;

        let mut errors = Errors::new();
        validate_and_write_from_manifest(manifest, &package, build_date, package_name, &mut errors);
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

    /// Create `WasmModulePackage` from a `Package`.
    pub(crate) fn from_package(package: Package) -> Self {
        Self(package)
    }

    /// Validate package with its signature and other contents.
    /// If `untrasted` flag is `true` the signature will not be verified.
    pub(crate) fn validate(&self, untrasted: bool) -> anyhow::Result<()> {
        let mut errors = Errors::new();

        self.get_metadata()
            .map_or_else(errors.get_add_err_fn(), |_| ());
        self.get_component()
            .map_or_else(errors.get_add_err_fn(), |_| ());
        self.get_config_with_schema()
            .map_or_else(errors.get_add_err_fn(), |_| ());
        self.get_settings_schema()
            .map_or_else(errors.get_add_err_fn(), |_| ());

        if !untrasted {
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
                "Signature payload mismatch.\nExpected: {}\nGot: {}",
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
        &self, private_key: &PrivateKey, certificate: &Certificate,
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

    /// Get `Metadata` object from package.
    pub(crate) fn get_metadata(&self) -> anyhow::Result<Metadata<Self>> {
        self.0
            .get_file(Self::METADATA_FILE.into())
            .map_err(|_| MissingPackageFileError(Self::METADATA_FILE.to_string()))
            .map(|file| Metadata::<Self>::from_reader(file.reader()?))?
    }

    /// Get `wasm::module::Module` object from package.
    pub(crate) fn get_component(&self) -> anyhow::Result<wasm::module::Module> {
        self.0
            .get_file(Self::COMPONENT_FILE.into())
            .map_err(|_| MissingPackageFileError(Self::METADATA_FILE.to_string()))
            .map(|file| wasm::module::Module::from_reader(file.reader()?))?
    }

    /// Get `Signature` object from package.
    pub(crate) fn get_signature(&self) -> anyhow::Result<Option<Signature<SignaturePayload>>> {
        self.0
            .get_file(Self::AUTHOR_COSE_FILE.into())
            .ok()
            .map(|file| Signature::<SignaturePayload>::from_reader(file.reader()?))
            .transpose()
    }

    /// Get `ConfigSchema` object from package.
    pub(crate) fn get_config_schema(&self) -> anyhow::Result<Option<ConfigSchema>> {
        self.0
            .get_file(Self::CONFIG_SCHEMA_FILE.into())
            .ok()
            .map(|file| ConfigSchema::from_reader(file.reader()?))
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

        if let Ok(file) = self.0.get_file(Self::CONFIG_FILE.into()) {
            let config_file = Config::from_reader(file.reader()?, config_schema.validator())?;
            Ok((Some(config_file), Some(config_schema)))
        } else {
            Ok((None, Some(config_schema)))
        }
    }

    /// Get `SettingsSchema` object from package if present.
    pub(crate) fn get_settings_schema(&self) -> anyhow::Result<Option<SettingsSchema>> {
        self.0
            .get_file(Self::SETTINGS_SCHEMA_FILE.into())
            .ok()
            .map(|file| SettingsSchema::from_reader(file.reader()?))
            .transpose()
    }

    /// Get share dir from package if present.
    #[allow(dead_code)]
    pub(crate) fn get_share(&self) -> Option<Dir> {
        self.0.get_dir(&Self::SHARE_DIR.into()).ok()
    }

    /// Copy all content of the `WasmModulePackage` to the provided `package`.
    pub(crate) fn copy_to_package(&self, package: &Package, path: &Path) -> anyhow::Result<()> {
        package.copy_dir(&self.0, path)
    }
}

/// Validate and write all content of the `Manifest` to the provided `package`.
fn validate_and_write_from_manifest(
    manifest: &Manifest, package: &Package, build_date: DateTime<Utc>, package_name: &str,
    errors: &mut Errors,
) {
    validate_and_write_metadata(
        manifest.metadata.build(),
        build_date,
        package_name,
        package,
        Path::default(),
    )
    .unwrap_or_else(errors.get_add_err_fn());

    validate_and_write_component(manifest.component.build(), package, Path::default())
        .unwrap_or_else(errors.get_add_err_fn());

    if let Some(config) = &manifest.config {
        validate_and_write_config(config, package).unwrap_or_else(errors.get_add_err_fn());
    }

    if let Some(settings) = &manifest.settings {
        validate_and_write_settings_schema(settings.schema.build(), package)
            .unwrap_or_else(errors.get_add_err_fn());
    }

    if let Some(share_dir) = &manifest.share {
        write_share_dir(share_dir.build(), package, Path::default())
            .unwrap_or_else(errors.get_add_err_fn());
    }
}

/// Validate metadata.json file and write it to the package to the provided dir path.
/// Also updates `Metadata` object by setting `build_date` and `name` properties.
fn validate_and_write_metadata(
    resource: &impl ResourceTrait, build_date: DateTime<Utc>, name: &str, package: &Package,
    mut path: Path,
) -> anyhow::Result<()> {
    let metadata_reader = resource.get_reader()?;

    let mut metadata = Metadata::<WasmModulePackage>::from_reader(metadata_reader)
        .map_err(|err| FileError::from_string(resource.to_string(), Some(err)))?;
    metadata.set_build_date(build_date);
    metadata.set_name(name);

    let resource = BytesResource::new(resource.name()?, metadata.to_bytes()?);
    path.push_elem(WasmModulePackage::METADATA_FILE.into());
    package.copy_resource_file(&resource, path)?;
    Ok(())
}

/// Validate WASM component file and write it to the package to the provided dir path.
fn validate_and_write_component(
    resource: &impl ResourceTrait, package: &Package, mut path: Path,
) -> anyhow::Result<()> {
    let component_reader = resource.get_reader()?;

    wasm::module::Module::from_reader(component_reader)
        .map_err(|err| FileError::from_string(resource.to_string(), Some(err)))?;

    path.push_elem(WasmModulePackage::COMPONENT_FILE.into());
    package.copy_resource_file(resource, path)?;
    Ok(())
}

/// Validate config schema and config file and write them to the package.
fn validate_and_write_config(manifest: &ManifestConfig, package: &Package) -> anyhow::Result<()> {
    let config_schema =
        validate_and_write_config_schema(manifest.schema.build(), package, Path::default())?;
    if let Some(config_file) = &manifest.file {
        validate_and_write_config_file(
            config_file.build(),
            &config_schema,
            package,
            Path::default(),
        )?;
    }
    Ok(())
}

/// Validate config schema and write it to the package to the provided dir path.
fn validate_and_write_config_schema(
    resource: &impl ResourceTrait, package: &Package, mut path: Path,
) -> anyhow::Result<ConfigSchema> {
    let config_schema_reader = resource.get_reader()?;
    let config_schema = ConfigSchema::from_reader(config_schema_reader)
        .map_err(|err| FileError::from_string(resource.to_string(), Some(err)))?;

    let resource = BytesResource::new(resource.name()?, config_schema.to_bytes()?);
    path.push_elem(WasmModulePackage::CONFIG_SCHEMA_FILE.into());
    package.copy_resource_file(&resource, path)?;

    Ok(config_schema)
}

/// Validate config file and write it to the package.
pub(crate) fn validate_and_write_config_file(
    resource: &impl ResourceTrait, config_schema: &ConfigSchema, package: &Package, mut path: Path,
) -> anyhow::Result<()> {
    let config_reader = resource.get_reader()?;

    let config = Config::from_reader(config_reader, config_schema.validator())
        .map_err(|err| FileError::from_string(resource.to_string(), Some(err)))?;

    let resource = BytesResource::new(resource.name()?, config.to_bytes()?);
    path.push_elem(WasmModulePackage::CONFIG_FILE.into());
    package.copy_resource_file(&resource, path)?;
    Ok(())
}

/// Validate settings schema file and it to the package.
fn validate_and_write_settings_schema(
    resource: &impl ResourceTrait, package: &Package,
) -> anyhow::Result<()> {
    let setting_schema_reader = resource.get_reader()?;
    let settings_schema = SettingsSchema::from_reader(setting_schema_reader)
        .map_err(|err| FileError::from_string(resource.to_string(), Some(err)))?;

    let resource = BytesResource::new(resource.name()?, settings_schema.to_bytes()?);
    package.copy_resource_file(&resource, WasmModulePackage::SETTINGS_SCHEMA_FILE.into())?;
    Ok(())
}

/// Write share dir to the package.
pub(crate) fn write_share_dir(
    resource: &impl ResourceTrait, package: &Package, mut path: Path,
) -> anyhow::Result<()> {
    path.push_elem(WasmModulePackage::SHARE_DIR.into());
    package.copy_resource_dir(resource, &path)?;
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use temp_dir::TempDir;

    use super::*;
    use crate::{
        hdf5::resources::{FsResource, ResourceBuilder},
        packaging::sign::{
            certificate::{self, tests::certificate_str},
            keys::tests::private_key_str,
        },
    };

    pub(crate) struct ModulePackageFiles {
        pub(crate) metadata: Metadata<WasmModulePackage>,
        pub(crate) component: Vec<u8>,
        pub(crate) config_schema: ConfigSchema,
        pub(crate) config: Config,
        pub(crate) settings_schema: SettingsSchema,
    }

    pub(crate) fn prepare_default_package_files() -> ModulePackageFiles {
        let metadata = Metadata::<WasmModulePackage>::from_reader(
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
            ).to_string().as_bytes(),
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

        ModulePackageFiles {
            metadata,
            component: component.as_bytes().to_vec(),
            config_schema,
            config,
            settings_schema,
        }
    }

    pub(crate) fn prepare_package_dir(
        module_name: String, dir: &TempDir, module_package_files: &ModulePackageFiles,
    ) -> Manifest {
        let config_path = dir.path().join("config.json");
        let config_schema_path = dir.path().join("config.schema.json");
        let metadata_path = dir.path().join("metadata.json");
        let component_path = dir.path().join("module.wasm");
        let settings_schema_path = dir.path().join("settings.schema.json");

        std::fs::write(
            &metadata_path,
            module_package_files
                .metadata
                .to_bytes()
                .expect("cannot decode metadata to bytes")
                .as_slice(),
        )
        .expect("Cannot create metadata.json file");
        std::fs::write(&component_path, module_package_files.component.as_slice())
            .expect("Cannot create module.wasm file");
        std::fs::write(
            &config_path,
            module_package_files
                .config
                .to_bytes()
                .expect("cannot decode config to bytes")
                .as_slice(),
        )
        .expect("Cannot create config.json file");
        std::fs::write(
            &config_schema_path,
            module_package_files
                .config_schema
                .to_bytes()
                .expect("cannot decode config schema to bytes")
                .as_slice(),
        )
        .expect("Cannot create config.schema.json file");
        std::fs::write(
            &settings_schema_path,
            module_package_files
                .settings_schema
                .to_bytes()
                .expect("cannot decode settings schema to bytes")
                .as_slice(),
        )
        .expect("Cannot create settings.schema.json file");

        Manifest {
            name: module_name,
            metadata: ResourceBuilder::Fs(FsResource::new(metadata_path)),
            component: ResourceBuilder::Fs(FsResource::new(component_path)),
            config: manifest::ManifestConfig {
                file: Some(ResourceBuilder::Fs(FsResource::new(config_path))),
                schema: ResourceBuilder::Fs(FsResource::new(config_schema_path)),
            }
            .into(),
            settings: manifest::ManifestSettings {
                schema: ResourceBuilder::Fs(FsResource::new(settings_schema_path)),
            }
            .into(),
            share: None,
        }
    }

    pub(crate) fn check_module_integrity(
        module_files: &ModulePackageFiles, module_package: &WasmModulePackage,
    ) {
        let package_metadata = module_package
            .get_metadata()
            .expect("Cannot get metadata from package");
        assert_eq!(module_files.metadata, package_metadata);

        // check component WASM file
        assert!(module_package.get_component().is_ok());

        // check config and config schema JSON files
        let (package_config, package_config_schema) = module_package
            .get_config_with_schema()
            .expect("Cannot get config from package");
        assert_eq!(
            module_files.config,
            package_config.expect("Missing config in package")
        );
        assert_eq!(
            module_files.config_schema,
            package_config_schema.expect("Missing config schema in package")
        );

        // check settings schema JSON file
        let package_settings_schema = module_package
            .get_settings_schema()
            .expect("Cannot get settings schema from package");
        assert_eq!(
            module_files.settings_schema,
            package_settings_schema.expect("Missing settings schema in package")
        );
    }

    #[test]
    fn from_dir_test() {
        let dir = TempDir::new().expect("cannot create temp dir");

        let mut module_package_files = prepare_default_package_files();

        let manifest = prepare_package_dir("module".to_string(), &dir, &module_package_files);

        let build_time = DateTime::default();
        let package =
            WasmModulePackage::build_from_manifest(&manifest, dir.path(), None, build_time)
                .expect("Cannot create module package");

        assert!(package.validate(true).is_ok());

        // check metadata JSON file
        module_package_files.metadata.set_name(&manifest.name);
        module_package_files.metadata.set_build_date(build_time);

        check_module_integrity(&module_package_files, &package);
    }

    #[test]
    fn sign_test() {
        let dir = TempDir::new().expect("cannot create temp dir");

        let mut module_package_files = prepare_default_package_files();

        let manifest = prepare_package_dir("module".to_string(), &dir, &module_package_files);

        let build_time = DateTime::default();
        let package =
            WasmModulePackage::build_from_manifest(&manifest, dir.path(), None, build_time)
                .expect("Cannot create module package");

        assert!(package.validate(true).is_ok());
        assert!(package.validate(false).is_err());
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
            .expect("Cannot sign package twice with the same private key");

        assert!(package.get_signature().expect("Package error").is_some());

        assert!(
            package.validate(false).is_err(),
            "Missing certificate in the storage."
        );

        certificate::storage::add_certificate(certificate)
            .expect("Failed to add certificate to the storage.");
        assert!(package.validate(false).is_ok());

        // corrupt payload with the modifying metadata.json file
        module_package_files.metadata.set_name("New name");
        package
            .0
            .remove_file(WasmModulePackage::METADATA_FILE.into())
            .expect("Failed to remove file");
        package
            .0
            .copy_resource_file(
                &BytesResource::new(
                    WasmModulePackage::METADATA_FILE.to_string(),
                    module_package_files
                        .metadata
                        .to_bytes()
                        .expect("Failed to decode metadata."),
                ),
                WasmModulePackage::METADATA_FILE.into(),
            )
            .expect("Failed to copy resource to the package.");

        assert!(
            package.validate(false).is_err(),
            "Corrupted signature payload."
        );
    }
}
