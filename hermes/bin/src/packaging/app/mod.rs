//! Hermes application package.

pub(crate) mod manifest;

use std::path::Path;

use chrono::{DateTime, Utc};
use manifest::Manifest;

use crate::{
    errors::Errors,
    packaging::{
        metadata::{Metadata, MetadataSchema},
        package::Package,
        resources::{bytes_resource::BytesResource, ResourceTrait},
        FileError,
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

        validate_and_write_metadata(manifest, build_time, package_name, &package)
            .unwrap_or_else(errors.get_add_err_fn());
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
}

/// Validate metadata.json file and write it to the package.
/// Also updates `Metadata` object by setting `build_date` and `name` properties.
fn validate_and_write_metadata(
    manifest: &Manifest, build_date: DateTime<Utc>, name: &str, package: &Package,
) -> anyhow::Result<()> {
    let resource = &manifest.metadata;
    let metadata_reader = resource.get_reader()?;

    let mut metadata = Metadata::<ApplicationPackage>::from_reader(metadata_reader)
        .map_err(|err| FileError::from_string(resource.location(), Some(err)))?;
    metadata.set_build_date(build_date);
    metadata.set_name(name);

    let resource = BytesResource::new(resource.name()?, metadata.to_bytes()?);
    package.copy_file(&resource, ApplicationPackage::METADATA_FILE.into())?;
    Ok(())
}

/// Write www dir to the package.
fn write_www_dir(manifest: &Manifest, package: &Package) -> anyhow::Result<()> {
    if let Some(share_dir) = &manifest.share {
        package.copy_dir_recursively(share_dir, &ApplicationPackage::WWW_DIR.into())?;
    }
    Ok(())
}

/// Write share dir to the package.
fn write_share_dir(manifest: &Manifest, package: &Package) -> anyhow::Result<()> {
    if let Some(share_dir) = &manifest.share {
        package.copy_dir_recursively(share_dir, &ApplicationPackage::SHARE_DIR.into())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {}
