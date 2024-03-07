//! Integration tests for Hermes WASM components

// SEE: https://docs.rs/libtest-mimic/latest/libtest_mimic/index.html

const ENV_MODULE_DIR: &str = "TEST_WASM_MODULE_DIR";

use std::{env, error::Error, ffi::OsStr, fs, path::Path};

use hermes::{runtime_extensions::hermes::integration_test::event::*, wasm::module::Module};
use libtest_mimic::{Arguments, Failed, Trial};

fn main() -> Result<(), Box<dyn Error>> {
    let args = Arguments::from_args();
    let tests = collect_tests()?;
    libtest_mimic::run(&args, tests).exit();
}

/// Creates as many tests as required for each `.wasm` file in the current directory or
/// sub-directories of the current directory.
fn collect_tests() -> Result<Vec<Trial>, Box<dyn Error>> {
    fn visit_dir(path: &Path, tests: &mut Vec<Trial>) -> Result<(), Box<dyn Error>> {
        let entries: Vec<_> = fs::read_dir(path)?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|entry| entry.file_type().map(|file_type| (file_type, entry.path())))
            .collect::<Result<_, _>>()?;
        let wasm_file_paths: Vec<_> = entries
            .iter()
            .filter_map(|(file_type, path)| {
                (file_type.is_file() && path.extension() == Some(OsStr::new("wasm"))).then(|| path)
            })
            .collect();
        let dir_paths: Vec<_> = entries
            .iter()
            .filter_map(|(file_type, path)| file_type.is_dir().then(|| path))
            .collect();

        // process `.wasm` files
        for file_path in wasm_file_paths.into_iter() {
            let name = file_path.strip_prefix(path)?.display().to_string();

            // Execute the wasm tests to get their name
            // Load WASM module in the executor.
            let wasm_buf = fs::read(file_path)?;
            let mut module = Module::new(name.clone(), &wasm_buf)?;

            // Run the tests in a loop until no more tests.
            for i in 0..32 {
                let on_test_event = OnTestEvent {
                    test: i,
                    run: false,
                };

                module.execute_event(&on_test_event)?;

                let result;
                unsafe {
                    result = TEST_RESULT_QUEUE.pop();
                }

                match result {
                    Some(Some(result)) => {
                        let path_string = file_path.to_string_lossy().to_string();
                        let test = Trial::test(result.name, move || execute_test(i, path_string))
                            .with_kind(name.clone());
                        tests.push(test);
                    },
                    _ => {
                        break;
                    },
                }
            }

            // Run the benches in a loop until no more benches.
            for i in 0..32 {
                let on_test_event = OnBenchEvent {
                    test: i,
                    run: false,
                };

                module.execute_event(&on_test_event)?;

                let result;
                unsafe {
                    result = BENCH_RESULT_QUEUE.pop();
                }

                match result {
                    Some(Some(result)) => {
                        let path_string = file_path.to_string_lossy().to_string();
                        let test = Trial::test(result.name, move || execute_bench(i, path_string))
                            .with_kind(name.clone());
                        tests.push(test);
                    },
                    _ => {
                        break;
                    },
                }
            }
        }

        // process items inside directories
        for path in dir_paths.into_iter() {
            visit_dir(path, tests)?;
        }

        Ok(())
    }

    // Read wasm components to be test in a directory.
    let mut tests = Vec::new();
    let test_module_dir =
        env::var(ENV_MODULE_DIR).unwrap_or_else(|_| panic!("{} is required", ENV_MODULE_DIR));
    let path = Path::new(&test_module_dir);

    visit_dir(&path, &mut tests)?;

    Ok(tests)
}

/// Executes a test from a wasm component.
fn execute(test_case: u32, path: String) -> Result<(), Failed> {
    let content = fs::read(path).map_err(|e| format!("Cannot read file: {e}"))?;

    let mut module = Module::new(test_case.to_string(), &content)?;

    // Load the module into the executor
    let on_test_event = OnTestEvent {
        test: test_case,
        run: true,
    };

    // Execute the test_case
    module.execute_event(&on_test_event)?;

    // Check the result
    let result;
    unsafe {
        result = TEST_RESULT_QUEUE.pop();
    }

    match result {
        Some(Some(result)) => {
            assert!(result.status);
            Ok(())
        },
        _ => Err(Failed::from("result unexpected")),
    }
}

/// Tests a wasm component numbered integration test in test.
fn execute_test(test_case: u32, path: String) -> Result<(), Failed> {
    execute(test_case, path)
}

/// Tests a wasm component numbered integration test in bench.
fn execute_bench(test_case: u32, path: String) -> Result<(), Failed> {
    execute(test_case, path)
}
