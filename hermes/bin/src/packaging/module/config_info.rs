//! Hermes WASM module's config info object.

use super::{Config, ConfigSchema};

/// Config info object.
pub(crate) struct ConfigInfo {
    /// Config schema.
    #[allow(dead_code)]
    pub(crate) schema: ConfigSchema,
    /// Config value itself.
    #[allow(dead_code)]
    pub(crate) val: Option<Config>,
}
