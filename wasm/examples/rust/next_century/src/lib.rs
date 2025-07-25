// Allow everything since this is generated code.
#[allow(clippy::all, unused)]
mod hermes;
mod stub;

struct TestComponent;

fn log_shutdown() {
    const FILE: &str = "next_century/src/lib.rs";
    const MSG: &str = "Issuing shutdown...";

    hermes::hermes::logging::api::log(
        hermes::hermes::logging::api::Level::Info,
        Some(&FILE),
        None,
        None,
        None,
        None,
        &MSG,
        None,
    );
}

impl hermes::exports::hermes::init::event::Guest for TestComponent {
    fn init() -> bool {
        const JAN_1_2100_SECONDS: u64 = 4102434000;

        let now_seconds = hermes::wasi::clocks::wall_clock::now().seconds;

        // Waiting for the next century.
        if now_seconds < JAN_1_2100_SECONDS {
            log_shutdown();
            hermes::hermes::init::api::done(1);
        }

        true
    }
}

hermes::export!(TestComponent with_types_in hermes);
