//! Run cli command

use std::path::PathBuf;

use clap::Args;
use console::Emoji;

use crate::{
    app::{HermesApp, HermesAppName},
    cli::Cli,
    ipfs::{HermesIpfsNode},
    packaging::{
        app::ApplicationPackage,
        sign::certificate::{self, Certificate},
    },
    reactor::HermesReactor,
    vfs::VfsBootstrapper,
};
use hermes_ipfs::IpfsBuilder;

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
        let hermes_home_dir = Cli::hermes_home()?;
        let ipfs_data_path = hermes_home_dir.as_path().join("ipfs");
        let mut bootstrapper = VfsBootstrapper::new(hermes_home_dir, app_name.clone());
        package.mount_to_vfs(&mut bootstrapper)?;
        let vfs = bootstrapper.bootstrap()?;

        println!("{} Bootstrapping IPFS node", Emoji::new("🖧", ""),);
        let hermes_ipfs_node = HermesIpfsNode::bootstrap(move || {
            IpfsBuilder::new()
            .with_default()
            .set_default_listener()
            .disable_tls()
            .set_disk_storage(ipfs_data_path.clone())
        })?;

        println!("{} Running application {app_name}\n", Emoji::new("🚀", ""),);
        let mut modules = Vec::new();
        for module_info in package.get_modules()? {
            let module = module_info.get_component()?;
            modules.push(module);
        }
        let app = HermesApp::new(HermesAppName(app_name), hermes_ipfs_node, vfs, modules);

        let mut reactor = HermesReactor::new(vec![app])?;
        reactor.wait()?;

        Ok(())
    }
}
