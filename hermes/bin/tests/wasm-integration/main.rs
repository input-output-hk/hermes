//! Integration tests for Hermes WASM components

// SEE: https://docs.rs/libtest-mimic/latest/libtest_mimic/index.html

const ENV_MODULE_DIR: &str = "TEST_WASM_MODULE_DIR";

use libtest_mimic::{Arguments, Failed, Trial};

use hermes::{
    wasm::module::Module,
    runtime_extensions::hermes::integration_test::event::*
};

use std::{env, error::Error, ffi::OsStr, fs, path::Path};

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
            .filter_map(|(file_type, path)| (file_type.is_file() && path.extension() == Some(OsStr::new("wasm"))).then(|| path))
            .collect();
        let dir_paths: Vec<_> = entries
            .iter()
            .filter_map(|(file_type, path)| file_type.is_dir().then(|| path))
            .collect();

        // process `.wasm` files
        for file_path in wasm_file_paths.into_iter() {
            let name = file_path
                .strip_prefix(path)?
                .display()
                .to_string();

            // Execute the wasm tests to get their name
            // Load WASM module in the executor.
            let wasm_buf = fs::read(file_path)?;
            let mut module = Module::new(name.clone(), &wasm_buf)?;

            // Run the tests in a loop until no more tests.            
            for i in 0..32 {
                let on_test_event = OnTestEvent {
                    test: i,
                    run: false
                };

                module.execute_event(&on_test_event)?;

                let result;
                unsafe { result = TEST_RESULT_QUEUE.pop(); }

                dbg!(&result);

                if let Some(None) = result {
                    break;
                }
                if let Some(Some(result)) = result {
                    let path_string = path.to_string_lossy().to_string();
                    let test = Trial::test(result.name, move || execute_test(i, path_string)).with_kind(name.clone());
                    tests.push(test);
                }
            }

            /* for i in 0..32 {
                let on_test_event = hermes::runtime_extensions::hermes::integration_test::event::OnBenchEvent {
                    test: i,
                    run: false
                };

                module.execute_event(&on_test_event)?;

                /* unsafe {
                    let result = BENCH_RESULT_QUEUE.pop();
                    assert!(result.is_some());
                    dbg!(result);
                } */

                // if result is not None {
                //   let test = Trial::test(result.name, move || execute_bench(test_case, &path)).with_kind(name);
                //   tests.push(test);
                //   test_case += 1;
                // } else {
                //   no_more_tests = true;
                // }
            } */
        }

        // process items inside directories
        for path in dir_paths.into_iter() {
            visit_dir(path, tests)?;
        }

        Ok(())
    }

    // We recursively look for `.rs` files, starting from the current
    // directory.
    let mut tests = Vec::new();

    // Maybe we point this to an env_var or something so its easier in CI/CD.
    let test_module_dir = env::var(ENV_MODULE_DIR).unwrap_or_else(|_| panic!("{} is required", ENV_MODULE_DIR));

    let path = Path::new(&test_module_dir);

    visit_dir(&path, &mut tests)?;

    Ok(tests)
}

/// Test a wasm modules numbered integration test.
fn execute_test(test_case: u32, path: String) -> Result<(), Failed> {
    let content = fs::read(path).map_err(|e| format!("Cannot read file: {e}"))?;

    let _module = Module::new(test_case.to_string(), &content)?;

    dbg!(test_case, content);

    // Load the module into the executor
    // Execute the test_case
    // Check the result

    Ok(())
}

// /// Test a wasm modules numbered integration test.
// fn execute_bench(test_case: u32, path: &Path) -> Result<(), Failed> {
//     let content = fs::read(path).map_err(|e| format!("Cannot read file: {e}"))?;

//     // Load the module into the executor
//     // Execute the test_case benchmark
//     // Check the result

//     Ok(())
// }
