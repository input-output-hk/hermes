//! Hermes application package.

pub(crate) mod manifest;

use std::path::Path;

use chrono::{DateTime, Utc};
use manifest::Manifest;

use crate::packaging::package::Package;

/// Hermes application package.
pub(crate) struct ApplicationPackage(#[allow(dead_code)] Package);

impl ApplicationPackage {
    /// Hermes application package file extension.
    const FILE_EXTENSION: &'static str = "happ";

    /// Create a new Hermes application package package from a manifest file.
    pub(crate) fn build_from_manifest<P: AsRef<Path>>(
        manifest: &Manifest, output_path: P, package_name: Option<&str>, _build_time: DateTime<Utc>,
    ) -> anyhow::Result<Self> {
        let package_name = package_name.unwrap_or(&manifest.name);
        let mut package_path = output_path.as_ref().join(package_name);
        package_path.set_extension(Self::FILE_EXTENSION);
        let package = Package::create(&package_path)?;

        Ok(Self(package))
    }

    /// Open an existing application package.
    #[allow(dead_code)]
    pub(crate) fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let package = Package::open(path)?;
        Ok(Self(package))
    }
}

#[cfg(test)]
mod tests {}
