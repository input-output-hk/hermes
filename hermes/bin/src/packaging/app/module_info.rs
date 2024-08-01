//! An application's module info object

use super::module::ModulePackage;
use crate::{
    hdf5::{Dir, File},
    wasm::module::Module,
};

/// Application package module info.
pub(crate) struct AppModuleInfo {
    /// Module name.
    pub(super) name: String,
    /// Module package.
    pub(super) package: ModulePackage,
    /// Application defined module's `config.json` file
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

    /// Get module's WASM component file
    pub(super) fn get_component_file(&self) -> anyhow::Result<File> {
        self.package.get_component_file()
    }

    /// Get module's WASM metadata file
    pub(super) fn get_metadata_file(&self) -> anyhow::Result<File> {
        self.package.get_metadata_file()
    }

    /// Get module's WASM config schema file
    pub(super) fn get_config_schema_file(&self) -> Option<File> {
        self.package.get_config_schema_file()
    }

    /// Get module's WASM config file
    pub(super) fn get_config_file(&self) -> Option<File> {
        self.app_config.clone().or(self.package.get_config_file())
    }

    /// Get module's WASM settings schema file
    pub(super) fn get_settings_schema_file(&self) -> Option<File> {
        self.package.get_settings_schema_file()
    }

    /// Get module's share dir
    pub(super) fn get_share(&self) -> Option<Dir> {
        self.app_share.clone().or(self.package.get_share_dir())
    }
}
