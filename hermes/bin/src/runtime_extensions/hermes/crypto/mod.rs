//! Crypto runtime extension implementation.

use std::collections::HashMap;

use crate::runtime_extensions::{
    bindings::hermes::crypto::api::Ed25519Bip32PrivateKey,
    state::{Context, Stateful},
};

mod host;

#[derive(Default, Clone)]

/// Resource holder: a simple resource manager.
struct ResHolder<T> {
    /// Map of resources.
    resources: HashMap<u32, T>,
    /// The next id that will be used.
    next_id: u32,
}

impl<T> ResHolder<T> {
    /// Insert new Resource where item is added with the id.
    /// Id will be incremented by 1 each time.
    fn add(&mut self, item: T) -> u32 {
        let id = self.next_id;
        self.resources.insert(id, item);
        self.next_id += 1;
        id
    }

    /// Get the item from resources using id if possible.
    fn get(&self, id: u32) -> Option<&T> {
        self.resources.get(&id)
    }

    /// Drop the item from resources using id if possible.
    fn drop(&mut self, id: u32) -> Result<(), ()> {
        self.resources.remove(&id).map(|_| ()).ok_or(())
    }
}

/// State
pub(crate) struct State {
    /// Resource of Ed25519-Bip32 private key.
    private_key: ResHolder<Ed25519Bip32PrivateKey>,
}

impl Stateful for State {
    fn new(_ctx: &Context) -> Self {
        State {
            private_key: ResHolder::default(),
        }
    }
}
