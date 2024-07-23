//! Run cli command

use std::path::PathBuf;

use clap::Args;
use console::Emoji;

use crate::{
    app::{HermesApp, HermesAppName},
    cli::Cli,
    packaging::{
        app::ApplicationPackage,
        sign::certificate::{self, Certificate},
    },
    reactor::HermesReactor,
    vfs::{Hdf5Mount, Hdf5MountToLib, Vfs, VfsBootstrapper},
};

/// Run cli command
#[derive(Args)]
pub(crate) struct Run {
    /// Path to the Hermes application package to run
    app_package: PathBuf,

    /// Path to the trusted certificate
    #[clap(name = "cert", short)]
    certificates: Vec<PathBuf>,

    /// Flag which disables package signature verification
    #[clap(long, action = clap::ArgAction::SetTrue)]
    untrusted: bool,
}

impl Run {
    /// Run the hermes application
    pub(crate) fn exec(self) -> anyhow::Result<()> {
        for cert_path in self.certificates {
            let cert = Certificate::from_file(cert_path)?;
            certificate::storage::add_certificate(cert)?;
        }

        let package = ApplicationPackage::from_file(self.app_package)?;
        package.validate(self.untrusted)?;

        let app_name = package.get_metadata()?.get_name()?;

        println!("{} Bootstrapping virtual filesystem", Emoji::new("🗄️", ""));
        let vfs = bootstrap_vfs(app_name.clone(), &package)?;

        println!("{} Running application {app_name}\n", Emoji::new("🚀", ""),);
        let mut modules = Vec::new();
        for module_info in package.get_modules()? {
            let module = module_info.get_component()?;
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
    let mut mount = Hdf5Mount::default();

    mount.with_root_files(vec![
        package.get_icon_file()?,
        package.get_metadata_file()?.file(),
    ]);

    let modules = package.get_modules()?;
    let mut to_lib_content = Vec::with_capacity(modules.len());
    for module_info in modules {
        let to_lib = Hdf5MountToLib {
            dir_name: module_info.get_name(),
            files: Vec::new(),
            share: module_info.get_share(),
        };
        to_lib_content.push(to_lib);
    }
    mount.with_to_lib(to_lib_content);

    if let Some(share_dir) = package.get_share_dir() {
        mount.with_share_dir(share_dir);
    }
    if let Some(www_dir) = package.get_www_dir() {
        mount.with_www_dir(www_dir);
    }

    let hermes_home_dir = Cli::hermes_home()?;
    let mut bootstrapper = VfsBootstrapper::new(hermes_home_dir, app_name);
    bootstrapper.set_hdf5_mount(mount);
    let vfs = bootstrapper.bootstrap()?;
    Ok(vfs)
}
