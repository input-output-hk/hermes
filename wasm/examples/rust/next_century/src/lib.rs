wit_bindgen::generate!({
    world: "hermes:app/app",
    path: "../../../wasi/wit",
    inline: "
        package hermes:app;

        world app {
            import wasi:clocks/wall-clock@0.2.6;
            import hermes:logging/api;
            import hermes:init/api;
            
            export hermes:init/event;
        }
    ",
    generate_all,
});

export!(TestComponent);

struct TestComponent;

fn log_shutdown() {
    const FILE: &str = "next_century/src/lib.rs";
    const MSG: &str = "Issuing shutdown..";

    hermes::logging::api::log(
        hermes::logging::api::Level::Info,
        Some(&FILE),
        None,
        None,
        None,
        None,
        &MSG,
        None,
    );
}

impl exports::hermes::init::event::Guest for TestComponent {
    fn init() -> bool {
        const JAN_1_2100_SECONDS: u64 = 4102434000;

        let now_seconds = wasi::clocks::wall_clock::now().seconds;

        // Waiting for the next century.
        if now_seconds < JAN_1_2100_SECONDS {
            log_shutdown();
            hermes::init::api::done(1);
        }

        true
    }
}
