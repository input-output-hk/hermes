//! Crypto runtime extension implementation.

use dashmap::DashMap;
use rusty_ulid::Ulid;

use crate::runtime_extensions::state::{Context, Stateful};

mod host;
type XPriv = [u8; 96];
#[derive(Default, Clone)]

/// Resource holder: a simple resource manager.
struct ResHolder<XPriv> {
    /// Map of resources.
    resources: DashMap<u32, XPriv>,
}

impl<XPriv> ResHolder<XPriv> {
    /// Insert new Resource where item is added with the id.
    /// Id will be incremented by 1 each time.
    fn add(&mut self, item: XPriv) -> u32 {
        // Get the highest key and increment it by 1
        // Can't rely on the length since item can be removed.
        let id = self
            .resources
            .iter()
            .map(|entry| entry.key())
            .max()
            .unwrap_or(&0)
            + 1;
        self.resources.insert(id, item);
        id
    }

    /// Get the item from resources using id if possible.
    fn get(&self, id: u32) -> Option<&XPriv> {
        self.resources.get(&id).map(|entry| entry.value().clone())
    }

    /// Drop the item from resources using id if possible.
    fn drop(&mut self, id: u32) -> Result<(), ()> {
        self.resources.remove(&id).map(|_| ()).ok_or(())
    }
}

struct ExtendedKeyMap {
    priv_to_index: DashMap<XPriv, u32>,
    index_to_priv: DashMap<u32, XPriv>,
}

struct CryptoState {
    current_exec_count: u64,
    resource_holder: ResHolder<XPriv>,
}

pub(crate) type Storage = DashMap<String, DashMap<Ulid, DashMap<Option<String>, CryptoState>>>;
/// State
pub(crate) struct State {
    storage: Storage,
    extended_key_map: ExtendedKeyMap,
}

impl Stateful for State {
    fn new(ctx: &Context) -> Self {
        State {
            storage: DashMap::default(),
            extended_key_map: ExtendedKeyMap {
                priv_to_index: DashMap::default(),
                index_to_priv: DashMap::default(),
            },
        }
    }
}
