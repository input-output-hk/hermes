//! Hermes application package.

mod author_payload;
pub(crate) mod manifest;

use chrono::{DateTime, Utc};
use manifest::{Manifest, ManifestModule};

use crate::{
    errors::Errors,
    hdf5::{
        resources::{BytesResource, ResourceTrait},
        Dir, Path,
    },
    packaging::{
        hash::Blake2b256,
        metadata::{Metadata, MetadataSchema},
        package::Package,
        sign::{
            certificate::Certificate,
            keys::PrivateKey,
            signature::{Signature, SignaturePayloadEncoding},
        },
        wasm_module::{self, WasmModulePackage},
        FileError, MissingPackageFileError,
    },
};

/// Hermes application package.
pub(crate) struct ApplicationPackage(Package);

impl MetadataSchema for ApplicationPackage {
    const METADATA_SCHEMA: &'static str =
        include_str!("../../../../schemas/hermes_app_metadata.schema.json");
}

impl ApplicationPackage {
    /// Application package signature file path.
    const AUTHOR_COSE_FILE: &'static str = "author.cose";
    /// Application package file extension.
    const FILE_EXTENSION: &'static str = "happ";
    /// Application package icon file path.
    const ICON_FILE: &'static str = "icon.svg";
    /// Application package metadata file path.
    const METADATA_FILE: &'static str = "metadata.json";
    /// Application WASM modules directory path.
    const MODULES_DIR: &'static str = "lib";
    /// Application package `share` directory path.
    const SHARE_DIR: &'static str = "share";
    /// Application package `srv` directory name.
    const SRV_DIR: &'static str = "srv";
    /// Application shareable directory path.
    const USR_DIR: &'static str = "usr";
    /// Application package `www` directory path.
    const WWW_DIR: &'static str = "www";

    /// Create a new Hermes application package package from a manifest file.
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

    /// Open an existing application package.
    pub(crate) fn from_file<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Self> {
        let package = Package::open(path)?;
        Ok(Self(package))
    }

    /// Validate package with its signature and other contents.
    /// If `untrusted` flag is `true` the signature will not be verified.
    pub(crate) fn validate(&self, untrusted: bool) -> anyhow::Result<()> {
        let mut errors = Errors::new();

        self.get_metadata()
            .map_or_else(errors.get_add_err_fn(), |_| ());

        match self.get_modules() {
            Ok(modules) => {
                if modules.is_empty() && self.get_www().is_none() && self.get_share().is_none() {
                    errors.add_err(anyhow::anyhow!("Invalid package, must contain at least one module or www or share directory"));
                }

                for (_, module_package) in modules {
                    module_package
                        .validate(untrusted)
                        .unwrap_or_else(errors.get_add_err_fn());
                }
            },
            Err(err) => errors.add_err(err),
        }

        if !untrusted {
            self.verify_author_sign()
                .unwrap_or_else(errors.get_add_err_fn());
        }

        errors.return_result(())
    }

