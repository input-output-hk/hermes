// Allow everything since this is generated code.
#[allow(clippy::all, unused)]
mod hermes;
mod stub;

struct TestComponent;

fn log_cardano_age(days: f64) {
    const FILE: &str = "cardano_age/src/lib.rs";

    let msg = format!("Cardano is live for {days} days!");

    hermes::hermes::logging::api::log(
        hermes::hermes::logging::api::Level::Info,
        Some(&FILE),
        None,
        None,
        None,
        None,
        &msg,
        None,
    );
}

impl hermes::exports::hermes::init::event::Guest for TestComponent {
    fn init() -> bool {
        const CARDANO_LAUNCH_SECONDS: u64 = 1506246291;
        const SECONDS_IN_A_DAY: u64 = 24 * 60 * 60;

        let elapsed_seconds = hermes::wasi::clocks::wall_clock::now()
            .seconds
            .saturating_sub(CARDANO_LAUNCH_SECONDS);

        let elapsed_days = elapsed_seconds as f64 / SECONDS_IN_A_DAY as f64;
        log_cardano_age(elapsed_days);

        hermes::hermes::init::api::done(0);

        true
    }
}

hermes::export!(TestComponent with_types_in hermes);
