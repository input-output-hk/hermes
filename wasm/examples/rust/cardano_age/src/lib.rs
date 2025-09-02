wit_bindgen::generate!({
    world: "me:my-app/my-world",
    path: "../../../wasi/wit",
    inline: "
        package me:my-app;
        world my-world {
            import wasi:clocks/wall-clock@0.2.6;
            import hermes:logging/api;
            import hermes:init/api;
            
            export hermes:init/event;
        }
    ",
    generate_all,
});

struct TestComponent;

fn log_cardano_age(days: f64) {
    const FILE: &str = "cardano_age/src/lib.rs";

    let msg = format!("Cardano is live for {days} days!");

    hermes::logging::api::log(
        hermes::logging::api::Level::Info,
        Some(&FILE),
        None,
        None,
        None,
        None,
        &msg,
        None,
    );
}

impl exports::hermes::init::event::Guest for TestComponent {
    fn init() -> bool {
        const CARDANO_LAUNCH_SECONDS: u64 = 1506246291;
        const SECONDS_IN_A_DAY: u64 = 24 * 60 * 60;

        let elapsed_seconds = wasi::clocks::wall_clock::now()
            .seconds
            .saturating_sub(CARDANO_LAUNCH_SECONDS);

        let elapsed_days = elapsed_seconds as f64 / SECONDS_IN_A_DAY as f64;
        log_cardano_age(elapsed_days);

        hermes::init::api::done(0);

        true
    }
}

export!(TestComponent);
