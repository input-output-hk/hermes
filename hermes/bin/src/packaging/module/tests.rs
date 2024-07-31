//! Hermes WASM package tests.

use temp_dir::TempDir;

use super::*;
use crate::{
    hdf5::resources::ResourceBuilder,
    packaging::sign::{
        certificate::{self, tests::certificate_str},
        keys::tests::private_key_str,
    },
};

pub(crate) struct ModulePackageFiles {
    pub(crate) metadata: Metadata<ModulePackage>,
    pub(crate) component: Vec<u8>,
    pub(crate) config_schema: ConfigSchema,
    pub(crate) config: Config,
    pub(crate) settings_schema: SettingsSchema,
}

#[allow(clippy::unwrap_used)]
pub(crate) fn prepare_default_package_content() -> ModulePackageFiles {
    let metadata = Metadata::<ModulePackage>::from_reader(
        serde_json::json!(
            {
                "$schema": "https://raw.githubusercontent.com/input-output-hk/hermes/main/hermes/schemas/hermes_module_metadata.schema.json",
                "name": "Test module",
                "version": "V1.0.0",
                "description": "Some description",
                "src": ["https://github.com/input-output-hk/hermes"],
                "copyright": ["Copyright Ⓒ 2024, IOG Singapore."],
                "license": [{"spdx": "MIT"}]
            }
        ).to_string().as_bytes(),
    ).unwrap();
    let config_schema =
        ConfigSchema::from_reader(serde_json::json!({}).to_string().as_bytes()).unwrap();

    let config = Config::from_reader(
        serde_json::json!({}).to_string().as_bytes(),
        config_schema.validator(),
    )
    .unwrap();

    let settings_schema =
        SettingsSchema::from_reader(serde_json::json!({}).to_string().as_bytes()).unwrap();

    let component = br#"
        (component
            (core module $Module
                (export "foo" (func $foo))
                (func $foo (result i32)
                    i32.const 1
                )
            )
            (core instance $module (instantiate (module $Module)))
            (func $foo (result s32) (canon lift (core func $module "foo")))
            (export "foo" (func $foo))
        )"#
    .to_vec();

    ModulePackageFiles {
        metadata,
        component,
        config_schema,
        config,
        settings_schema,
    }
}

#[allow(clippy::unwrap_used)]
pub(crate) fn prepare_package_dir(
    module_name: String, dir: &TempDir, module_package_files: &ModulePackageFiles,
) -> Manifest {
    let config_path = dir.path().join("config.json");
    let config_schema_path = dir.path().join("config.schema.json");
    let metadata_path = dir.path().join("metadata.json");
    let component_path = dir.path().join("module.wasm");
    let settings_schema_path = dir.path().join("settings.schema.json");

    std::fs::write(
        &metadata_path,
        module_package_files.metadata.to_bytes().unwrap().as_slice(),
    )
    .unwrap();
    std::fs::write(&component_path, module_package_files.component.as_slice()).unwrap();
    std::fs::write(
        &config_path,
        module_package_files.config.to_bytes().unwrap().as_slice(),
    )
    .unwrap();
    std::fs::write(
        &config_schema_path,
        module_package_files
            .config_schema
            .to_bytes()
            .unwrap()
            .as_slice(),
    )
    .unwrap();
    std::fs::write(
        &settings_schema_path,
        module_package_files
            .settings_schema
            .to_bytes()
            .unwrap()
            .as_slice(),
    )
    .unwrap();

    Manifest {
        name: module_name,
        metadata: ResourceBuilder::Fs(metadata_path),
        component: ResourceBuilder::Fs(component_path),
        config: manifest::ManifestConfig {
            file: Some(ResourceBuilder::Fs(config_path)),
            schema: ResourceBuilder::Fs(config_schema_path),
        }
        .into(),
        settings: manifest::ManifestSettings {
            schema: ResourceBuilder::Fs(settings_schema_path),
        }
        .into(),
        share: None,
    }
}

