//! An application's module info object

use crate::{
    hdf5::{Dir, File},
    packaging::module::ModulePackage,
    wasm::module::Module,
};

/// Application package module info.
pub(crate) struct AppModuleInfo {
    /// Module name.
    pub(super) name: String,
    /// Module package.
    pub(super) package: ModulePackage,
    /// Application defined module's `config.json` file
    #[allow(dead_code)]
    pub(super) app_config: Option<File>,
    /// Application defined module's `share` directory
    pub(super) app_share: Option<Dir>,
}

impl AppModuleInfo {
    /// Get module's name
    pub(crate) fn get_name(&self) -> String {
        self.name.clone()
    }

    /// Get module's WASM component
    pub(crate) fn get_component(&self) -> anyhow::Result<Module> {
        self.package.get_component()
    }

    /// Get module's share dir
    pub(crate) fn get_share(&self) -> Option<Dir> {
        self.app_share.clone().or(self.package.get_share())
    }
}
