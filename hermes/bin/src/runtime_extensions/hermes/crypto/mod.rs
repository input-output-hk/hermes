//! Crypto runtime extension implementation.

use std::collections::HashMap;

use crate::runtime_extensions::{
    bindings::hermes::crypto::api::{Ed25519Bip32PrivateKey},
    state::{Context, Stateful},
};

mod host;

#[derive(Default, Clone)]
struct ResHolder<T> {
    resources: HashMap<u32, T>,
    next_id: u32,
}

impl<T> ResHolder<T> {
    fn new(&mut self, item: T) -> u32 {
        let id = self.next_id;
        self.resources.insert(id, item);
        self.next_id += 1;
        id
    }

    fn get(&self, id: u32) -> Option<&T> {
        self.resources.get(&id)
    }

    fn drop(&mut self, id: u32) -> Result<(), ()> {
        self.resources.remove(&id).map(|_| ()).ok_or(())
    }
}

#[derive(Default, Clone)]
#[allow(dead_code)]
struct Ed25519Bip32Struct {
    private_key: Ed25519Bip32PrivateKey,
}

/// State
pub(crate) struct State {
    private_key: ResHolder<Ed25519Bip32Struct>,
}

impl Stateful for State {
    fn new(_ctx: &Context) -> Self {
        State {
            private_key: ResHolder::default(),
        }
    }
}