#[allow(clippy::unwrap_used)]
pub(crate) fn check_module_integrity(
    module_files: &ModulePackageFiles, module_package: &ModulePackage,
) {
    let package_metadata = module_package.get_metadata().unwrap();
    assert_eq!(module_files.metadata, package_metadata);

    // check WASM component file
    assert!(module_package.get_component().is_ok());

    // check config and config schema JSON files
    let config_info = module_package.get_config_info().unwrap().unwrap();
    assert_eq!(module_files.config, config_info.val.unwrap());
    assert_eq!(module_files.config_schema, config_info.schema);

    // check settings schema JSON file
    let package_settings_schema = module_package.get_settings_schema().unwrap();
    assert_eq!(
        module_files.settings_schema,
        package_settings_schema.unwrap()
    );
}

#[test]
#[allow(clippy::unwrap_used)]
fn from_dir_test() {
    let dir = TempDir::new().unwrap();

    let mut module_package_files = prepare_default_package_content();

    let manifest = prepare_package_dir("module".to_string(), &dir, &module_package_files);

    let build_time = DateTime::default();
    let package =
        ModulePackage::build_from_manifest(&manifest, dir.path(), None, build_time).unwrap();

    assert!(package.validate(true).is_ok());

    // Module package during the build process updates metadata file
    // to have a corresponded values update `module_package_files`.
    module_package_files.metadata.set_name(&manifest.name);
    module_package_files.metadata.set_build_date(build_time);

    // check module package integrity
    let package_metadata = package.get_metadata().unwrap();
    assert_eq!(module_package_files.metadata, package_metadata);

    // check WASM component file
    assert!(package.get_component().is_ok());

    // check config and config schema JSON files
    let config_info = package.get_config_info().unwrap().unwrap();
    assert_eq!(module_package_files.config, config_info.val.unwrap());
    assert_eq!(module_package_files.config_schema, config_info.schema);

    // check settings schema JSON file
    let package_settings_schema = package.get_settings_schema().unwrap();
    assert_eq!(
        module_package_files.settings_schema,
        package_settings_schema.unwrap()
    );
}

#[test]
#[allow(clippy::unwrap_used)]
fn sign_test() {
    let dir = TempDir::new().unwrap();

    let module_package_files = prepare_default_package_content();

    let manifest = prepare_package_dir("module".to_string(), &dir, &module_package_files);

    let build_time = DateTime::default();
    let package =
        ModulePackage::build_from_manifest(&manifest, dir.path(), None, build_time).unwrap();

    assert!(package.validate(true).is_ok());
    assert!(package.validate(false).is_err());
    assert!(package.get_signature().unwrap().is_none());

    let private_key = PrivateKey::from_str(&private_key_str()).unwrap();
    let certificate = Certificate::from_str(&certificate_str()).unwrap();
    package.sign(&private_key, &certificate).unwrap();
    package.sign(&private_key, &certificate).unwrap();

    assert!(package.get_signature().unwrap().is_some());

    assert!(
        package.validate(false).is_err(),
        "Missing certificate in the storage."
    );

    certificate::storage::add_certificate(certificate).unwrap();
    assert!(package.validate(false).is_ok());
}

#[allow(clippy::unwrap_used)]
fn sign_package(package: &ModulePackage) {
    let private_key = PrivateKey::from_str(&private_key_str()).unwrap();
    let certificate = Certificate::from_str(&certificate_str()).unwrap();
    package.sign(&private_key, &certificate).unwrap();
    certificate::storage::add_certificate(certificate).unwrap();
    assert!(package.validate(false).is_ok());
}

