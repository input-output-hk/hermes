//! cli app sign command

use std::path::PathBuf;

use clap::Args;
use console::Emoji;

use crate::packaging::{
    app::ApplicationPackage,
    sign::{certificate::Certificate, keys::PrivateKey},
};

/// Application package signing
#[derive(Args)]
pub(crate) struct SignCommand {
    /// Defines the location of the builded application package.
    package: PathBuf,

    /// Defines the location of the ED2559 private key associated with the signing key.
    private_key: PathBuf,

    /// Defines the location of the x.509 certificate associated with the signing key.
    cert: PathBuf,
}

impl SignCommand {
    /// Run cli command
    pub(crate) fn exec(self) -> anyhow::Result<()> {
        println!("{} Sign application package...", Emoji::new("📝", ""));

        let private_key = PrivateKey::from_file(self.private_key)?;
        let cert = Certificate::from_file(self.cert)?;
        let package = ApplicationPackage::from_file(self.package)?;

        package.validate(true)?;
        package.author_sign(&private_key, &cert)?;

        println!("{} Done", Emoji::new("✅", ""));
        Ok(())
    }
}
