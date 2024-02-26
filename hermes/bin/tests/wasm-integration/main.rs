//! Integration tests for Hermes WASM components

// SEE: https://docs.rs/libtest-mimic/latest/libtest_mimic/index.html

use libtest_mimic::{Arguments, /* Failed, */ Trial};

use wasmtime::{
    component::{Component, Linker},
    Config, Engine, Store,
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
        // let current_dir = env::current_dir()?;
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
        for path in wasm_file_paths.into_iter() {
            // let name = path
            //     .strip_prefix(&current_dir)?
            //     .display()
            //     .to_string();

            // Execute the wasm tests to get their name
            // Load WASM module in the executor.
            let (_, _, _) = {
                let mut config = Config::new();
                config.wasm_component_model(true);
                let engine = Engine::new(&config)?;
                let component = Component::from_file(&engine, &path)?;

                let linker = Linker::new(&engine);
                let mut store = Store::new(&engine, ());
                let instance = linker.instantiate(&mut store, &component)?;

                (component, store, instance)
            };

            // Run the tests in a loop until no more tests.
            // let mut no_more_tests = false;
            // let mut test_case = 0;
            loop {
                // execute result = test(test_case,false)

                // if result is not None {
                //   let test = Trial::test(result.name, move || execute_text(test_case, &path)).with_kind(name);
                //   tests.push(test);
                //   test_case += 1;
                // } else {
                //   no_more_tests = true;
                // }

                if /* no_more_tests */ true {
                    break;
                }
            }

            // let mut no_more_tests = false;
            // let mut test_case = 0;
            loop {
                // execute result = bench(test_case,false)

                // if result is not None {
                //   let test = Trial::test(result.name, move || execute_bench(test_case, &path)).with_kind(name);
                //   tests.push(test);
                //   test_case += 1;
                // } else {
                //   no_more_tests = true;
                // }

                if /* no_more_tests */ true {
                    break;
                }
            }
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
    let current_dir = env::current_dir()?;

    visit_dir(&current_dir, &mut tests)?;

    Ok(tests)
}

// /// Test a wasm modules numbered integration test.
// fn execute_test(test_case: u32, path: &Path) -> Result<(), Failed> {
//     let content = fs::read(path).map_err(|e| format!("Cannot read file: {e}"))?;

//     // Load the module into the executor
//     // Execute the test_case
//     // Check the result

//     Ok(())
// }

// /// Test a wasm modules numbered integration test.
// fn execute_bench(test_case: u32, path: &Path) -> Result<(), Failed> {
//     let content = fs::read(path).map_err(|e| format!("Cannot read file: {e}"))?;

//     // Load the module into the executor
//     // Execute the test_case benchmark
//     // Check the result

//     Ok(())
// }
