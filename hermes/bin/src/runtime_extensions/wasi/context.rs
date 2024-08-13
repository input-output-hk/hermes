//! Hermes WASI runtime context.

use std::collections::HashMap;

use super::descriptors::Descriptor;

/// Contains all data needed to execute the WASI APIs.
#[derive(Clone, Debug, Default)]
pub(crate) struct WasiContext {
    /// Descriptors currently opened in this context.
    descriptors: HashMap<u32, Descriptor>,
}

impl WasiContext {
    /// Stores a new [`Descriptor`] in this WASI context.
    ///
    /// Returns the identifier used to reference the descriptor.
    pub fn put_descriptor(&mut self, desc: Descriptor) -> u32 {
        loop {
            // We add 2 here in order to reserve rep = 0 to STDOUT and
            // rep = 1 to STDERR.
            let rep = rand::random::<u32>().saturating_add(2);

            if self.descriptors.contains_key(&rep) {
                continue;
            }

            self.descriptors.insert(rep, desc);
            break rep;
        }
    }

    /// Removes a [`Descriptor`] from this WASI context.
    ///
    /// This also closes the handle and the streams associated with it.
    pub fn remove_descriptor(&mut self, rep: u32) {
        self.descriptors.remove(&rep);
    }

    /// Gets a reference to the [`Descriptor`] with the given identifier.
    ///
    /// Returns [`None`] if there's not descriptor with the given id.
    pub fn descriptor(&self, rep: u32) -> Option<&Descriptor> {
        self.descriptors.get(&rep)
    }

    /// Gets a mutable reference to the [`Descriptor`] with the given identifier.
    ///
    /// Returns [`None`] if there's not descriptor with the given id.
    pub fn descriptor_mut(&mut self, rep: u32) -> Option<&mut Descriptor> {
        self.descriptors.get_mut(&rep)
    }
}
