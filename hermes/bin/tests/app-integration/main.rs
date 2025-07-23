//! Application runner for Hermes WASM components

// SEE: https://docs.rs/libtest-mimic/latest/libtest_mimic/index.html

/// A parameter identifier specifying the directory for placing test WebAssembly
/// components.
const ENV_MODULE_DIR: &str = "TEST_WASM_MODULE_DIR";
/// A parameter identifier specifying the max number of milliseconds to run the test for.
const ENV_TIMEOUT_MS: &str = "TIMEOUT_MS";
/// A standard value assigned to [`ENV_MODULE_DIR`] when it's not specified.
const DEFAULT_ENV_MODULE_DIR: &str = "../../wasm/test-components";
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

use hermes::{
    app::Application, event::queue::Exit, reactor, vfs::VfsBootstrapper, wasm::module::Module,
};
use libtest_mimic::Arguments;
use temp_dir::TempDir;
use tracing::{level_filters::LevelFilter, subscriber::SetGlobalDefaultError};
use tracing_subscriber::{fmt::time, FmtSubscriber};

/// Init the logger
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
    main_internal().map_or(internal_failure, |exit| {
        exit.unwrap_exit_code_or(internal_failure)
    })
}

fn main_internal() -> anyhow::Result<Exit> {
    init_logger()?;

    let timeout_ms = get_timeout_ms()?;
    let args = Arguments::from_args();
    let temp_dir = TempDir::new()?;

    let apps = collect_apps(&args, &temp_dir)?;

    init_ipfs(&temp_dir)?;
    let exit_lock = reactor::init()?;

    for app in apps {
        reactor::load_app(app)?;
    }

    let exit = exit_lock.wait_timeout(Duration::from_millis(timeout_ms));

    Ok(exit)
}

/// Collect all the wasm modules to run from a specified directory.
fn visit_dir(
    args: &Arguments, path: &Path, modules: &mut Vec<(String, Module)>,
) -> anyhow::Result<()> {
    let raw_entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_e) => return Ok(()), // If the directory of modules can not be found, just skip it.
    };

    let entries: Vec<_> = raw_entries
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|entry| entry.file_type().map(|file_type| (file_type, entry.path())))
        .collect::<Result<_, _>>()?;

    let wasm_file_paths: Vec<_> = entries
        .iter()
        .filter_map(|(file_type, path)| {
            (file_type.is_file() && path.extension() == Some(OsStr::new("wasm"))).then_some(path)
        })
        .collect();

    let dir_paths: Vec<_> = entries
        .iter()
        .filter_map(|(file_type, path)| file_type.is_dir().then_some(path))
        .collect();

    // Collect `.wasm` files
    for file_path in wasm_file_paths {
        let name = file_path.strip_prefix(path)?.display().to_string();

        for skipped_name in &args.skip {
            if name.contains(skipped_name.as_str()) {
                continue;
            }
        }

        if let Some(filter) = args.filter.as_ref() {
            if !args.exact && !name.contains(filter.as_str()) {
                continue;
            }
            if args.exact && name != filter.as_str() {
                continue;
            }
        }

        let wasm_buf = fs::read(file_path)?;
        let module = Module::from_bytes(&wasm_buf)?;
        modules.push((name, module));
    }

    // process items inside directories
    for path in dir_paths {
        visit_dir(&args, path, modules)?;
    }

    Ok(())
}

/// Collects `.wasm` files in the current directory or sub-directories of the current
/// directory. Return a [`String`] module name along with each compiled [`Module`].
fn collect_modules(args: &Arguments) -> anyhow::Result<Vec<(String, Module)>> {
    // All wasm components in a directory.
    let mut modules = Vec::new();
    let test_module_dir =
        env::var(ENV_MODULE_DIR).unwrap_or_else(|_| DEFAULT_ENV_MODULE_DIR.to_owned());
    let path = Path::new(&test_module_dir);

    visit_dir(args, path, &mut modules)?;

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
