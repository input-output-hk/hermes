//! Hermes application package.

mod app_builder;
mod author_payload;
mod manifest;
mod module_info;
#[cfg(test)]
mod tests;

pub(crate) use app_builder::build_app;
use chrono::{DateTime, Utc};
pub(crate) use manifest::{Manifest, ManifestModule};
pub(crate) use module_info::AppModuleInfo;

use super::{
    hash::Blake2b256,
    metadata::{Metadata, MetadataSchema},
    module::{self, ModulePackage},
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
        resources::{BytesResource, ResourceTrait},
        Dir, File, Path,
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
    /// Application package 'lib' directory path.
    const LIB_DIR: &'static str = "lib";
    /// Application package metadata file path.
    const METADATA_FILE: &'static str = "metadata.json";
    /// Application package overridden module's config file name.
    const MODULE_CONFIG_FILE: &'static str = "config.json";
    /// Application package overridden module's 'share' dir name.
    const MODULE_SHARE_DIR: &'static str = "share";
    /// Application package `srv` directory name.
    const SRV_DIR: &'static str = "srv";
    /// Application package `srv/share` directory path.
    const SRV_SHARE_DIR: &'static str = "srv/share";
    /// Application package `srv/www` directory path.
    const SRV_WWW_DIR: &'static str = "srv/www";
    /// Application package 'usr' directory path.
    const USR_DIR: &'static str = "usr";
    /// Application package 'usr/lib' directory path.
    const USR_LIB_DIR: &'static str = "usr/lib";

    /// Create a new Hermes application package package from a manifest file.
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

    /// Open an existing application package.
    pub(crate) fn from_file<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Self> {
        let package = Package::open(path)?;
        Ok(Self(package))
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

        match self.get_modules() {
            Ok(modules) => {
                if modules.is_empty()
                    && self.get_www_dir().is_none()
                    && self.get_share_dir().is_none()
                {
                    errors.add_err(anyhow::anyhow!("Invalid package, must contain at least one module or www or share directory"));
                }

                for module_info in modules {
                    let module_name = module_info.get_name();
                    module_info
                        .validate(untrusted)
                        .map_err(|err| {
                            anyhow::anyhow!("Invalid module package `{module_name}`:\n{err}")
                        })
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
                "Application package signature payload mismatch.\nExpected: {}\nGot: {}",
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
        &self,
        private_key: &PrivateKey,
        certificate: &Certificate,
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

        for module_info in self.get_modules()? {
            let module_name = module_info.get_name();
            let module_sign = module_info.get_signature()?.ok_or(anyhow::anyhow!(
                "Module {module_name} not signed, missing author.cose signature"
            ))?;
            let module_sign_hash = Blake2b256::hash(module_sign.to_bytes()?.as_slice());

            let mut signature_payload_module_builder =
                author_payload::SignaturePayloadModuleBuilder::new(
                    module_name.clone(),
                    module_sign_hash,
                );

            let usr_module_config_path: Path = format!(
                "{}/{}/{}",
                Self::USR_LIB_DIR,
                module_name,
                Self::MODULE_CONFIG_FILE
            )
            .into();
            if let Some(config_hash) = self.0.calculate_file_hash(usr_module_config_path)? {
                signature_payload_module_builder.with_config(config_hash);
            }

            let usr_module_share_path: Path = format!(
                "{}/{}/{}",
                Self::USR_LIB_DIR,
                module_name,
                Self::MODULE_SHARE_DIR
            )
            .into();
            if let Some(share_hash) = self.0.calculate_dir_hash(&usr_module_share_path)? {
                signature_payload_module_builder.with_share(share_hash);
            }

            signature_payload_builder.with_module(signature_payload_module_builder.build());
        }

        if let Some(www_hash) = self.0.calculate_dir_hash(&Self::SRV_WWW_DIR.into())? {
            signature_payload_builder.with_www(www_hash);
        }
        if let Some(share_hash) = self.0.calculate_dir_hash(&Self::SRV_SHARE_DIR.into())? {
            signature_payload_builder.with_share(share_hash);
        }

        Ok(signature_payload_builder.build())
    }

    /// Get application package name.
    pub(crate) fn get_app_name(&self) -> anyhow::Result<String> {
        self.get_metadata()?.get_name()
    }

    /// Get icon `File` object from package.
    fn get_icon_file(&self) -> anyhow::Result<File> {
        self.0
            .get_file(Self::ICON_FILE.into())
            .map_err(|_| MissingPackageFileError(Self::ICON_FILE.to_string()).into())
    }

    /// Get metadata `File` object from package.
    fn get_metadata_file(&self) -> anyhow::Result<File> {
        self.0
            .get_file(Self::METADATA_FILE.into())
            .map_err(|_| MissingPackageFileError(Self::METADATA_FILE.to_string()).into())
    }

    /// Get `Metadata` object from package.
    pub(crate) fn get_metadata(&self) -> anyhow::Result<Metadata<Self>> {
        self.get_metadata_file().map(Metadata::from_reader)?
    }

    /// Get author `Signature` object from package.
    pub(crate) fn get_author_signature(
        &self
    ) -> anyhow::Result<Option<Signature<author_payload::SignaturePayload>>> {
        self.0
            .get_file(Self::AUTHOR_COSE_FILE.into())
            .ok()
            .map(Signature::from_reader)
            .transpose()
    }

    /// Get `Vec<WasmModulePackage>` from package.
    pub(crate) fn get_modules(&self) -> anyhow::Result<Vec<AppModuleInfo>> {
        let lib_dirs = self.0.get_dirs(&Self::LIB_DIR.into())?;
        let usr_lib = self.0.get_dir(&Self::USR_LIB_DIR.into())?;

        let mut modules = Vec::with_capacity(lib_dirs.len());
        for dir in lib_dirs {
            let name = dir.path().pop_elem();
            let package = ModulePackage::from_package(Package::mount(dir));

            let usr_lib_module = usr_lib.get_dir(&name.as_str().into())?;
            let app_share = usr_lib_module.get_dir(&Self::MODULE_SHARE_DIR.into()).ok();
            let app_config = usr_lib_module
                .get_file(Self::MODULE_CONFIG_FILE.into())
                .ok();

            let module_info = AppModuleInfo::new(name, package, app_config, app_share);
            modules.push(module_info);
        }
        Ok(modules)
    }

    /// Get www dir from package if present.
    fn get_www_dir(&self) -> Option<Dir> {
        self.0.get_dir(&Self::SRV_WWW_DIR.into()).ok()
    }

    /// Get share dir from package if present.
    fn get_share_dir(&self) -> Option<Dir> {
        self.0.get_dir(&Self::SRV_SHARE_DIR.into()).ok()
    }

    /// Validate and write all content of the `Manifest` to the provided `package`.
    fn validate_and_write_from_manifest(
        manifest: &Manifest,
        package: &Package,
        build_date: DateTime<Utc>,
        package_name: &str,
        errors: &mut Errors,
    ) {
        validate_and_write_icon(&manifest.icon.build(), package, Self::ICON_FILE.into())
            .unwrap_or_else(errors.get_add_err_fn());
        validate_and_write_metadata(
            &manifest.metadata.build(),
            build_date,
            package_name,
            package,
            Self::METADATA_FILE.into(),
        )
        .unwrap_or_else(errors.get_add_err_fn());

        package
            .create_dir(Self::LIB_DIR.into())
            .map_or_else(errors.get_add_err_fn(), |_| ());
        package
            .create_dir(Self::USR_DIR.into())
            .map_or_else(errors.get_add_err_fn(), |_| ());
        package
            .create_dir(Self::USR_LIB_DIR.into())
            .map_or_else(errors.get_add_err_fn(), |_| ());
        for module in &manifest.modules {
            validate_and_write_module(
                module,
                package,
                &Self::LIB_DIR.into(),
                &Self::USR_LIB_DIR.into(),
                Self::MODULE_CONFIG_FILE,
                Self::MODULE_SHARE_DIR,
            )
            .unwrap_or_else(errors.get_add_err_fn());
        }

        package
            .create_dir(Self::SRV_DIR.into())
            .map_or_else(errors.get_add_err_fn(), |_| ());
        if let Some(www_dir) = &manifest.www {
            write_www_dir(&www_dir.build(), package, Self::SRV_WWW_DIR.into())
                .unwrap_or_else(errors.get_add_err_fn());
        }
        if let Some(share_dir) = &manifest.share {
            write_share_dir(&share_dir.build(), package, Self::SRV_SHARE_DIR.into())
                .unwrap_or_else(errors.get_add_err_fn());
        }
    }
}

/// Validate icon.svg file and write it to the package to the provided dir path.
fn validate_and_write_icon(
    resource: &impl ResourceTrait,
    dir: &Dir,
    path: Path,
) -> anyhow::Result<()> {
    // Perform validation before copy resource to a file.
    if !is_svg_file(resource, &path)? {
        return Err(anyhow::anyhow!("Invalid icon, not a svg file"));
    }
    dir.copy_resource_file(resource, path)?;
    Ok(())
}

/// Validate svg file.
fn is_svg_file(
    resource: &impl ResourceTrait,
    path: &Path,
) -> anyhow::Result<bool> {
    if !path.to_string().ends_with("svg") {
        return Ok(false);
    }

    let mut reader = resource.get_reader()?;
    let mut buf = Vec::new();
    std::io::copy(&mut reader, &mut buf)?;

    let opt = usvg::Options::default();
    match usvg::Tree::from_data(&buf, &opt) {
        Ok(_) => Ok(true),
        Err(e) => {
            match e {
                // Fail to parse svg file == not a svg file.
                usvg::Error::ParsingFailed(_) => Ok(false),
                _ => Err(e.into()),
            }
        },
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

    let mut metadata = Metadata::<ApplicationPackage>::from_reader(metadata_reader)
        .map_err(|err| FileError::from_string(resource.to_string(), Some(err)))?;
    metadata.set_build_date(build_date);
    metadata.set_name(name);

    let resource = BytesResource::new(resource.name()?, metadata.to_bytes()?);
    dir.copy_resource_file(&resource, path)?;
    Ok(())
}

/// Validate WASM module package and write it to the package to the provided dir path.
fn validate_and_write_module(
    manifest: &ManifestModule,
    dir: &Dir,
    modules_path: &Path,
    usr_modules_path: &Path,
    config_file_name: &str,
    share_dir_name: &str,
) -> anyhow::Result<()> {
    let module_package = ModulePackage::from_file(manifest.package.upload_to_fs())?;
    module_package.validate(true)?;

    let module_original_name = module_package.get_metadata()?.get_name()?;
    let module_name = manifest.name.clone().unwrap_or(module_original_name);

    let modules_dir = dir.get_dir(modules_path)?;
    let module_package_dir = modules_dir.create_dir(module_name.as_str().into())?;
    module_package.copy_to_dir(&module_package_dir, &Path::default())?;

    let usr_modules_dir = dir.get_dir(usr_modules_path)?;
    let module_overridable_dir = usr_modules_dir.create_dir(module_name.as_str().into())?;

    if let Some(config) = &manifest.config {
        let config_schema = module_package.get_config_schema()?.ok_or(anyhow::anyhow!(
            "Missing config schema for module {module_name}"
        ))?;

        module::validate_and_write_config_file(
            &config.build(),
            &config_schema,
            &module_overridable_dir,
            config_file_name.into(),
        )?;
    }
    if let Some(share_dir) = &manifest.share {
        module::write_share_dir(
            &share_dir.build(),
            &module_overridable_dir,
            share_dir_name.into(),
        )?;
    }
    Ok(())
}

/// Write www dir to the package to the provided dir path to the provided dir path.
fn write_www_dir(
    resource: &impl ResourceTrait,
    dir: &Dir,
    path: Path,
) -> anyhow::Result<()> {
    let www_dir = dir.create_dir(path)?;
    www_dir.copy_resource_dir(resource, &Path::default())?;
    Ok(())
}

/// Write share dir to the package to the provided dir path.
fn write_share_dir(
    resource: &impl ResourceTrait,
    dir: &Dir,
    path: Path,
) -> anyhow::Result<()> {
    let share_dir = dir.create_dir(path)?;
    share_dir.copy_resource_dir(resource, &Path::default())?;
    Ok(())
}
