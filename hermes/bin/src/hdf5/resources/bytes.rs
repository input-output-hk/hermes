//! Raw bytes resource implementation.

use std::{
    fmt::{Debug, Display},
    io::Read,
};

use super::ResourceTrait;

/// Raw bytes resource struct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BytesResource {
    /// name of the resource
    name: String,
    /// raw bytes
    data: Vec<u8>,
}

impl Display for BytesResource {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl BytesResource {
    /// Create a new `BytesResource` instance.
    pub(crate) fn new(
        name: String,
        data: Vec<u8>,
    ) -> Self {
        Self { name, data }
    }
}

impl ResourceTrait for BytesResource {
    fn name(&self) -> anyhow::Result<String> {
        Ok(self.name.clone())
    }

    fn is_dir(&self) -> bool {
        false
    }

    fn is_file(&self) -> bool {
        true
    }

    fn get_reader(&self) -> anyhow::Result<impl Read + Debug> {
        Ok(self.data.as_slice())
    }

    fn get_directory_content(&self) -> anyhow::Result<Vec<Self>>
    where Self: Sized {
        Ok(Vec::new())
    }
}
