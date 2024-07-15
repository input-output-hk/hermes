//! Run cli command

use std::path::PathBuf;

use crate::{
    packaging::{
        app::ApplicationPackage,
        sign::certificate::{self, Certificate},
    },
    reactor::HermesReactor,
};

/// Run cli command
pub(crate) struct Run;

impl Run {
    /// Run the hermes application
    pub(crate) fn exec(
        app_package: PathBuf, certificates: Vec<PathBuf>, unstrusted: bool,
    ) -> anyhow::Result<()> {
        for cert_path in certificates {
            let cert = Certificate::from_file(cert_path)?;
            certificate::storage::add_certificate(cert)?;
        }

        let package = ApplicationPackage::from_file(app_package)?;
        package.validate(unstrusted)?;

        let mut reactor = HermesReactor::new(vec![])?;
        reactor.wait()?;

        Ok(())
    }
}
