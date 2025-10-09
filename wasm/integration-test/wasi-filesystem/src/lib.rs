wit_bindgen::generate!({
    world: "hermes:app/hermes",
    path: "../../wasi/wit",
    inline: "
        package hermes:app;

        world hermes {
            include wasi:filesystem/imports@0.2.6;

            export hermes:integration-test/event;
        }
    ",
    generate_all,
});

export!(TestComponent);

use hermes::integration_test::api::TestResult;

mod tests;

struct TestComponent;

impl exports::hermes::integration_test::event::Guest for TestComponent {
    fn test(test: u32, run: bool) -> Option<TestResult> {
        let test_fns = tests::test_fns();

        if let Some((test_name, test_fn)) = test_fns.get(test as usize) {
            let status = if run {
                test_fn()
                    .map_err(|e| {
                        eprintln!("{e:?}");
                        e
                    })
                    .is_ok()
            } else {
                true
            };

            Some(TestResult {
                name: test_name.to_string(),
                status,
            })
        } else {
            None
        }
    }

    fn bench(_test: u32, _run: bool) -> Option<TestResult> {
        None
    }
}
