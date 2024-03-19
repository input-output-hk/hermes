//! Crypto state

use std::{
    cmp::Eq,
    hash::{Hash, Hasher},
};

use dashmap::DashMap;
use ed25519_bip32::XPrv;
use once_cell::sync::Lazy;

use crate::app::HermesAppName;

/// Map of app name to resource holder
type State = DashMap<HermesAppName, ResourceHolder>;

/// Wrapper for `XPrv` to implement Hash used in `DashMap`
#[derive(Eq, Clone, PartialEq)]
struct WrappedXPrv(XPrv);

/// Implement Hash for `WrappedXPrv`
impl Hash for WrappedXPrv {
    /// Hasher for `WrappedXPrv`
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_ref().hash(state);
    }
}

/// Implement From for `XPrv` to `WrappedXPrv`
impl From<XPrv> for WrappedXPrv {
    /// Turn `XPrv` into `WrappedXPrv`
    fn from(xprv: XPrv) -> Self {
        WrappedXPrv(xprv)
    }
}

/// Resource holder to hold the resources for `XPrv`.
#[derive(Clone)]
pub(crate) struct ResourceHolder {
    /// Map of resource to id.
    id_to_resource_map: DashMap<u32, XPrv>,
    /// Map of id to resource.
    resource_to_id_map: DashMap<WrappedXPrv, u32>,
    /// Current Id.
    current_id: u32,
}

impl ResourceHolder {
    /// Generate new resource holder.
    fn new() -> Self {
        Self {
            id_to_resource_map: DashMap::new(),
            resource_to_id_map: DashMap::new(),
            current_id: 0,
        }
    }

    /// Get the next id and increment the current id.
    fn get_and_increment_next_id(&mut self) -> u32 {
        self.current_id += 1;
        self.current_id
    }

    /// Get the resource from id if possible.
    fn get_resource_from_id(&self, id: u32) -> Option<XPrv> {
        self.id_to_resource_map
            .get(&id)
            .map(|entry| entry.value().clone())
    }

    /// Get the id from resource if possible.
    fn get_id_from_resource(&self, resource: &XPrv) -> Option<u32> {
        self.resource_to_id_map
            .get(&WrappedXPrv::from(resource.clone()))
            .map(|entry| *entry.value())
    }

    /// Drop the item from resources using id if possible.
    /// Return the id of the resource if successful remove from maps
    fn drop(&mut self, id: u32) -> Option<u32> {
        // Check if the resource exists in id_to_resource_map.
        if let Some(resource) = self.get_resource_from_id(id) {
            // Check if the id exists in resource_to_id_map.
            if let Some(associated_id) = self.get_id_from_resource(&resource) {
                // The id should be the same.
                if associated_id == id {
                    // Remove the resource from both maps.
                    if let Some(r) = self.id_to_resource_map.remove(&id) {
                        self.resource_to_id_map.remove(&WrappedXPrv(r.1));
                        return Some(associated_id);
                    }
                }
            }
        }
        None
    }
}

/// Global state to hold the resources.
static CRYPTO_INTERNAL_STATE: Lazy<State> = Lazy::new(DashMap::new);

/// Get the state.
pub(super) fn get_state() -> &'static State {
    &CRYPTO_INTERNAL_STATE
}

/// Set the state according to the app context.
pub(crate) fn set_state(app_name: HermesAppName) {
    CRYPTO_INTERNAL_STATE.insert(app_name, ResourceHolder::new());
}

/// Get the resource from the state using id if possible.
pub(crate) fn get_resource(app_name: &HermesAppName, id: u32) -> Option<XPrv> {
    if let Some(res_holder) = CRYPTO_INTERNAL_STATE.get(app_name) {
        return res_holder.get_resource_from_id(id);
    }
    None
}

/// Add the resource of `XPrv` to the state if possible.
/// Return the id if successful.
pub(crate) fn add_resource(app_name: &HermesAppName, xprv: XPrv) -> Option<u32> {
    if let Some(mut res_holder) = CRYPTO_INTERNAL_STATE.get_mut(app_name) {
        let wrapped_xprv = WrappedXPrv::from(xprv.clone());
        // Check whether the resource already exists.
        if !res_holder.resource_to_id_map.contains_key(&wrapped_xprv) {
            // if not get the next id and insert the resource to both maps.
            let id = res_holder.get_and_increment_next_id();
            res_holder.id_to_resource_map.insert(id, xprv);
            res_holder.resource_to_id_map.insert(wrapped_xprv, id);
            return Some(id);
        }
    }
    None
}

/// Delete the resource from the state using id if possible.
pub(crate) fn delete_resource(app_name: &HermesAppName, id: u32) -> Option<u32> {
    if let Some(mut res_holder) = CRYPTO_INTERNAL_STATE.get_mut(app_name) {
        return res_holder.drop(id);
    }
    None
}

#[cfg(test)]
mod tests_crypto_state {
    use std::thread;

    use super::*;
    const KEY1: [u8; 96] = [
        0xF8, 0xA2, 0x92, 0x31, 0xEE, 0x38, 0xD6, 0xC5, 0xBF, 0x71, 0x5D, 0x5B, 0xAC, 0x21, 0xC7,
        0x50, 0x57, 0x7A, 0xA3, 0x79, 0x8B, 0x22, 0xD7, 0x9D, 0x65, 0xBF, 0x97, 0xD6, 0xFA, 0xDE,
        0xA1, 0x5A, 0xDC, 0xD1, 0xEE, 0x1A, 0xBD, 0xF7, 0x8B, 0xD4, 0xBE, 0x64, 0x73, 0x1A, 0x12,
        0xDE, 0xB9, 0x4D, 0x36, 0x71, 0x78, 0x41, 0x12, 0xEB, 0x6F, 0x36, 0x4B, 0x87, 0x18, 0x51,
        0xFD, 0x1C, 0x9A, 0x24, 0x73, 0x84, 0xDB, 0x9A, 0xD6, 0x00, 0x3B, 0xBD, 0x08, 0xB3, 0xB1,
        0xDD, 0xC0, 0xD0, 0x7A, 0x59, 0x72, 0x93, 0xFF, 0x85, 0xE9, 0x61, 0xBF, 0x25, 0x2B, 0x33,
        0x12, 0x62, 0xED, 0xDF, 0xAD, 0x0D,
    ];

