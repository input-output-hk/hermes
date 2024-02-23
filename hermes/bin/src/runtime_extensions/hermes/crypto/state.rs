//! Crypto state

use dashmap::DashMap;
use once_cell::sync::Lazy;
use rusty_ulid::Ulid;

pub(crate) type Storage = DashMap<String, DashMap<Ulid, DashMap<Option<String>, CryptoState>>>;
type XPriv = [u8; 96];

/// Map of XPriv to index and index to XPriv.
struct ExtendedKeyMap {
    priv_to_index: DashMap<XPriv, u32>,
    index_to_priv: DashMap<u32, XPriv>,
}

struct CryptoState {
    current_exec_count: u64,
    resource_holder: ResourceHolder<XPriv>,
}

/// Crypto internal state
struct InternalState {
    storage: Storage,
    extended_key_map: ExtendedKeyMap,
}

static CRYPTO_INTERNAL_STATE: Lazy<InternalState> = Lazy::new(|| InternalState {
    storage: Storage::default(),
    extended_key_map: ExtendedKeyMap {
        priv_to_index: DashMap::new(),
        index_to_priv: DashMap::new(),
    },
});

#[derive(Default, Clone)]
/// A resource manager.
struct ResourceHolder<XPriv> {
    /// Map of resources.
    resources: DashMap<u32, XPriv>,
}

impl ResourceHolder<XPriv> {
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
