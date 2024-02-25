//! Crypto state

use std::sync::Mutex;

use dashmap::DashMap;
//  std::sync::LazyLock is still unstable
use once_cell::sync::Lazy;
use rusty_ulid::Ulid;

type Storage = DashMap<String, DashMap<Ulid, DashMap<Option<String>, CryptoState>>>;
type XPriv = [u8; 96];

#[allow(dead_code)]
#[derive(Default, Clone, Debug)]
struct CryptoState {
    current_exec_count: u64,
    resources: DashMap<u32, XPriv>,
}

/// Crypto internal state
struct InternalState {
    storage: Storage,
}

#[allow(dead_code)]
// CryptoState should be mutable so Mutex is used.
static CRYPTO_INTERNAL_STATE: Lazy<Mutex<InternalState>> = Lazy::new(|| {
    Mutex::new(InternalState {
        storage: Storage::default(),
    })
});

impl InternalState {
    #[allow(dead_code)]
    fn new_storage(&mut self, app_name: String, module_id: Ulid, event_name: Option<String>) {
        let event_map = DashMap::new();
        event_map.insert(
            event_name,
            CryptoState {
                current_exec_count: 0,
                resources: DashMap::new(),
            },
        );

        let module_map = DashMap::new();
        module_map.insert(module_id, event_map);

        let app_map = DashMap::new();
        app_map.insert(app_name, module_map);
        self.storage = app_map;
    }

    #[allow(dead_code)]
    /// Get the storage.
    fn _get_storage(&self) -> &Storage {
        &self.storage
    }

    #[allow(dead_code)]
    /// Check context
    fn check_context(
        &self, app_name: &String, module_id: &Ulid, event_name: Option<String>,
    ) -> bool {
        self.storage.contains_key(app_name)
            && self.storage.get(app_name).unwrap().contains_key(module_id)
            && self
                .storage
                .get(app_name)
                .unwrap()
                .get(module_id)
                .unwrap()
                .contains_key(&event_name)
    }
}

impl CryptoState {
    /// Insert new Resource where item is added with the id.
    /// Id will be incremented by 1 each time.
    fn _add_resource(&mut self, item: XPriv) -> u32 {
        // Get the highest key and increment it by 1
        // Can't rely on the length since item can be removed.
        let id = self
            .resources
            .iter()
            .map(|entry| entry.key().clone())
            .max()
            .unwrap_or(0)
            + 1;
        let _ = self.current_exec_count + 1;
        self.resources.insert(id, item);
        id
    }


    /// Get the item from resources using id if possible.
    fn _get(&self, id: u32) -> Option<XPriv> {
        self.resources.get(&id).map(|x| x.clone())
    }

    /// Drop the item from resources using id if possible.
    fn _drop(&mut self, id: u32) -> Result<(), ()> {
        self.resources.remove(&id).map(|_| ()).ok_or(())
    }
}


#[cfg(test)]
mod tests_crypto_state {
    use super::*;
    #[test]
    fn test_storage_context() {
        let ulid: Ulid = 1.into();
        CRYPTO_INTERNAL_STATE.lock().unwrap().new_storage("Test".to_string(), ulid, Some("Test Event".to_string()));
        debug_assert_eq!(CRYPTO_INTERNAL_STATE.lock().unwrap().check_context(&"Test".to_string(), &ulid, Some("Test Event".to_string())), true);
    }
}