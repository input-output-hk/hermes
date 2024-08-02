//! Hermes WASI runtime context.

use std::collections::HashMap;

use super::descriptors::{Descriptor, Stream};
use crate::hdf5::Dir;

/// WASI context errors.
pub(crate) enum Error {
    /// Represents trying to reference a descriptor or stream identifier that does not
    /// exist.
    NoEntry,
}

/// WASI context result type.
pub(crate) type Result<T> = std::result::Result<T, Error>;

/// Contains all data needed to execute the WASI APIs.
#[derive(Clone, Debug)]
pub(crate) struct WasiContext {
    /// Descriptors currently opened in this context.
    descriptors: HashMap<u32, Descriptor>,
    /// Input streams currently opened in this context.
    input_streams: HashMap<u32, Stream>,
    /// Output streams currently opened in this context.
    output_streams: HashMap<u32, Stream>,
    /// List of preopen directories in this context.
    preopens: Vec<(u32, String)>,
}

impl WasiContext {
    /// Creates a new empty WASI context.
    pub fn new() -> Self {
        Self {
            descriptors: HashMap::new(),
            input_streams: HashMap::new(),
            output_streams: HashMap::new(),
            preopens: Vec::new(),
        }
    }

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
        self.input_streams.remove(&rep);
        self.output_streams.remove(&rep);
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

    /// Stores in this WASI context a new output stream associated to the
    /// [`Descriptor`] with the given identifier.
    ///
    /// Fails if there's no descriptor with the given id.
    pub fn put_output_stream(&mut self, desc_rep: u32, offset: u64) -> Result<()> {
        if !self.descriptors.contains_key(&desc_rep) {
            return Err(Error::NoEntry);
        }

        self.output_streams.insert(desc_rep, Stream::new(offset));

        Ok(())
    }

    /// Removes the output stream associated with the given descriptor identifier.
    pub fn remove_output_stream(&mut self, desc_rep: u32) {
        self.output_streams.remove(&desc_rep);
    }

    /// Gets a mutable reference to the output stream associated with the given
    /// descriptor identifier.
    ///
    /// Returns [`None`] if there's not such output stream.
    pub fn output_stream_mut(&mut self, desc_rep: u32) -> Option<&mut Stream> {
        self.output_streams.get_mut(&desc_rep)
    }

    /// Stores in this WASI context a new input stream associated to the [`Descriptor`]
    /// with the given identifier.
    ///
    /// Fails if there's no descriptor with the given id.
    pub fn put_input_stream(&mut self, desc_rep: u32, offset: u64) -> Result<()> {
        if !self.descriptors.contains_key(&desc_rep) {
            return Err(Error::NoEntry);
        }

        self.input_streams.insert(desc_rep, Stream::new(offset));

        Ok(())
    }

    /// Removes the input stream associated with the given descriptor identifier.
    pub fn remove_input_stream(&mut self, desc_rep: u32) {
        self.input_streams.remove(&desc_rep);
    }

    /// Gets a mutable reference to the input stream associated with the given
    /// descriptor identifier.
    ///
    /// Returns [`None`] if there's not such output stream.
    pub fn input_stream_mut(&mut self, desc_rep: u32) -> Option<&mut Stream> {
        self.input_streams.get_mut(&desc_rep)
    }

    /// Adds a preopen directory to the preopens list.
    pub fn put_preopen_dir(&mut self, path: String, dir: Dir) -> u32 {
        let rep = self.put_descriptor(Descriptor::Dir(dir));
        self.preopens.push((rep, path));

        rep
    }

    /// Returns the list of descriptor identifiers and paths of the current preopen
    /// directories.
    pub fn preopen_dirs(&self) -> &Vec<(u32, String)> {
        &self.preopens
    }
}
