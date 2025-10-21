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
    app::{Application, ApplicationName},
    event::queue::Exit,
    ipfs, pool, reactor,
    runtime_extensions::init::trait_app::{RteApp, RteInitApp},
    vfs::VfsBootstrapper,
    wasm::module::Module,
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
#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(clap::Args)]
pub struct Playground {
    /// Wasm components to load as apps in this example
    components: Vec<PathBuf>,

    /// Select the name for the app.
    #[arg(long, default_value = "playground-app")]
    app_name: String,

    /// Shutdown the playground after the timeout (milliseconds)
    #[arg(long)]
    timeout_ms: Option<u64>,
}

impl Playground {
    /// Run playground CLI command
    pub fn exec(self) -> anyhow::Result<Exit> {
        let exit_lock = reactor::init()?;

        println!("{} Running a playground...", Emoji::new("✨", ""));

        ensure!(
            !self.components.is_empty(),
            "At least one component is required to run"
        );

        let temp_dir = TempDir::new()?;

        let app =
            create_and_init_app_with_temp_dir_vfs(self.app_name, &self.components, &temp_dir)?;

        tracing::info!("{} Bootstrapping IPFS node", console::Emoji::new("🖧", ""),);
        init_ipfs(&temp_dir)?;

        pool::init()?;
        println!("{} Loading an application...", Emoji::new("🛠️", ""),);

        reactor::load_app(app)?;

        let exit = if let Some(timeout_ms) = self.timeout_ms {
            exit_lock.wait_timeout(Duration::from_millis(timeout_ms))
        } else {
            exit_lock.wait()
        };

        // Wait for scheduled tasks to be finished.
        pool::terminate();
        reactor::drop_all_apps()?;
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
        let app_name = ApplicationName::new(&name);
        let module = Module::from_bytes(&app_name, &wasm_buf)?;
        modules.push((name, module));
    }

    Ok(modules)
}

/// Create an application with provided VFS.
fn create_app(
    app_name: String,
    vfs_dir_path: &Path,
    named_modules: Vec<(String, Module)>,
) -> anyhow::Result<Application> {
    let vfs_name = [&app_name, "_vfs"].concat();
    let vfs = VfsBootstrapper::new(vfs_dir_path, vfs_name).bootstrap()?;
    let module_registry = named_modules
        .iter()
        .map(|(name, module)| (name.to_owned(), module.id().clone()))
        .collect();
    let modules = named_modules
        .into_iter()
        .map(|(_, module)| module)
        .collect();
    let app = Application::new(ApplicationName(app_name), vfs, modules, module_registry);

    Ok(app)
}

/// Create a temp subdirectory.
fn create_temp_dir_child(
    temp_dir: &TempDir,
    child_path: &Path,
) -> anyhow::Result<PathBuf> {
    let child_absolute_path = temp_dir.path().join(child_path);
    fs::create_dir_all(child_absolute_path.as_path())?;
    Ok(child_absolute_path)
}

/// Collects `.wasm` files in the current directory or sub-directories of the current
/// directory. Then creates an applications out of all of them.
fn create_and_init_app_with_temp_dir_vfs(
    app_name: String,
    components: &[PathBuf],
    temp_dir: &TempDir,
) -> anyhow::Result<Application> {
    let named_modules = collect_modules(components)?;

    let vfs_dir_path = create_temp_dir_child(temp_dir, Path::new("vfs"))?;
    let app = create_app(app_name, &vfs_dir_path, named_modules)?;

    RteApp::new().init(app.name())?;

    Ok(app)
}