#[test]
#[allow(clippy::unwrap_used)]
fn corrupted_metadata_test() {
    let dir = TempDir::new().unwrap();

    let module_package_files = prepare_default_package_content();

    let manifest = prepare_package_dir("module".to_string(), &dir, &module_package_files);

    let build_time = DateTime::default();
    let package =
        ModulePackage::build_from_manifest(&manifest, dir.path(), None, build_time).unwrap();

    sign_package(&package);

    {
        package
            .0
            .remove_file(ModulePackage::METADATA_FILE.into())
            .unwrap();
        assert!(
            package.validate(false).is_err(),
            "Missing required metadata file."
        );
    }

    {
        let new_metadata = Metadata::<ModulePackage>::from_reader(
            serde_json::json!(
                {
                    "$schema": "https://raw.githubusercontent.com/input-output-hk/hermes/main/hermes/schemas/hermes_module_metadata.schema.json",
                    "name": "new test module",
                    "version": "V1.0.0",
                    "description": "Some new description",
                    "src": ["https://github.com/input-output-hk/hermes"],
                    "copyright": ["Copyright Ⓒ 2024, IOG Singapore."],
                    "license": [{"spdx": "MIT"}]
                }
            ).to_string().as_bytes(),
        ).unwrap();
        assert_ne!(module_package_files.metadata, new_metadata);

        package
            .0
            .copy_resource_file(
                &BytesResource::new(
                    ModulePackage::METADATA_FILE.to_string(),
                    new_metadata.to_bytes().unwrap(),
                ),
                ModulePackage::METADATA_FILE.into(),
            )
            .unwrap();

        assert!(package.get_metadata().is_ok());
        assert!(
            package.validate(false).is_err(),
            "Corrupted signature payload."
        );
    }
}

#[test]
#[allow(clippy::unwrap_used)]
fn corrupted_component_test() {
    let dir = TempDir::new().unwrap();

    let module_package_files = prepare_default_package_content();

    let manifest = prepare_package_dir("module".to_string(), &dir, &module_package_files);

    let build_time = DateTime::default();
    let package =
        ModulePackage::build_from_manifest(&manifest, dir.path(), None, build_time).unwrap();

    sign_package(&package);

    {
        package
            .0
            .remove_file(ModulePackage::COMPONENT_FILE.into())
            .unwrap();
        assert!(
            package.validate(false).is_err(),
            "Missing required component file."
        );
    }

    {
        let new_component = br#"
        (component
            (core module $Module
                (export "bar" (func $bar))
                (func $bar (result i32)
                    i32.const 1
                )
            )
            (core instance $module (instantiate (module $Module)))
            (func $bar (result s32) (canon lift (core func $module "bar")))
            (export "bar" (func $bar))
        )"#;
        assert_ne!(module_package_files.component.as_slice(), new_component);

        package
            .0
            .copy_resource_file(
                &BytesResource::new(
                    ModulePackage::COMPONENT_FILE.to_string(),
                    new_component.to_vec(),
                ),
                ModulePackage::COMPONENT_FILE.into(),
            )
            .unwrap();

        assert!(package.get_component().is_ok());
        assert!(
            package.validate(false).is_err(),
            "Corrupted signature payload."
        );
    }
}

#[test]
#[allow(clippy::unwrap_used)]
fn corrupted_config_test() {
    let dir = TempDir::new().unwrap();

    let module_package_files = prepare_default_package_content();

    let manifest = prepare_package_dir("module".to_string(), &dir, &module_package_files);

    let build_time = DateTime::default();
    let package =
        ModulePackage::build_from_manifest(&manifest, dir.path(), None, build_time).unwrap();

    sign_package(&package);

    {
        package
            .0
            .remove_file(ModulePackage::CONFIG_FILE.into())
            .unwrap();
        let config_info = package.get_config_info().unwrap().unwrap();
        assert!(config_info.val.is_none());
        assert!(
            package.validate(false).is_err(),
            "Corrupted signature payload."
        );
    }

    {
        let config_schema = package.get_config_schema().unwrap().unwrap();
        let new_config = Config::from_reader(
            serde_json::json!({
                "new_prop": "new value",
            })
            .to_string()
            .as_bytes(),
            config_schema.validator(),
        )
        .unwrap();
        assert_ne!(module_package_files.config, new_config);

        package
            .0
            .copy_resource_file(
                &BytesResource::new(
                    ModulePackage::CONFIG_FILE.to_string(),
                    new_config.to_bytes().unwrap(),
                ),
                ModulePackage::CONFIG_FILE.into(),
            )
            .unwrap();

        let config_info = package.get_config_info().unwrap().unwrap();
        assert!(config_info.val.is_some());
        assert!(
            package.validate(false).is_err(),
            "Corrupted signature payload."
        );
    }
}

