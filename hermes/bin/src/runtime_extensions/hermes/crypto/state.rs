//! Crypto state

use std::{
    cmp::Eq,
    hash::{Hash, Hasher},
    sync::Arc,
};

use dashmap::DashMap;
use ed25519_bip32::XPrv;
//  std::sync::LazyLock is still unstable
use once_cell::sync::Lazy;
use rusty_ulid::Ulid;

/// Map of app name, module ULID, event name, and module execution counter to resource
/// holder
type State = DashMap<String, DashMap<Ulid, DashMap<String, DashMap<u64, ResourceHolder>>>>;

/// Wrapper for XPrv to implement Hash used in DashMap
#[derive(Eq, Clone, PartialEq)]
struct WrappedXPrv(XPrv);

/// Implemnt Hash for WrappedXPrv
impl Hash for WrappedXPrv {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_ref().hash(state);
    }
}

impl From<XPrv> for WrappedXPrv {
    /// Turn XPrv into WrappedXPrv
    fn from(xprv: XPrv) -> Self {
        WrappedXPrv(xprv)
    }
}

/// Resource holder to hold the resources for XPrv.
#[derive(Clone)]
pub(crate) struct ResourceHolder {
    /// Map of resource to id.
    id_to_resource_map: DashMap<u32, XPrv>,
    /// Map of id to resource.
    resource_to_id_map: DashMap<WrappedXPrv, u32>,
    /// Current Id.
    current_id: u32,
}

// TODO - Remove dead code, once everthing is done
#[allow(dead_code)]
impl ResourceHolder {
    fn new() -> Self {
        Self {
            id_to_resource_map: DashMap::new(),
            resource_to_id_map: DashMap::new(),
            current_id: 0,
        }
    }

    fn get_and_increment_next_id(&mut self) -> u32 {
        let next_id = self.current_id + 1;
        self.current_id = next_id;
        next_id
    }

    fn get_resource_from_id(&self, id: &u32) -> Option<XPrv> {
        self.id_to_resource_map
            .get(id)
            .map(|entry| entry.value().clone())
    }

    fn get_id_from_resource(&self, resource: &XPrv) -> Option<u32> {
        self.resource_to_id_map
            .get(&WrappedXPrv::from(resource.clone()))
            .map(|entry| entry.value().clone())
    }

