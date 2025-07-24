//! Application runner for Hermes WASM components.
//! This runs each wasm file as a separate anonymous application.

/// A parameter identifier specifying the max number of milliseconds to run the test for.
const ENV_TIMEOUT_MS: &str = "TIMEOUT_MS";
/// A standard value assigned to [`ENV_TIMEOUT_MS`] when it's not specified.
const DEFAULT_ENV_TIMEOUT_MS: u64 = 10 * 1000;

use std::{
    env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
    time::Duration,
};

use anyhow::{anyhow, ensure};
use clap::Parser;
use hermes::{
    app::Application, event::queue::Exit, reactor, vfs::VfsBootstrapper, wasm::module::Module,
};
use temp_dir::TempDir;
use tracing::{level_filters::LevelFilter, subscriber::SetGlobalDefaultError};
use tracing_subscriber::{fmt::time, FmtSubscriber};

/// An example Wasm Runner
///
/// Loads each wasm component from the arguments as a standalone Hermes application
/// consisting of a single module with a temporary VFS.
///
/// This allows to run Wasm components without properly packaging them.
///
/// Returns an exit code that can be inspected for custom values issued by Wasm components.
#[derive(Debug, clap::Parser)]
pub struct Arguments {
    /// Wasm components to load as apps in this example.
    components: Vec<PathBuf>,
}

/// Init the logger
#[allow(dead_code)]
fn init_logger() -> Result<(), SetGlobalDefaultError> {
    let subscriber = FmtSubscriber::builder()
        .json()
        .with_level(true)
        .with_thread_names(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_timer(time::UtcTime::rfc_3339())
        .with_max_level(LevelFilter::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
}

/// Initialize the IPFS node
fn init_ipfs(temp_dir: &TempDir) -> anyhow::Result<()> {
    let ipfs_dir = create_temp_dir_child(&temp_dir, Path::new("ipfs"))?;
    // disable bootstrapping the IPFS node to default addresses for testing
    let default_bootstrap = false;
    hermes::ipfs::bootstrap(ipfs_dir.as_path(), default_bootstrap)
}

/// Get the timeout value from env.
fn get_timeout_ms() -> anyhow::Result<u64> {
    env::var(ENV_TIMEOUT_MS)
        .map(|s| s.parse())
        .unwrap_or_else(|_| Ok(DEFAULT_ENV_TIMEOUT_MS))
        .map_err(anyhow::Error::from)
}

fn main() -> ExitCode {
    let internal_failure = ExitCode::from(101);
    main_internal()
        .inspect_err(|err| {
            // Use `tracing::error!` once logger initialization is fixed.
            eprintln!("Failed to run applications: {err}");
        })
        .map_or(internal_failure, |exit| {
            exit.unwrap_exit_code_or(internal_failure)
        })
}

fn main_internal() -> anyhow::Result<Exit> {
    // This is necessary otherwise the logging functions inside hermes are silent during the
    // test run.
    // init_logger()?;
    // This causes issues with normal test runs, so comment out for now.
    // info!("Starting Hermes WASM integration tests");

    let timeout_ms = get_timeout_ms()?;
    let args = Arguments::try_parse()?;
    let temp_dir = TempDir::new()?;

    let apps = collect_apps(&args, &temp_dir)?;
    ensure!(!apps.is_empty(), "At least one app is required to run");

    init_ipfs(&temp_dir)?;
    let exit_lock = reactor::init()?;

    for app in apps {
        reactor::load_app(app)?;
    }

    let exit = exit_lock.wait_timeout(Duration::from_millis(timeout_ms));

    Ok(exit)
}

/// Collects `.wasm` files in the current directory or sub-directories of the current
/// directory. Return a [`String`] module name along with each compiled [`Module`].
fn collect_modules(args: &Arguments) -> anyhow::Result<Vec<(String, Module)>> {
    // All wasm components in a directory.
    let mut modules = Vec::new();

    // Collect component files
    for file_path in &args.components {
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
fn collect_apps(args: &Arguments, temp_dir: &TempDir) -> anyhow::Result<Vec<Application>> {
    let modules = collect_modules(&args)?;
    let mut apps = Vec::with_capacity(modules.len());
    for (module_name, module) in modules {
        let vfs_dir_path =
            create_temp_dir_child(&temp_dir, Path::new("vfs").join(&module_name).as_path())?;
        let app = create_one_module_app(module_name, vfs_dir_path.as_path(), module)?;
        apps.push(app);
    }
    Ok(apps)
}