    /// Verify author package signature.
    fn verify_author_sign(&self) -> anyhow::Result<()> {
        if let Some(signature) = self.get_author_signature()? {
            let expected_payload = self.get_author_signature_payload()?;
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

    /// Sign the package as an author and store signature inside it.
    /// If signature already exists it will be extended with a new signature.
    pub(crate) fn author_sign(
        &self, private_key: &PrivateKey, certificate: &Certificate,
    ) -> anyhow::Result<()> {
        let mut signature = if let Some(existing_signature) = self.get_author_signature()? {
            self.0.remove_file(Self::AUTHOR_COSE_FILE.into())?;
            existing_signature
        } else {
            let payload = self.get_author_signature_payload()?;
            Signature::new(payload)
        };

        signature.add_sign(private_key, certificate)?;

        let signature_bytes = signature.to_bytes()?;
        let signature_resource =
            BytesResource::new(Self::AUTHOR_COSE_FILE.to_string(), signature_bytes);

        self.0
            .copy_resource_file(&signature_resource, Self::AUTHOR_COSE_FILE.into())
    }

    /// Build and return `author_payload::SignaturePayload`.
    fn get_author_signature_payload(&self) -> anyhow::Result<author_payload::SignaturePayload> {
        let metadata_hash = self
            .0
            .calculate_file_hash(Self::METADATA_FILE.into())?
            .ok_or(MissingPackageFileError(Self::METADATA_FILE.to_string()))?;
        let icon_hash = self
            .0
            .calculate_file_hash(Self::ICON_FILE.into())?
            .ok_or(MissingPackageFileError(Self::ICON_FILE.to_string()))?;

        let mut signature_payload_builder =
            author_payload::SignaturePayloadBuilder::new(metadata_hash.clone(), icon_hash.clone());

        let usr_module_path = Path::new(vec![Self::USR_DIR.into(), Self::MODULES_DIR.into()]);
        for (module_name, module_package) in self.get_modules()? {
            let module_sign = module_package.get_signature()?.ok_or(anyhow::anyhow!(
                "Module {module_name} not signed, missing author.cose signature"
            ))?;
            let module_sign_hash = Blake2b256::hash(module_sign.to_bytes()?.as_slice());

            let mut signature_payload_module_builder =
                author_payload::SignaturePayloadModuleBuilder::new(
                    module_name.clone(),
                    module_sign_hash,
                );

            let mut usr_module_config_path = usr_module_path.clone();
            usr_module_config_path.push_elem(WasmModulePackage::CONFIG_FILE.into());
            if let Some(config_hash) = self.0.calculate_file_hash(usr_module_config_path)? {
                signature_payload_module_builder.with_config(config_hash);
            }

            let mut usr_module_share_path = usr_module_path.clone();
            usr_module_share_path.push_elem(WasmModulePackage::SHARE_DIR.into());
            if let Some(share_hash) = self.0.calculate_dir_hash(&usr_module_share_path)? {
                signature_payload_module_builder.with_share(share_hash);
            }

            signature_payload_builder.with_module(signature_payload_module_builder.build());
        }

        if let Some(www_hash) = self.0.calculate_dir_hash(&Self::WWW_DIR.into())? {
            signature_payload_builder.with_www(www_hash);
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
            .map(Metadata::<Self>::from_reader)?
    }

    /// Get author `Signature` object from package.
    pub(crate) fn get_author_signature(
        &self,
    ) -> anyhow::Result<Option<Signature<author_payload::SignaturePayload>>> {
        self.0
            .get_file(Self::AUTHOR_COSE_FILE.into())
            .ok()
            .map(Signature::<author_payload::SignaturePayload>::from_reader)
            .transpose()
    }

    /// Get `Vec<WasmModulePackage>` from package.
    pub(crate) fn get_modules(&self) -> anyhow::Result<Vec<(String, WasmModulePackage)>> {
        let dirs = self.0.get_dirs(&Self::MODULES_DIR.into())?;
        let mut modules = Vec::with_capacity(dirs.len());
        for dir in dirs {
            let dir_name = dir.path().pop_elem()?;
            let package = WasmModulePackage::from_package(Package::mount(dir));
            modules.push((dir_name, package));
        }
        Ok(modules)
    }

    /// Get www dir from package if present.
    pub(crate) fn get_www(&self) -> Option<Dir> {
        self.0.get_dir(&Self::WWW_DIR.into()).ok()
    }

    /// Get share dir from package if present.
    pub(crate) fn get_share(&self) -> Option<Dir> {
        self.0.get_dir(&Self::SHARE_DIR.into()).ok()
    }
}

/// Validate and write all content of the `Manifest` to the provided `package`.
fn validate_and_write_from_manifest(
    manifest: &Manifest, package: &Package, build_date: DateTime<Utc>, package_name: &str,
    errors: &mut Errors,
) {
    validate_and_write_icon(manifest.icon.build(), package).unwrap_or_else(errors.get_add_err_fn());
    validate_and_write_metadata(manifest.metadata.build(), build_date, package_name, package)
        .unwrap_or_else(errors.get_add_err_fn());

    match package.create_dir(ApplicationPackage::MODULES_DIR.into()) {
        Ok(modules_dir) => {
            for module in &manifest.modules {
                validate_and_write_module(module, &modules_dir)
                    .unwrap_or_else(errors.get_add_err_fn());
            }
        },
        Err(err) => errors.add_err(err),
    };

    match package.create_dir(ApplicationPackage::SRV_DIR.into()) {
        Ok(srv_dir) => {
            if let Some(www_dir) = &manifest.www {
                write_www_dir(www_dir.build(), &srv_dir).unwrap_or_else(errors.get_add_err_fn());
            }
            if let Some(share_dir) = &manifest.share {
                write_share_dir(share_dir.build(), &srv_dir)
                    .unwrap_or_else(errors.get_add_err_fn());
            }
        },
        Err(err) => errors.add_err(err),
    };
}

/// Validate icon.svg file and write it to the package to the provided dir path.
fn validate_and_write_icon(resource: &impl ResourceTrait, dir: &Dir) -> anyhow::Result<()> {
    // TODO: https://github.com/input-output-hk/hermes/issues/282
    dir.copy_resource_file(resource, ApplicationPackage::ICON_FILE.into())?;
    Ok(())
}

/// Validate metadata.json file and write it to the package to the provided dir path.
/// Also updates `Metadata` object by setting `build_date` and `name` properties.
fn validate_and_write_metadata(
    resource: &impl ResourceTrait, build_date: DateTime<Utc>, name: &str, dir: &Dir,
) -> anyhow::Result<()> {
    let metadata_reader = resource.get_reader()?;

    let mut metadata = Metadata::<ApplicationPackage>::from_reader(metadata_reader)
        .map_err(|err| FileError::from_string(resource.to_string(), Some(err)))?;
    metadata.set_build_date(build_date);
    metadata.set_name(name);

    let resource = BytesResource::new(resource.name()?, metadata.to_bytes()?);
    dir.copy_resource_file(&resource, ApplicationPackage::METADATA_FILE.into())?;
    Ok(())
}

/// Validate WASM module package and write it to the package to the provided dir path.
fn validate_and_write_module(manifest: &ManifestModule, modules_dir: &Dir) -> anyhow::Result<()> {
    let module_package = WasmModulePackage::from_file(manifest.package.upload_to_fs())?;
    module_package.validate(true)?;

    let module_original_name = module_package.get_metadata()?.get_name()?;
    let module_name = manifest.name.clone().unwrap_or(module_original_name);

    let module_package_dir = modules_dir.create_dir(module_name.as_str().into())?;
    module_package.copy_to_dir(&module_package_dir, Path::default())?;

    let module_overridable_dir = module_package_dir;

    if let Some(config) = &manifest.config {
        let config_schema = module_package.get_config_schema()?.ok_or(anyhow::anyhow!(
            "Missing config schema for module {module_name}"
        ))?;

        wasm_module::validate_and_write_config_file(
            config.build(),
            &config_schema,
            &module_overridable_dir,
        )?;
    }
    if let Some(share_dir) = &manifest.share {
        wasm_module::write_share_dir(share_dir.build(), &module_overridable_dir)?;
    }
    Ok(())
}

/// Write www dir to the package to the provided dir path to the provided dir path.
fn write_www_dir(resource: &impl ResourceTrait, srv_dir: &Dir) -> anyhow::Result<()> {
    let www_dir = srv_dir.create_dir(ApplicationPackage::WWW_DIR.into())?;
    www_dir.copy_resource_dir(resource, Path::default())?;
    Ok(())
}

/// Write share dir to the package to the provided dir path.
fn write_share_dir(resource: &impl ResourceTrait, srv_dir: &Dir) -> anyhow::Result<()> {
    let share_dir = srv_dir.create_dir(ApplicationPackage::SHARE_DIR.into())?;
    share_dir.copy_resource_dir(resource, Path::default())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use temp_dir::TempDir;

    use super::*;
    use crate::{
        hdf5::resources::{FsResource, ResourceBuilder},
        packaging::sign::{
            certificate::{self, tests::certificate_str},
            keys::tests::private_key_str,
        },
    };

    struct ApplicationPackageFiles {
        metadata: Metadata<ApplicationPackage>,
        icon: Vec<u8>,
        modules: Vec<wasm_module::tests::ModulePackageFiles>,
    }

    fn default_module_name(i: usize) -> String {
        format!("module_{i}")
    }

    fn prepare_default_package_files(modules_num: usize) -> ApplicationPackageFiles {
        let metadata = Metadata::<ApplicationPackage>::from_reader(
            serde_json::json!(
                {
                    "$schema": "https://raw.githubusercontent.com/input-output-hk/hermes/main/hermes/schemas/hermes_module_metadata.schema.json",
                    "name": "Test app",
                    "version": "V1.0.0",
                    "description": "Some description",
                    "src": ["https://github.com/input-output-hk/hermes"],
                    "copyright": ["Copyright Ⓒ 2024, IOG Singapore."],
                    "license": [{"spdx": "MIT"}]
                }
            ).to_string().as_bytes(),
        ).expect("Invalid metadata");
        let icon = b"icon_image_svg_content".to_vec();

        let mut modules = Vec::with_capacity(modules_num);
        for _ in 0..modules_num {
            modules.push(wasm_module::tests::prepare_default_package_files());
        }

        ApplicationPackageFiles {
            metadata,
            icon,
            modules,
        }
    }

    fn prepare_package_dir(
        app_name: String, override_module_name: &[String], build_date: DateTime<Utc>,
        dir: &TempDir, app_package_files: &mut ApplicationPackageFiles,
    ) -> Manifest {
        let metadata_path = dir.path().join("metadata.json");
        let icon_path = dir.path().join("icon.png");

        std::fs::write(
            &metadata_path,
            app_package_files
                .metadata
                .to_bytes()
                .expect("Failed to decode metadata to bytes")
                .as_slice(),
        )
        .expect("Failed to create metadata.json file");

        std::fs::write(&icon_path, app_package_files.icon.as_slice())
            .expect("Failed to create metadata.json file");

        let mut modules = Vec::new();
        for (i, module_package_files) in app_package_files.modules.iter_mut().enumerate() {
            let default_module_name = default_module_name(i);
            let mut module_package_path = dir.path().join(&default_module_name);
            module_package_path.set_extension(WasmModulePackage::FILE_EXTENSION);

            let module_manifest = wasm_module::tests::prepare_package_dir(
                default_module_name.clone(),
                dir,
                module_package_files,
            );

            WasmModulePackage::build_from_manifest(&module_manifest, dir.path(), None, build_date)
                .expect("Failed to create module package");

            // WASM module package during the build process updates metadata file
            // to have a corresponded values update `module_package_files`.
            module_package_files
                .metadata
                .set_name(default_module_name.as_str());
            module_package_files.metadata.set_build_date(build_date);

            modules.push(ManifestModule {
                name: override_module_name.get(i).cloned(),
                package: ResourceBuilder::Fs(FsResource::new(module_package_path)),
                config: None,
                share: None,
            });
        }

        Manifest {
            name: app_name,
            icon: ResourceBuilder::Fs(FsResource::new(icon_path)),
            metadata: ResourceBuilder::Fs(FsResource::new(metadata_path)),
            modules,
            www: None,
            share: None,
        }
    }

    #[test]
    fn from_dir_test() {
        let dir = TempDir::new().expect("Failed to create temp dir");

        let modules_num = 4;
        let mut app_package_files = prepare_default_package_files(modules_num);

        // override module names for first 2 modules
        let override_module_name = vec!["test_module_1".into(), "test_module_2".into()];
        let build_date = DateTime::default();
        let manifest = prepare_package_dir(
            "app".to_string(),
            &override_module_name,
            build_date,
            &dir,
            &mut app_package_files,
        );

        let package =
            ApplicationPackage::build_from_manifest(&manifest, dir.path(), None, build_date)
                .expect("Cannot create module package");

        assert!(package.validate(true).is_ok());

        // check metadata JSON file
        app_package_files.metadata.set_name(&manifest.name);
        app_package_files.metadata.set_build_date(build_date);

        let package_metadata = package
            .get_metadata()
            .expect("Cannot get metadata from package");
        assert_eq!(app_package_files.metadata, package_metadata);

        // check WASM modules
        let modules = package
            .get_modules()
            .expect("Failed to get WASM modules from package");
        assert_eq!(modules.len(), app_package_files.modules.len());

        for (app_module_name, module_package) in modules {
            let package_module_name = module_package
                .get_metadata()
                .expect("Cannot get metadata from package")
                .get_name()
                .expect("Failed to get module name");
            let (i, module_files) = app_package_files
                .modules
                .iter_mut()
                .enumerate()
                .find(|(_, module)| {
                    module
                        .metadata
                        .get_name()
                        .expect("Failed to get module name")
                        == *package_module_name
                })
                .expect("Failed to find module in app package files by module name");

            let manifest_module_name = manifest
                .modules
                .get(i)
                .expect("Empty manifest modules")
                .name
                .clone();
            assert_eq!(
                app_module_name,
                manifest_module_name.unwrap_or(
                    module_files
                        .metadata
                        .get_name()
                        .expect("Failed to get module name")
                )
            );

            wasm_module::tests::check_module_integrity(module_files, &module_package);
        }
    }

    #[test]
    fn author_sing_test() {
        let dir = TempDir::new().expect("Failed to create temp dir");

        let modules_num = 4;
        let mut app_package_files = prepare_default_package_files(modules_num);

        // override module names for first 2 modules
        let override_module_name = vec!["test module 1".into(), "test module 2".into()];
        let build_date = DateTime::default();
        let manifest = prepare_package_dir(
            "app".to_string(),
            &override_module_name,
            build_date,
            &dir,
            &mut app_package_files,
        );

        let package =
            ApplicationPackage::build_from_manifest(&manifest, dir.path(), None, build_date)
                .expect("Cannot create module package");

        assert!(package.validate(true).is_ok());
        assert!(package.validate(false).is_err());
        assert!(package
            .get_author_signature()
            .expect("Package error")
            .is_none());

        let private_key =
            PrivateKey::from_str(&private_key_str()).expect("Cannot create private key");
        let certificate =
            Certificate::from_str(&certificate_str()).expect("Cannot create certificate");

        // sign wasm modules packages first
        for (_, module_package) in package.get_modules().expect("Failed to get modules") {
            module_package
                .sign(&private_key, &certificate)
                .expect("Cannot sign module package");
        }

        package
            .author_sign(&private_key, &certificate)
            .expect("Cannot sign package");
        package
            .author_sign(&private_key, &certificate)
            .expect("Cannot sign package twice with the same private key");

        assert!(package
            .get_author_signature()
            .expect("Package error")
            .is_some());

        assert!(
            package.validate(false).is_err(),
            "Missing certificate in the storage."
        );

        certificate::storage::add_certificate(certificate)
            .expect("Failed to add certificate to the storage.");
        assert!(package.validate(false).is_ok());

        // corrupt payload with the modifying metadata.json file
        app_package_files.metadata.set_name("New name");
        package
            .0
            .remove_file(ApplicationPackage::METADATA_FILE.into())
            .expect("Failed to remove file");
        package
            .0
            .copy_resource_file(
                &BytesResource::new(
                    ApplicationPackage::METADATA_FILE.to_string(),
                    app_package_files
                        .metadata
                        .to_bytes()
                        .expect("Failed to decode metadata."),
                ),
                ApplicationPackage::METADATA_FILE.into(),
            )
            .expect("Failed to copy resource to the package.");

        assert!(
            package.validate(false).is_err(),
            "Corrupted signature payload."
        );
    }
}
