//! Run cli command

use std::path::PathBuf;

use console::Emoji;

use crate::{
    app::{HermesApp, HermesAppName},
    cli::Cli,
    packaging::{
        app::ApplicationPackage,
        sign::certificate::{self, Certificate},
    },
    reactor::HermesReactor,
    vfs::Vfs,
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

        let app_name = package.get_metadata()?.get_name()?;

        println!("{} Bootstrapping virtual filesystem", Emoji::new("🗄️", ""));
        let vfs = Vfs::bootstrap(Cli::hermes_home(), app_name.as_str())?;

        println!("{} Running application {app_name} ", Emoji::new("🚀", ""),);
        let app = HermesApp::new(HermesAppName(app_name), vfs, vec![]);

        let mut reactor = HermesReactor::new(vec![app])?;
        reactor.wait()?;

        Ok(())
    }
}
