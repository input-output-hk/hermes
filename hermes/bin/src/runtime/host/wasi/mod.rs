//! Runtime modules - extensions - WASI standard extensions

pub(crate) mod cli;
pub(crate) mod clocks;
pub(crate) mod filesystem;
pub(crate) mod http;
pub(crate) mod io;
pub(crate) mod random;

use crate::runtime::extensions::Stateful;

#[allow(dead_code)]
/// WASI State
pub(crate) struct State {
    /// WASI CLI State
    cli: cli::State,
    /// WASI Clock State
    clocks: clocks::State,
    /// WASI Filesystem State
    filesystem: filesystem::State,
    /// WASI HTTP State
    http: http::State,
    /// WASI IO State
    io: io::State,
    /// WASI Random State
    random: random::State,
}

impl Stateful for State {
    fn new(ctx: &crate::state::Context) -> Self {
        Self {
            cli: cli::State::new(ctx),
            clocks: clocks::State::new(ctx),
            filesystem: filesystem::State::new(ctx),
            http: http::State::new(ctx),
            io: io::State::new(ctx),
            random: random::State::new(ctx),
        }
    }
}
