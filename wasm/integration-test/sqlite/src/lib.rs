//! Hermes SQLite module integration test with WASM runtime.

wit_bindgen::generate!({
    world: "hermes:app/hermes",
    path: "../../wasi/wit",
    inline: "
        package hermes:app;

        world hermes {
            import hermes:sqlite/api;

            export hermes:integration-test/event;
        }
    ",
    generate_all,
});

export!(TestComponent);

mod test;

use hermes::integration_test::api::TestResult;
use hermes::sqlite;
use test::*;

struct TestComponent;

impl exports::hermes::integration_test::event::Guest for TestComponent {
    fn test(test: u32, run: bool) -> Option<TestResult> {
        TESTS.get(test as usize).map(|item| TestResult {
            name: String::from(item.name),
            status: {
                if run {
                    (item.executor)().is_ok()
                } else {
                    true
                }
            },
        })
    }

    fn bench(test: u32, run: bool) -> Option<TestResult> {
        BENCHES.get(test as usize).map(|item| TestResult {
            name: String::from(item.name),
            status: {
                if run {
                    (item.executor)().is_ok()
                } else {
                    true
                }
            },
        })
    }
}