    const KEY2: [u8; 96] = [
        0x60, 0xD3, 0x99, 0xDA, 0x83, 0xEF, 0x80, 0xD8, 0xD4, 0xF8, 0xD2, 0x23, 0x23, 0x9E, 0xFD,
        0xC2, 0xB8, 0xFE, 0xF3, 0x87, 0xE1, 0xB5, 0x21, 0x91, 0x37, 0xFF, 0xB4, 0xE8, 0xFB, 0xDE,
        0xA1, 0x5A, 0xDC, 0x93, 0x66, 0xB7, 0xD0, 0x03, 0xAF, 0x37, 0xC1, 0x13, 0x96, 0xDE, 0x9A,
        0x83, 0x73, 0x4E, 0x30, 0xE0, 0x5E, 0x85, 0x1E, 0xFA, 0x32, 0x74, 0x5C, 0x9C, 0xD7, 0xB4,
        0x27, 0x12, 0xC8, 0x90, 0x60, 0x87, 0x63, 0x77, 0x0E, 0xDD, 0xF7, 0x72, 0x48, 0xAB, 0x65,
        0x29, 0x84, 0xB2, 0x1B, 0x84, 0x97, 0x60, 0xD1, 0xDA, 0x74, 0xA6, 0xF5, 0xBD, 0x63, 0x3C,
        0xE4, 0x1A, 0xDC, 0xEE, 0xF0, 0x7A,
    ];

    #[test]
    fn test_basic_func_resource() {
        let prv = XPrv::from_bytes_verified(KEY1).expect("Invalid private key");
        let app_name: HermesAppName = HermesAppName("App name".to_string());
        // Set the global state.
        set_state(app_name.clone());

        // Add the resource.
        let id1 = add_resource(&app_name, prv.clone());
        // Should return id 1.
        assert_eq!(id1, Some(1));
        // Get the resource from id 1.
        let resource = get_resource(&app_name, 1);
        // The resource should be the same
        assert_eq!(resource, Some(prv.clone()));

        // Add another resource, with the same key.
        let id2 = add_resource(&app_name, prv.clone());
        // Resource already exist, so it should return None.
        assert_eq!(id2, None);
        // Get the resource from id.
        let k2 = get_resource(&app_name, 2);
        // Resource already exist, so it should return None.
        assert_eq!(k2, None);

        // Dropping the resource with id 1.
        let drop_id_1 = delete_resource(&app_name, 1);
        assert_eq!(drop_id_1, Some(1));
        // Dropping the resource with id 2 which doesn't exist.
        let drop_id_2 = delete_resource(&app_name, 2);
        assert_eq!(drop_id_2, None);

        let res_holder = CRYPTO_INTERNAL_STATE
            .get(&app_name)
            .expect("App name not found");
        assert_eq!(res_holder.id_to_resource_map.len(), 0);
        assert_eq!(res_holder.resource_to_id_map.len(), 0);
    }

    #[test]
    fn test_thread_safe_insert_resources() {
        let app_name: HermesAppName = HermesAppName("App name 2".to_string());

        // Setup initial state.
        set_state(app_name.clone());

        // Run the test with multiple threads.
        let mut handles = vec![];

        // Spawning 20 threads.
        for _ in 0..20 {
            let handle = thread::spawn(|| {
                let app_name: HermesAppName = HermesAppName("App name 2".to_string());
                let prv1 = XPrv::from_bytes_verified(KEY1).expect("Invalid private key");
                // Adding resource
                add_resource(&app_name, prv1.clone());
                let app_name: HermesAppName = HermesAppName("App name 2".to_string());
                let prv2 = XPrv::from_bytes_verified(KEY2).expect("Invalid private key");
                // Adding resource.
                add_resource(&app_name, prv2.clone());
            });
            handles.push(handle);
        }

        // Wait for all threads to finish.
        for handle in handles {
            handle.join().expect("Thread panicked");
        }

        // Checking the results.
        let prv1 = XPrv::from_bytes_verified(KEY1).expect("Invalid private key");
        let prv2 = XPrv::from_bytes_verified(KEY2).expect("Invalid private key");

        let res_holder = CRYPTO_INTERNAL_STATE
            .get(&app_name)
            .expect("App name not found");

        // Maps should contains 2 resources.
        assert_eq!(res_holder.id_to_resource_map.len(), 2);
        assert_eq!(res_holder.resource_to_id_map.len(), 2);
        assert_eq!(res_holder.current_id, 2);
        // Maps should contains prv1 and prv2.
        assert!(res_holder
            .resource_to_id_map
            .contains_key(&WrappedXPrv::from(prv1.clone())));
        assert!(res_holder
            .resource_to_id_map
            .contains_key(&WrappedXPrv::from(prv2.clone())));
        // Map should contains id 1 and 2.
        assert!(res_holder.id_to_resource_map.contains_key(&1));
        assert!(res_holder.id_to_resource_map.contains_key(&2));
    }
}
