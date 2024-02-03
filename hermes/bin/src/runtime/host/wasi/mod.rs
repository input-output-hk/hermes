//! Runtime modules - extensions - WASI standard extensions

pub(crate) mod io;
pub(crate) mod random;
//pub(crate) mod filesystem;
pub(crate) mod cli;
pub(crate) mod clocks;
//pub(crate) mod http;

/*
wasi::cli::environment::add_to_linker(linker, get)?;
wasi::io::error::add_to_linker(linker, get)?;
wasi::io::streams::add_to_linker(linker, get)?;
wasi::cli::stdin::add_to_linker(linker, get)?;
wasi::cli::stdout::add_to_linker(linker, get)?;
wasi::cli::stderr::add_to_linker(linker, get)?;
wasi::clocks::monotonic_clock::add_to_linker(linker, get)?;
wasi::clocks::wall_clock::add_to_linker(linker, get)?;
wasi::filesystem::types::add_to_linker(linker, get)?;
wasi::filesystem::preopens::add_to_linker(linker, get)?;
wasi::random::random::add_to_linker(linker, get)?;
wasi::random::insecure::add_to_linker(linker, get)?;
wasi::random::insecure_seed::add_to_linker(linker, get)?;
wasi::http::types::add_to_linker(linker, get)?;
wasi::http::outgoing_handler::add_to_linker(linker, get)?;
*/

use crate::runtime::extensions::NewState;

/// WASI State
pub(crate) struct State {
    /// WASI CLI State
    cli: cli::State,
    /// WASI Clock State
    clocks: clocks::State,
    /// WASI IO State
    io: io::State,
    /// WASI Random State
    random: random::State,
}

impl NewState for State {
    fn new(ctx: &crate::wasm::context::Context) -> Self {
        Self {
            cli: cli::State::new(ctx),
            clocks: clocks::State::new(ctx),
            io: io::State::new(ctx),
            random: random::State::new(ctx),
        }
    }
}
