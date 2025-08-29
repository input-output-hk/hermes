//! An application's module info object

use super::{
    module::{Config, ConfigInfo, SignaturePayload},
    Metadata, ModulePackage, Signature,
};
use crate::{
    app::ApplicationName,
    hdf5::{Dir, File},
    wasm::module::Module,
};

/// Application package module info.
pub(crate) struct AppModuleInfo {
    /// Module name.
    name: String,
    /// Module package.
    package: ModulePackage,
    /// Application defined module's `config.json` file
    app_config: Option<File>,
    /// Application defined module's `share` directory
    app_share: Option<Dir>,
}

impl AppModuleInfo {
    /// Create a new `AppModuleInfo` instance
    pub(crate) fn new(
        name: String,
        package: ModulePackage,
        app_config: Option<File>,
        app_share: Option<Dir>,
    ) -> Self {
        Self {
            name,
            package,
            app_config,
            app_share,
        }
    }

    /// Get module's name
    pub(crate) fn get_name(&self) -> String {
        self.name.clone()
    }

    /// Validate module package with its signature and other contents.
    /// If `untrusted` flag is `true` the signature will not be verified.
    pub(crate) fn validate(
        &self,
        untrusted: bool,
    ) -> anyhow::Result<()> {
        self.package.validate(untrusted)
    }

    /// Get module's WASM component
    pub(crate) fn get_component(
        &self,
        app_name: &ApplicationName,
    ) -> anyhow::Result<Module> {
        self.package.get_component(app_name)
    }

    /// Get module's metadata
    #[allow(dead_code)]
    pub(crate) fn get_metadata(&self) -> anyhow::Result<Metadata<ModulePackage>> {
        self.package.get_metadata()
    }

    /// Get module's author signature
    pub(crate) fn get_signature(&self) -> anyhow::Result<Option<Signature<SignaturePayload>>> {
        self.package.get_signature()
    }

    /// Get module's config info
    #[allow(dead_code)]
    pub(crate) fn get_config_info(&self) -> anyhow::Result<Option<ConfigInfo>> {
        let Some(mut config_info) = self.package.get_config_info()? else {
            return Ok(None);
        };

        if let Some(app_config) = self.app_config.clone() {
            let app_config = Config::from_reader(app_config, config_info.schema.validator())?;
            config_info.val = Some(app_config);
        }
        Ok(Some(config_info))
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
    pub(super) fn get_share_dir(&self) -> Option<Dir> {
        self.app_share.clone().or(self.package.get_share_dir())
    }
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use super::{
        super::{
            super::sign::{certificate::Certificate, keys::PrivateKey},
            module::tests::{check_module_package_integrity, ModulePackageContent},
        },
        *,
    };

    impl AppModuleInfo {
        pub(crate) fn check_module_package_integrity(
            &self,
            module_files: &ModulePackageContent,
        ) {
            check_module_package_integrity(module_files, &self.package);
        }

        pub(crate) fn sign(
            &self,
            private_key: &PrivateKey,
            certificate: &Certificate,
        ) -> anyhow::Result<()> {
            self.package.sign(private_key, certificate)
        }
    }
}
