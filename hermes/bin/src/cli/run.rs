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
    vfs::{Vfs, VfsBootstrapper},
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
        let vfs = bootstrap_vfs(app_name.clone(), &package)?;

        println!("{} Running application {app_name}\n", Emoji::new("🚀", ""),);
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

/// Bootstrap Hermes virtual filesystem
fn bootstrap_vfs(app_name: String, package: &ApplicationPackage) -> anyhow::Result<Vfs> {
    let hermes_home_dir = Cli::hermes_home()?;
    let mut bootstrapper = VfsBootstrapper::new(hermes_home_dir, app_name);

    if let Some(share_dir) = package.get_share_dir() {
        bootstrapper.with_mounted_share(share_dir);
    }
    if let Some(www_dir) = package.get_www_dir() {
        bootstrapper.with_mounted_www(www_dir);
    }

    let vfs = bootstrapper.bootstrap()?;
    Ok(vfs)
}
