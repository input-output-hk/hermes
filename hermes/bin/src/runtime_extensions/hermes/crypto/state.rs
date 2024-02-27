//! Crypto state

use std::sync::Arc;

use dashmap::DashMap;
use ed25519_bip32::XPrv;
//  std::sync::LazyLock is still unstable
use once_cell::sync::Lazy;
use rusty_ulid::Ulid;

/// Map of app name, module ULID, event name, and module execution counter to resource holder
type State = DashMap<String, DashMap<Ulid, DashMap<String, DashMap<u64, ResourceHolder>>>>;

#[allow(dead_code)]
#[derive(Default, Clone, Debug)]
pub(crate) struct ResourceHolder {
    /// Map of resources.
    resources: DashMap<u32, XPrv>,
}

#[allow(dead_code)]
impl ResourceHolder {
    /// Insert new Resource where item is added with the id.
    /// Id will be incremented by 1 each time.

    fn get_id(&self) -> u32 {
        // Get the highest key and increment it by 1
        // Can't rely on the length since item can be removed.
        let id = self
            .resources
            .iter()
            .map(|entry| entry.key().clone())
            .max()
            .unwrap_or(0)
            + 1;
        id
    }

    /// Get the item from resources using id if possible.
    fn get_resource(&self, id: &u32) -> Option<XPrv> {
        self.resources.get(id).map(|entry| entry.value().clone())
    }

    /// Drop the item from resources using id if possible.
    fn drop(&mut self, id: u32) -> Result<(), ()> {
        self.resources.remove(&id).map(|_| ()).ok_or(())
    }
}

#[allow(dead_code)]
static CRYPTO_INTERNAL_STATE: Lazy<Arc<State>> = Lazy::new(|| Arc::new(DashMap::new()));

#[allow(dead_code)]
/// Set the state according to the app context.
pub(crate) fn set_state(app_name: String, module_id: Ulid, event_name: String, counter: u64) {
    // Counter -> ResourceHolder
    let counter_map = DashMap::new();
    counter_map.insert(
        counter,
        ResourceHolder {
            resources: DashMap::new(),
        },
    );
    // Event -> Counter
    let event_map = DashMap::new();
    event_map.insert(event_name, counter_map);
    // Module -> Event
    let module_map = DashMap::new();
    module_map.insert(module_id, event_map);
    // App -> Module
    CRYPTO_INTERNAL_STATE.insert(app_name, module_map);
}

/// Check the state whether the context exists and return the resources if possible.
pub(crate) fn check_context_and_return_resources(
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
/// Get the resource from the state using id if possible.
pub(crate) fn get_resource(
    app_name: &String, module_id: &Ulid, event_name:&String, counter: &u64, id: &u32,
) -> Option<XPrv> {
    let res_holder = check_context_and_return_resources(app_name, module_id, event_name, counter);
    if let Some(res_holder) = res_holder {
        return res_holder.get_resource(&id)
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
        if let Some(module_map) = app_map.get(module_id) {
            if let Some(event_map) = module_map.get(event_name) {
                if let Some(counter_map) = event_map.get(counter) {
                    // TODO - Check whether the resource already exists                    
                    let id = counter_map.get_id();
                    counter_map.resources.insert(id, xprv);
                    return Some(id)
                }
            }
        }
    }
    None
}
#[cfg(test)]
mod tests_crypto_state {
    use super::*;
    const KEY: [u8; 96] = [
        0xf8, 0xa2, 0x92, 0x31, 0xee, 0x38, 0xd6, 0xc5, 0xbf, 0x71, 0x5d, 0x5b, 0xac, 0x21,
        0xc7, 0x50, 0x57, 0x7a, 0xa3, 0x79, 0x8b, 0x22, 0xd7, 0x9d, 0x65, 0xbf, 0x97, 0xd6,
        0xfa, 0xde, 0xa1, 0x5a, 0xdc, 0xd1, 0xee, 0x1a, 0xbd, 0xf7, 0x8b, 0xd4, 0xbe, 0x64,
        0x73, 0x1a, 0x12, 0xde, 0xb9, 0x4d, 0x36, 0x71, 0x78, 0x41, 0x12, 0xeb, 0x6f, 0x36,
        0x4b, 0x87, 0x18, 0x51, 0xfd, 0x1c, 0x9a, 0x24, 0x73, 0x84, 0xdb, 0x9a, 0xd6, 0x00,
        0x3b, 0xbd, 0x08, 0xb3, 0xb1, 0xdd, 0xc0, 0xd0, 0x7a, 0x59, 0x72, 0x93, 0xff, 0x85,
        0xe9, 0x61, 0xbf, 0x25, 0x2b, 0x33, 0x12, 0x62, 0xed, 0xdf, 0xad, 0x0d,
    ];

    #[test]
    fn test_set_state_and_add_resource() {
       
        let prv = XPrv::from_bytes_verified(KEY).expect("Invalid private key");

        let app_name = "App name".to_string();
        let module_id: Ulid = 1.into();
        let event_name = "test_event".to_string();
        let counter = 10;

        // Set tje global state
        set_state(
            app_name.clone(),
            module_id.clone(),
            event_name.clone(),
            counter.clone(),
        );

        // Add the resource
        let id = add_resource(
            &app_name,
            &module_id,
            &event_name,
            &counter,
            prv.clone(),
        );

        // Should return id 1
        assert_eq!(id, Some(1));

        // Get the resource from id
        let k = get_resource(
            &app_name,
            &module_id,
            &event_name,
            &counter,
            &1,
        );

        // The resource should be the same
        assert_eq!(k, Some(prv));
    }
}
