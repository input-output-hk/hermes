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

impl WrappedXPrv {
    /// Turn XPrv into WrappedXPrv
    fn from_xprv(xprv: XPrv) -> Self {
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
    /// Next Id to be used.
    next_id: u32,
}

// TODO - Remove dead code, once everthing is done
#[allow(dead_code)]
impl ResourceHolder {
    fn new() -> Self {
        Self {
            id_to_resource_map: DashMap::new(),
            resource_to_id_map: DashMap::new(),
            next_id: 0,
        }
    }

    fn get_next_id(&mut self) -> u32 {
        let num = self.next_id + 1;
        self.next_id = num;
        num
    }

    //FIXME - This should return a reference?
    fn get_resource_from_id(&self, id: &u32) -> Option<XPrv> {
        self.id_to_resource_map.get(id).map(|entry| entry.value().clone())
    }
    
    //FIXME - This should return a reference?
    fn get_id_from_resource(&self, resource: &XPrv) -> Option<u32> {
        self.resource_to_id_map
            .get(&WrappedXPrv::from_xprv(resource.clone()))
            .map(|entry| entry.value().clone())
    }

    /// Drop the item from resources using id if possible.
    /// Return the id of the resource if successful remove from maps
    fn drop(&mut self, id: u32) -> Option<u32> {
        if let Some(resource) = self.get_resource_from_id(&id) {
            if let Some(associated_id) = self.get_id_from_resource(&resource) {
                if associated_id == id {
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
                    // FIXME - This shouldn't be clone?
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
// FIXME - Should this return a reference?
pub(crate) fn get_resource(
    app_name: &String, module_id: &Ulid, event_name: &String, counter: &u64, id: &u32,
) -> Option<XPrv> {
    let res_holder = check_context_and_return_resources(app_name, module_id, event_name, counter);
    if let Some(resource) = res_holder {
        return resource.get_resource_from_id(&id);
    }
    return None;
}

#[allow(dead_code)]
/// Add the resource of XPrv to the state if possible.
/// Return the id if successful.
pub(crate) fn add_resource(
    app_name: &String, module_id: &Ulid, event_name: &String, counter: &u64, xprv: XPrv,
) -> Option<u32> {
    let binding = CRYPTO_INTERNAL_STATE.clone();
    if let Some(app_map) = binding.get(app_name) {
        if let Some(module_map) = app_map.value().get(module_id) {
            if let Some(event_map) = module_map.value().get(event_name) {
                if let Some(counter_map) = event_map.value().get(counter) {
                    // Check whether the resource already exists.
                    let wrapped_xprv = WrappedXPrv::from_xprv(xprv.clone());
                    if !counter_map.resource_to_id_map.contains_key(&wrapped_xprv) {
                        // FIXME - This shouldn't be clone?
                        let id = counter_map.clone().get_next_id();
                        counter_map.id_to_resource_map.insert(id, xprv);
                        counter_map.resource_to_id_map.insert(wrapped_xprv, id);
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

    #[test]
    fn test_set_state_and_add_resource() {
        let prv = XPrv::from_bytes_verified(KEY1).expect("Invalid private key");

        let app_name = "App name".to_string();
        let module_id: Ulid = 1.into();
        let event_name = "test_event".to_string();
        let counter = 10;
        // Set the global state
        set_state(
            app_name.clone(),
            module_id.clone(),
            event_name.clone(),
            counter.clone(),
        );

        // Add the resource
        let id = add_resource(&app_name, &module_id, &event_name, &counter, prv.clone());
        // Should return id 1
        assert_eq!(id, Some(1));
        // Get the resource from id
        let k = get_resource(&app_name, &module_id, &event_name, &counter, &1);
        // The resource should be the same
        assert_eq!(k, Some(prv.clone()));

        // Add another resource, with the same key
        let id2 = add_resource(
            &app_name,
            &module_id,
            &event_name,
            &counter,
            prv.clone().into(),
        );
        // Resource already exist, so it should return None
        assert_eq!(id2, None);
        // Get the resource from id
        let k2 = get_resource(&app_name, &module_id, &event_name, &counter, &2);
        // Resource already exist, so it should return None
        assert_eq!(k2, None);
    }
}
