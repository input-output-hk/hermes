//! Run cli command

use std::path::PathBuf;

use clap::Args;
use console::Emoji;

use crate::{
    app::{HermesApp, HermesAppName},
    cli::Cli,
    hdf5::Path,
    packaging::{
        app::ApplicationPackage,
        sign::certificate::{self, Certificate},
    },
    reactor::HermesReactor,
    vfs::{Vfs, VfsBootstrapper},
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
    let hermes_home_dir = Cli::hermes_home()?;
    let mut bootstrapper = VfsBootstrapper::new(hermes_home_dir, app_name);

    let root_path = Path::default();
    bootstrapper.with_mounted_file(root_path.clone(), package.get_icon_file()?);
    bootstrapper.with_mounted_file(root_path.clone(), package.get_metadata_file()?);
    if let Some(share_dir) = package.get_share_dir() {
        bootstrapper.with_mounted_dir(root_path.clone(), share_dir);
    }
    if let Some(www_dir) = package.get_www_dir() {
        bootstrapper.with_mounted_dir(root_path.clone(), www_dir);
    }

    for module_info in package.get_modules()? {
        let lib_module_dir_path: Path =
            format!("{}/{}", Vfs::LIB_DIR, module_info.get_name()).into();
        bootstrapper.with_dir_to_create(lib_module_dir_path.clone());

        bootstrapper.with_mounted_file(
            lib_module_dir_path.clone(),
            module_info.get_metadata_file()?,
        );
        bootstrapper.with_mounted_file(
            lib_module_dir_path.clone(),
            module_info.get_component_file()?,
        );
        if let Some(config_schema) = module_info.get_config_schema_file() {
            bootstrapper.with_mounted_file(lib_module_dir_path.clone(), config_schema);
        }
        if let Some(config) = module_info.get_config_file() {
            bootstrapper.with_mounted_file(lib_module_dir_path.clone(), config);
        }
        if let Some(settings_schema) = module_info.get_settings_schema_file() {
            bootstrapper.with_mounted_file(lib_module_dir_path.clone(), settings_schema);
        }
        if let Some(share_dir) = module_info.get_share() {
            bootstrapper.with_mounted_dir(lib_module_dir_path.clone(), share_dir);
        }
    }

    let vfs = bootstrapper.bootstrap()?;
    Ok(vfs)
}
