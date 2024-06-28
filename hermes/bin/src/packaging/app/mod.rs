//! Hermes application package.

pub(crate) mod manifest;

use std::path::Path;

use crate::packaging::package::Package;

/// Hermes application package.
pub(crate) struct ApplicationPackage(Package);

impl ApplicationPackage {
    /// Open an existing application package.
    pub(crate) fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let package = Package::open(path)?;
        Ok(Self(package))
    }
}

#[cfg(test)]
mod tests {}
