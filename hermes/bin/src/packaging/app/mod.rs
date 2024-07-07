//! Hermes application package.

pub(crate) mod manifest;

use std::path::Path;

use chrono::{DateTime, Utc};
use manifest::{Manifest, ManifestModule};

use super::{resources::ResourceTrait, wasm_module::WasmModulePackage};
use crate::{
    errors::Errors,
    packaging::{
        metadata::{Metadata, MetadataSchema},
        package::Package,
        resources::BytesResource,
        FileError, MissingPackageFileError,
    },
};

/// Hermes application package.
pub(crate) struct ApplicationPackage(#[allow(dead_code)] Package);

impl MetadataSchema for ApplicationPackage {
    const METADATA_SCHEMA: &'static str =
        include_str!("../../../../schemas/hermes_app_metadata.schema.json");
}

impl ApplicationPackage {
    /// Hermes application package file extension.
    const FILE_EXTENSION: &'static str = "happ";
    /// Hermes application package icon file path.
    const ICON_FILE: &'static str = "icon.svg";
    /// Hermes application package metadata file path.
    const METADATA_FILE: &'static str = "metadata.json";
    /// Application package share directory path.
    const SHARE_DIR: &'static str = "srv/share";
    /// Application package www directory path.
    const WWW_DIR: &'static str = "srv/www";

    /// Create a new Hermes application package package from a manifest file.
    pub(crate) fn build_from_manifest<P: AsRef<Path>>(
        manifest: &Manifest, output_path: P, package_name: Option<&str>, build_time: DateTime<Utc>,
    ) -> anyhow::Result<Self> {
        let package_name = package_name.unwrap_or(&manifest.name);
        let mut package_path = output_path.as_ref().join(package_name);
        package_path.set_extension(Self::FILE_EXTENSION);
        let package = Package::create(&package_path)?;

        let mut errors = Errors::new();

        validate_and_write_icon(manifest, &package).unwrap_or_else(errors.get_add_err_fn());
        validate_and_write_metadata(manifest, build_time, package_name, &package)
            .unwrap_or_else(errors.get_add_err_fn());
        for module in &manifest.modules {
            validate_and_write_module(module, &package).unwrap_or_else(errors.get_add_err_fn());
        }
        write_www_dir(manifest, &package).unwrap_or_else(errors.get_add_err_fn());
        write_share_dir(manifest, &package).unwrap_or_else(errors.get_add_err_fn());

        if !errors.is_empty() {
            std::fs::remove_file(package_path).unwrap_or_else(errors.get_add_err_fn());
        }

        errors.return_result(Self(package))
    }

    /// Open an existing application package.
    #[allow(dead_code)]
    pub(crate) fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let package = Package::open(path)?;
        Ok(Self(package))
    }

    /// Validate package with its signature and other contents.
    #[allow(dead_code)]
    pub(crate) fn validate(&self) -> anyhow::Result<()> {
        let mut errors = Errors::new();

        self.get_metadata()
            .map_or_else(errors.get_add_err_fn(), |_| ());

        errors.return_result(())
    }

    /// Get `Metadata` object from package.
    pub(crate) fn get_metadata(&self) -> anyhow::Result<Metadata<Self>> {
        self.0
            .get_file_reader(Self::METADATA_FILE.into())?
            .map(Metadata::<Self>::from_reader)
            .ok_or(MissingPackageFileError(Self::METADATA_FILE.to_string()))?
    }
}

/// Validate icon.svg file and write it to the package.
fn validate_and_write_icon(manifest: &Manifest, package: &Package) -> anyhow::Result<()> {
    // TODO: https://github.com/input-output-hk/hermes/issues/282
    package.copy_file(manifest.icon.build(), ApplicationPackage::ICON_FILE.into())?;
    Ok(())
}

/// Validate metadata.json file and write it to the package.
/// Also updates `Metadata` object by setting `build_date` and `name` properties.
fn validate_and_write_metadata(
    manifest: &Manifest, build_date: DateTime<Utc>, name: &str, package: &Package,
) -> anyhow::Result<()> {
    let resource = manifest.metadata.build();
    let metadata_reader = resource.get_reader()?;

    let mut metadata = Metadata::<ApplicationPackage>::from_reader(metadata_reader)
        .map_err(|err| FileError::from_string(resource.to_string(), Some(err)))?;
    metadata.set_build_date(build_date);
    metadata.set_name(name);

    let resource = BytesResource::new(resource.name()?, metadata.to_bytes()?);
    package.copy_file(&resource, ApplicationPackage::METADATA_FILE.into())?;
    Ok(())
}

/// Validate WASM module package and write it to the package.
fn validate_and_write_module(manifest: &ManifestModule, _package: &Package) -> anyhow::Result<()> {
    let module_file_path = manifest.file.upload_to_fs();
    let module_package = WasmModulePackage::from_file(module_file_path)?;
    module_package.validate()?;

    Ok(())
}

/// Write www dir to the package.
fn write_www_dir(manifest: &Manifest, package: &Package) -> anyhow::Result<()> {
    if let Some(share_dir) = &manifest.share {
        package.copy_dir_recursively(share_dir.build(), &ApplicationPackage::WWW_DIR.into())?;
    }
    Ok(())
}

/// Write share dir to the package.
fn write_share_dir(manifest: &Manifest, package: &Package) -> anyhow::Result<()> {
    if let Some(share_dir) = &manifest.share {
        package.copy_dir_recursively(share_dir.build(), &ApplicationPackage::SHARE_DIR.into())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use temp_dir::TempDir;

    use super::*;
    use crate::packaging::resources::{FsResource, ResourceBuilder};

    fn prepare_default_package_files() -> Metadata<ApplicationPackage> {
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

        metadata
    }

    fn prepare_package_dir(
        app_name: String, dir: &TempDir, metadata: &Metadata<ApplicationPackage>,
    ) -> Manifest {
        let metadata_path = dir.path().join("metadata.json");
        let icon_path = dir.path().join("icon.png");

        std::fs::write(
            &metadata_path,
            metadata
                .to_bytes()
                .expect("cannot decode metadata to bytes")
                .as_slice(),
        )
        .expect("Cannot create metadata.json file");

        std::fs::write(&icon_path, b"icon_image_svg_content")
            .expect("Cannot create metadata.json file");

        Manifest {
            name: app_name,
            icon: ResourceBuilder::Fs(FsResource::new(icon_path)),
            metadata: ResourceBuilder::Fs(FsResource::new(metadata_path)),
            modules: vec![],
            www: None,
            share: None,
        }
    }

    #[test]
    fn from_dir_test() {
        let dir = TempDir::new().expect("cannot create temp dir");

        let mut metadata = prepare_default_package_files();

        let manifest = prepare_package_dir("app".to_string(), &dir, &metadata);

        let build_time = DateTime::default();
        let package =
            ApplicationPackage::build_from_manifest(&manifest, dir.path(), None, build_time)
                .expect("Cannot create module package");

        assert!(package.validate().is_ok());

        // check metadata JSON file
        metadata.set_name(&manifest.name);
        metadata.set_build_date(build_time);

        let package_metadata = package
            .get_metadata()
            .expect("Cannot get metadata from package");
        assert_eq!(metadata, package_metadata);
    }
}