    /// Drop the item from resources using id if possible.
    /// Return the id of the resource if successful remove from maps
    fn drop(&mut self, id: u32) -> Option<u32> {
        // Check if the resource exists in id_to_resource_map.
        if let Some(resource) = self.get_resource_from_id(&id) {
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

static CRYPTO_INTERNAL_STATE: Lazy<Arc<State>> = Lazy::new(|| Arc::new(DashMap::new()));

/// Check the state whether the context exists and return the resources if possible.
fn check_context_and_return_resources(
    app_name: &String, module_id: &Ulid, event_name: &String, counter: &u64,
) -> Option<ResourceHolder> {
    let binding = CRYPTO_INTERNAL_STATE.clone();
    if let Some(app_map) = binding.get(app_name) {
        if let Some(module_map) = app_map.get(module_id) {
            if let Some(event_map) = module_map.get(event_name) {
                if let Some(counter_map) = event_map.get(counter) {
                    return Some(counter_map.clone());
                }
            }
        }
    }
    return None;
}

#[allow(dead_code)]
/// Set the state according to the app context.
pub(crate) fn set_state(app_name: String, module_id: Ulid, event_name: String, counter: u64) {
    // Counter -> ResourceHolder
    let counter_map = DashMap::new();
    counter_map.insert(counter, ResourceHolder::new());
    // Event -> Counter
    let event_map = DashMap::new();
    event_map.insert(event_name, counter_map);
    // Module -> Event
    let module_map = DashMap::new();
    module_map.insert(module_id, event_map);
    // App -> Module
    CRYPTO_INTERNAL_STATE.insert(app_name, module_map);
}

#[allow(dead_code)]
/// Get the resource from the state using id if possible.
pub(crate) fn get_resource(
    app_name: &String, module_id: &Ulid, event_name: &String, counter: &u64, id: &u32,
) -> Option<XPrv> {
    let res_holder = check_context_and_return_resources(app_name, module_id, event_name, counter);
    if let Some(resource) = res_holder {
        return resource.get_resource_from_id(&id);
    }
    None
}

#[allow(dead_code)]
/// Add the resource of XPrv to the state if possible.
/// Return the id if successful.
pub(crate) fn add_resource(
    app_name: &String, module_id: &Ulid, event_name: &String, counter: &u64, xprv: XPrv,
) -> Option<u32> {
    let binding = CRYPTO_INTERNAL_STATE.clone();
    if let Some(app_map) = binding.get(app_name) {
        if let Some(module_map) = app_map.get(module_id) {
            if let Some(event_map) = module_map.get(event_name) {
                if let Some(mut counter_map) = event_map.get_mut(counter) {
                    let wrapped_xprv = WrappedXPrv::from(xprv.clone());
                    // Check whether the resource already exists.
                    if !counter_map.resource_to_id_map.contains_key(&wrapped_xprv) {
                        // if not get the next id and insert the resource to both maps.
                        let id = counter_map.get_and_increment_next_id();
                        counter_map.id_to_resource_map.insert(id, xprv);
                        counter_map.resource_to_id_map.insert(wrapped_xprv, id);
                        // FIXME - Remove log
                        println!("Resource added with id: {}", id);
                        return Some(id);
                    }
                }
            }
        }
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
        0x60, 0xd3, 0x99, 0xda, 0x83, 0xef, 0x80, 0xd8, 0xd4, 0xf8, 0xd2, 0x23, 0x23, 0x9e, 0xfd,
        0xc2, 0xb8, 0xfe, 0xf3, 0x87, 0xe1, 0xb5, 0x21, 0x91, 0x37, 0xff, 0xb4, 0xe8, 0xfb, 0xde,
        0xa1, 0x5a, 0xdc, 0x93, 0x66, 0xb7, 0xd0, 0x03, 0xaf, 0x37, 0xc1, 0x13, 0x96, 0xde, 0x9a,
        0x83, 0x73, 0x4e, 0x30, 0xe0, 0x5e, 0x85, 0x1e, 0xfa, 0x32, 0x74, 0x5c, 0x9c, 0xd7, 0xb4,
        0x27, 0x12, 0xc8, 0x90, 0x60, 0x87, 0x63, 0x77, 0x0e, 0xdd, 0xf7, 0x72, 0x48, 0xab, 0x65,
        0x29, 0x84, 0xb2, 0x1b, 0x84, 0x97, 0x60, 0xd1, 0xda, 0x74, 0xa6, 0xf5, 0xbd, 0x63, 0x3c,
        0xe4, 0x1a, 0xdc, 0xee, 0xf0, 0x7a,
    ];

    #[test]
    fn test_basic_func_resource() {
        let prv = XPrv::from_bytes_verified(KEY1).expect("Invalid private key");
        let app_name = "App name".to_string();
        let module_id: Ulid = 1.into();
        let event_name = "test_event".to_string();
        let counter = 10;

        // Set the global state.
        set_state(
            app_name.clone(),
            module_id.clone(),
            event_name.clone(),
            counter.clone(),
        );

        // Add the resource.
        let id1 = add_resource(&app_name, &module_id, &event_name, &counter, prv.clone());
        // Should return id 1.
        assert_eq!(id1, Some(1));
        // Get the resource from id 1.
        let resource = get_resource(&app_name, &module_id, &event_name, &counter, &1);
        // The resource should be the same
        assert_eq!(resource, Some(prv.clone()));

        // Add another resource, with the same key.
        let id2 = add_resource(&app_name, &module_id, &event_name, &counter, prv.clone());
        // Resource already exist, so it should return None.
        assert_eq!(id2, None);
        // Get the resource from id.
        let k2 = get_resource(&app_name, &module_id, &event_name, &counter, &2);
        // Resource already exist, so it should return None.
        assert_eq!(k2, None);

        let mut res_holder =
            check_context_and_return_resources(&app_name, &module_id, &event_name, &counter)
                .expect("Resource holder not found");
        // Dropping the resource with id 1.
        let drop_id_1 = res_holder.drop(1);
        assert_eq!(drop_id_1, Some(1));
        assert_eq!(res_holder.id_to_resource_map.len(), 0);
        assert_eq!(res_holder.resource_to_id_map.len(), 0);

        // Dropping the resource with id 2 which doesn't exist.
        let drop_id_1 = res_holder.drop(2);
        assert_eq!(drop_id_1, None);
        assert_eq!(res_holder.id_to_resource_map.len(), 0);
        assert_eq!(res_holder.resource_to_id_map.len(), 0);
    }

    #[test]
    fn test_thread_safe_insert_resources() {
        let app_name = "App name".to_string();
        let module_id: Ulid = 1.into();
        let event_name = "test_event".to_string();
        let counter = 10;

        // Setup initial state.
        set_state(
            app_name.clone(),
            module_id.clone(),
            event_name.clone(),
            counter,
        );

        // Run the test with multiple threads.
        let mut handles = vec![];

        // Spawning 20 threads.
        for _ in 0..20 {
            let handle = thread::spawn(|| {
                let app_name = "App name".to_string();
                let module_id: Ulid = 1.into();
                let event_name = "test_event".to_string();
                let counter = 10;
                let prv1 = XPrv::from_bytes_verified(KEY1).expect("Invalid private key");
                // Adding resource
                add_resource(&app_name, &module_id, &event_name, &counter, prv1.clone());
                let app_name = "App name".to_string();
                let module_id: Ulid = 1.into();
                let event_name = "test_event".to_string();
                let counter = 10;
                let prv2 = XPrv::from_bytes_verified(KEY2).expect("Invalid private key");
                // Adding resource.
                add_resource(&app_name, &module_id, &event_name, &counter, prv2.clone());
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

        let res_holder =
            check_context_and_return_resources(&app_name, &module_id, &event_name, &counter);
        res_holder.map(|res| {
            // Maps should contains 2 resources.
            assert_eq!(res.id_to_resource_map.len(), 2);
            assert_eq!(res.resource_to_id_map.len(), 2);
            assert_eq!(res.current_id, 2);
            // Maps should contains prv1 and prv2.
            assert_eq!(
                res.resource_to_id_map
                    .contains_key(&WrappedXPrv::from(prv1.clone())),
                true
            );
            assert_eq!(
                res.resource_to_id_map
                    .contains_key(&WrappedXPrv::from(prv2.clone())),
                true
            );
            // Map should contains id 1 and 2.
            assert_eq!(res.id_to_resource_map.contains_key(&1), true);
            assert_eq!(res.id_to_resource_map.contains_key(&2), true);
        });
    }
}
