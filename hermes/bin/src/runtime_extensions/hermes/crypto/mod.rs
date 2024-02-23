//! Crypto runtime extension implementation.

use crate::runtime_extensions::state::{Context, Stateful};

mod host;
pub(crate) struct State {}

impl Stateful for State {
    fn new(ctx: &Context) -> Self {
        // pass app_name .... to function in state.rs (initialize state)
        State {
            // storage: DashMap::default(),
            // extended_key_map: ExtendedKeyMap {
            //     priv_to_index: DashMap::default(),
            //     index_to_priv: DashMap::default(),
            // },
        }
    }
}
