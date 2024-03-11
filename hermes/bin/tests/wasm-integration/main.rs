//! Integration tests for Hermes WASM components

// SEE: https://docs.rs/libtest-mimic/latest/libtest_mimic/index.html

/// A parameter identifier specifying the directory for placing test WebAssembly
/// components.
const ENV_MODULE_DIR: &str = "TEST_WASM_MODULE_DIR";
/// A parameter identifier specifying the number of tests to run.
const ENV_N_TEST: &str = "N_TEST";
/// A parameter identifier specifying the number of benchmarks to run.
const ENV_N_BENCH: &str = "N_BENCH";
/// A standard value assigned to `ENV_MODULE_DIR` when it's not specified.
const DEFAULT_ENV_MODULE_DIR: &str = "../../wasm/test-components";
/// The default value for the number of tests to run when not specified.
const DEFAULT_ENV_N_TEST: &str = "32";
/// The default value for the number of benchmarks to run when not specified.
const DEFAULT_ENV_N_BENCH: &str = "32";

use std::{env, error::Error, ffi::OsStr, fs, path::Path};

use hermes::{
    runtime_extensions::hermes::integration_test::event::{execute_event, EventType},
    wasm::module::Module,
};
use libtest_mimic::{Arguments, Failed, Trial};

fn main() -> Result<(), Box<dyn Error>> {
    let args = Arguments::from_args();
    let tests = collect_tests()?;
    libtest_mimic::run(&args, tests).exit();
}

/// Creates as many tests as required for each `.wasm` file in the current directory or
/// sub-directories of the current directory.
fn collect_tests() -> Result<Vec<Trial>, Box<dyn Error>> {
    #[allow(clippy::missing_docs_in_private_items)]
    fn visit_dir(path: &Path, tests: &mut Vec<Trial>) -> Result<(), Box<dyn Error>> {
        let n_test: u32 = env::var(ENV_N_TEST)
            .unwrap_or_else(|_| DEFAULT_ENV_N_TEST.to_owned())
            .parse()?;
        let n_bench: u32 = env::var(ENV_N_BENCH)
            .unwrap_or_else(|_| DEFAULT_ENV_N_BENCH.to_owned())
            .parse()?;

        let entries: Vec<_> = fs::read_dir(path)?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|entry| entry.file_type().map(|file_type| (file_type, entry.path())))
            .collect::<Result<_, _>>()?;
        let wasm_file_paths: Vec<_> = entries
            .iter()
            .filter_map(|(file_type, path)| {
                (file_type.is_file() && path.extension() == Some(OsStr::new("wasm")))
                    .then_some(path)
            })
            .collect();
        let dir_paths: Vec<_> = entries
            .iter()
            .filter_map(|(file_type, path)| file_type.is_dir().then_some(path))
            .collect();

        // process `.wasm` files
        for file_path in wasm_file_paths {
            let name = file_path.strip_prefix(path)?.display().to_string();

            // Execute the wasm tests to get their name
            // Load WASM module in the executor.
            let wasm_buf = fs::read(file_path)?;
            let mut module = Module::new(&wasm_buf)?;

            let mut collect = |event_type: EventType, n: u32| -> Result<(), Box<dyn Error>> {
                for i in 0..n {
                    match execute_event(&mut module, i, false, event_type)? {
                        Some(result) => {
                            let path_string = file_path.to_string_lossy().to_string();
                            let test = Trial::test(result.name, move || {
                                execute(i, path_string, event_type)
                            })
                            .with_kind(name.clone());
                            tests.push(test);
                        },
                        _ => {
                            break;
                        },
                    }
                }

                Ok(())
            };

            // Run the tests in a loop until no more tests.
            collect(EventType::Test, n_test)?;
            // Run the benches in a loop until no more benches.
            collect(EventType::Bench, n_bench)?;
        }

        // process items inside directories
        for path in dir_paths {
            visit_dir(path, tests)?;
        }

        Ok(())
    }

    // Read wasm components to be test in a directory.
    let mut tests = Vec::new();
    let test_module_dir =
        env::var(ENV_MODULE_DIR).unwrap_or_else(|_| DEFAULT_ENV_MODULE_DIR.to_owned());
    let path = Path::new(&test_module_dir);

    visit_dir(path, &mut tests)?;

    Ok(tests)
}

/// Executes a test for a wasm component.
fn execute(test_case: u32, path: String, event_type: EventType) -> Result<(), Failed> {
    let wasm_buf = fs::read(path).map_err(|e| format!("Cannot read file: {e}"))?;

    let mut module = Module::new(&wasm_buf)?;

    match execute_event(&mut module, test_case, true, event_type)? {
        Some(result) => {
            assert!(result.status);
            Ok(())
        },
        _ => Err(Failed::from("result unexpected")),
    }
}
