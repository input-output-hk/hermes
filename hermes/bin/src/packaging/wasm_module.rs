//! Wasm module package.

use std::path::Path;

use super::{copy_file_from_dir_to_package, FileNotFoundError};
use crate::errors::Errors;

pub(crate) struct WasmModulePackage {
    package: hdf5::File,
}

impl WasmModulePackage {
    const CONFIG_JSON: &'static str = "config.json";
    const CONFIG_SCHEMA_JSON: &'static str = "config.schema.json";
    const METDATA_JSON: &'static str = "metadata.json";
    const MODULE_WASM: &'static str = "module.wasm";
    const SETTINGS_SCHEMA_JSON: &'static str = "settings.schema.json";

    pub(crate) fn from_dir<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let mut errors = Errors::new();

        let package = hdf5::File::create(path.join("module.hdf5"))?;

        match copy_file_from_dir_to_package(path, Self::CONFIG_SCHEMA_JSON, &package) {
            Ok(()) => {
                copy_file_from_dir_to_package(path, Self::CONFIG_JSON, &package)
                    .unwrap_or_else(|err| errors.add_err(err));
            },
            Err(err) if err.is::<FileNotFoundError>() => {},
            Err(err) => errors.add_err(err),
        }

        copy_file_from_dir_to_package(path, Self::METDATA_JSON, &package)
            .unwrap_or_else(|err| errors.add_err(err));
        copy_file_from_dir_to_package(path, Self::MODULE_WASM, &package)
            .unwrap_or_else(|err| errors.add_err(err));
        copy_file_from_dir_to_package(path, Self::SETTINGS_SCHEMA_JSON, &package)
            .or_else(|err| err.is::<FileNotFoundError>().then_some(()).ok_or(err))
            .unwrap_or_else(|err| errors.add_err(err));

        errors.return_result(Self { package })
    }
}

#[cfg(test)]
mod tests {
    use temp_dir::TempDir;

    use super::*;

    #[test]
    fn from_dir_test() {
        let dir = TempDir::new().expect("cannot create temp dir");

        let config_json = dir.path().join(WasmModulePackage::CONFIG_JSON);
        let config_schema_json = dir.path().join(WasmModulePackage::CONFIG_SCHEMA_JSON);
        let metadata_json = dir.path().join(WasmModulePackage::METDATA_JSON);
        let module_wasm_path = dir.path().join(WasmModulePackage::MODULE_WASM);
        let settings_schema_json = dir.path().join(WasmModulePackage::SETTINGS_SCHEMA_JSON);

        std::fs::File::create(config_json).expect("Cannot create config.json file");
        std::fs::File::create(config_schema_json).expect("Cannot create config.schema.json file");
        std::fs::File::create(metadata_json).expect("Cannot create metadata.json file");
        std::fs::File::create(module_wasm_path).expect("Cannot create module.wasm file");
        std::fs::File::create(settings_schema_json)
            .expect("Cannot create settings.schema.json file");

        WasmModulePackage::from_dir(dir.path()).expect("Cannot create module package");
    }

    #[test]
    fn from_dir_some_files_missing_test() {
        let dir = TempDir::new().expect("cannot create temp dir");

        // let metadata_json = dir.path().join(WasmModulePackage::METDATA_JSON);
        // let module_wasm_path = dir.path().join(WasmModulePackage::MODULE_WASM);

        // std::fs::File::create(metadata_json).expect("Cannot create metadata.json file");
        // std::fs::File::create(module_wasm_path).expect("Cannot create module.wasm file");

        WasmModulePackage::from_dir(dir.path()).expect("Cannot create module package");
    }
}