#[test]
#[allow(clippy::unwrap_used)]
fn corrupted_config_schema_test() {
    let dir = TempDir::new().unwrap();

    let module_package_files = prepare_default_package_content();

    let manifest = prepare_package_dir("module".to_string(), &dir, &module_package_files);

    let build_time = DateTime::default();
    let package =
        ModulePackage::build_from_manifest(&manifest, dir.path(), None, build_time).unwrap();

    sign_package(&package);

    {
        package
            .0
            .remove_file(ModulePackage::CONFIG_SCHEMA_FILE.into())
            .unwrap();
        let config_info = package.get_config_info().unwrap();
        assert!(config_info.is_none());
        assert!(
            package.validate(false).is_err(),
            "Corrupted signature payload."
        );
    }

    {
        let new_config_schema = ConfigSchema::from_reader(
            serde_json::json!({
                "title": "Test empty schema",
                "type": "object",
                "properties": {}
            })
            .to_string()
            .as_bytes(),
        )
        .unwrap();
        assert_ne!(module_package_files.config_schema, new_config_schema);

        package
            .0
            .copy_resource_file(
                &BytesResource::new(
                    ModulePackage::CONFIG_SCHEMA_FILE.to_string(),
                    new_config_schema.to_bytes().unwrap(),
                ),
                ModulePackage::CONFIG_SCHEMA_FILE.into(),
            )
            .unwrap();

        let config_info = package.get_config_info().unwrap();
        assert!(config_info.is_some());
        assert!(
            package.validate(false).is_err(),
            "Corrupted signature payload."
        );
    }
}

#[test]
#[allow(clippy::unwrap_used)]
fn corrupted_settings_schema_test() {
    let dir = TempDir::new().unwrap();

    let module_package_files = prepare_default_package_content();

    let manifest = prepare_package_dir("module".to_string(), &dir, &module_package_files);

    let build_time = DateTime::default();
    let package =
        ModulePackage::build_from_manifest(&manifest, dir.path(), None, build_time).unwrap();

    sign_package(&package);

    {
        package
            .0
            .remove_file(ModulePackage::SETTINGS_SCHEMA_FILE.into())
            .unwrap();
        let settings_schema = package.get_settings_schema().unwrap();
        assert!(settings_schema.is_none());
        assert!(
            package.validate(false).is_err(),
            "Corrupted signature payload."
        );
    }

    {
        let new_settings_schema = SettingsSchema::from_reader(
            serde_json::json!({
                "title": "Test empty schema",
                "type": "object",
                "properties": {}
            })
            .to_string()
            .as_bytes(),
        )
        .unwrap();
        assert_ne!(module_package_files.settings_schema, new_settings_schema);

        package
            .0
            .copy_resource_file(
                &BytesResource::new(
                    ModulePackage::SETTINGS_SCHEMA_FILE.to_string(),
                    new_settings_schema.to_bytes().unwrap(),
                ),
                ModulePackage::SETTINGS_SCHEMA_FILE.into(),
            )
            .unwrap();

        let settings_schema = package.get_settings_schema().unwrap();
        assert!(settings_schema.is_some());
        assert!(
            package.validate(false).is_err(),
            "Corrupted signature payload."
        );
    }
}
