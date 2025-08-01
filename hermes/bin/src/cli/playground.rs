//! Simplified runner for Hermes WASM components.
//! This runs each wasm file as a separate anonymous application.

use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{anyhow, ensure};
use console::Emoji;
use temp_dir::TempDir;

use crate::{
    app::Application, event::queue::Exit, ipfs, reactor, vfs::VfsBootstrapper, wasm::module::Module,
};

/// Hermes application playground
///
/// Loads each wasm component from the arguments as a standalone Hermes application
/// consisting of a single module with a temporary VFS.
///
/// This allows to run Wasm components without properly packaging them.
///
/// Returns an exit code that can be inspected for custom values issued by Wasm
/// components.
///
/// If an internal error occurred returns 101.
#[derive(Debug, clap::Args)]
pub struct Playground {
    /// Wasm components to load as apps in this example.
    components: Vec<PathBuf>,

    /// Shutdown the playground after the timeout (milliseconds)
    #[arg(long)]
    timeout_ms: Option<u64>,
}

impl Playground {
    /// Run playground CLI command
    pub fn exec(self) -> anyhow::Result<Exit> {
        println!("{} Running a playground...", Emoji::new("✨", ""));

        let temp_dir = TempDir::new()?;

        let apps = collect_apps(&self.components, &temp_dir)?;
        ensure!(!apps.is_empty(), "At least one app is required to run");

        tracing::info!("{} Bootstrapping IPFS node", console::Emoji::new("🖧", ""),);
        init_ipfs(&temp_dir)?;

        let exit_lock = reactor::init()?;

        println!(
            "{} Loading {} application(s)...",
            Emoji::new("🛠️", ""),
            apps.len(),
        );
        for app in apps {
            reactor::load_app(app)?;
        }

        let exit = if let Some(timeout_ms) = self.timeout_ms {
            exit_lock.wait_timeout(Duration::from_millis(timeout_ms))
        } else {
            exit_lock.wait()
        };

        Ok(exit)
    }
}

/// Initialize the IPFS node
fn init_ipfs(temp_dir: &TempDir) -> anyhow::Result<()> {
    let ipfs_dir = create_temp_dir_child(temp_dir, Path::new("ipfs"))?;
    // disable bootstrapping the IPFS node to default addresses for testing
    let default_bootstrap = false;
    ipfs::bootstrap(ipfs_dir.as_path(), default_bootstrap)
}

/// Collects `.wasm` files in the current directory or sub-directories of the current
/// directory. Return a [`String`] module name along with each compiled [`Module`].
fn collect_modules(components: &[PathBuf]) -> anyhow::Result<Vec<(String, Module)>> {
    // All wasm components in a directory.
    let mut modules = Vec::new();

    // Collect component files
    for file_path in components {
        let name = file_path
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or_else(|| anyhow!("Provided path is invalid: {}", file_path.display()))?
            .to_string();
        let wasm_buf = fs::read(file_path)?;
        let module = Module::from_bytes(&wasm_buf)?;
        modules.push((name, module));
    }

    Ok(modules)
}

/// Create one-module application with temp directory VFS.
fn create_one_module_app(
    name: String, vfs_dir_path: &Path, module: Module,
) -> anyhow::Result<Application> {
    let vfs_name = [name.as_str(), "_vfs"].concat();
    let vfs = VfsBootstrapper::new(vfs_dir_path, vfs_name).bootstrap()?;
    let app = Application::new(name, vfs, vec![module]);

    Ok(app)
}

/// Create a temp subdirectory.
fn create_temp_dir_child(temp_dir: &TempDir, child_path: &Path) -> anyhow::Result<PathBuf> {
    let child_absolute_path = temp_dir.path().join(child_path);
    fs::create_dir_all(child_absolute_path.as_path())?;
    Ok(child_absolute_path)
}

/// Collects `.wasm` files in the current directory or sub-directories of the current
/// directory. Then creates one-module applications out of each of them.
fn collect_apps(components: &[PathBuf], temp_dir: &TempDir) -> anyhow::Result<Vec<Application>> {
    let modules = collect_modules(components)?;
    let mut apps = Vec::with_capacity(modules.len());
    for (module_name, module) in modules {
        let vfs_dir_path =
            create_temp_dir_child(temp_dir, Path::new("vfs").join(&module_name).as_path())?;
        let app = create_one_module_app(module_name, vfs_dir_path.as_path(), module)?;
        apps.push(app);
    }
    Ok(apps)
}
