//! Hermes WASM module's config info object.

use super::{Config, ConfigSchema};

/// Config info object.
pub(crate) struct ConfigInfo {
    /// Config schema.
    pub(crate) schema: ConfigSchema,
    /// Config value itself.
    pub(crate) val: Option<Config>,
}
