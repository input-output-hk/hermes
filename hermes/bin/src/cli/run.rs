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
    vfs::VfsBootstrapper,
};

/// Run cli command
pub(crate) struct Run;

impl Run {
    /// Run the hermes application
    pub(crate) fn exec(
        app_package: PathBuf, certificates: Vec<PathBuf>, untrusted: bool,
    ) -> anyhow::Result<()> {
        for cert_path in certificates {
            let cert = Certificate::from_file(cert_path)?;
            certificate::storage::add_certificate(cert)?;
        }

        let package = ApplicationPackage::from_file(app_package)?;
        package.validate(untrusted)?;

        let app_name = package.get_metadata()?.get_name()?;

        println!("{} Bootstrapping virtual filesystem", Emoji::new("🗄️", ""));
        let hermes_home_dir = Cli::hermes_home()?;
        let vfs = VfsBootstrapper::new(hermes_home_dir, app_name.clone()).bootstrap()?;

        println!("{} Running application {app_name} ", Emoji::new("🚀", ""),);
        let mut modules = Vec::new();
        for (_, module_package) in package.get_modules()? {
            let module = module_package.get_component()?;
            modules.push(module);
        }
        let app = HermesApp::new(HermesAppName(app_name), vfs, modules);

        let mut reactor = HermesReactor::new(vec![app])?;
        reactor.wait()?;

        Ok(())
    }
}
