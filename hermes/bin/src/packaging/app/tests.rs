//! Hermes application package tests.

use temp_dir::TempDir;

use super::*;
use crate::{
    hdf5::resources::ResourceBuilder,
    packaging::sign::{
        certificate::{self, tests::certificate_str},
        keys::tests::private_key_str,
    },
};

struct ApplicationPackageFiles {
    metadata: Metadata<ApplicationPackage>,
    icon: Vec<u8>,
    modules: Vec<module::tests::ModulePackageContent>,
}

#[allow(clippy::unwrap_used)]
fn prepare_default_package_files(modules_num: usize) -> ApplicationPackageFiles {
    let metadata = Metadata::<ApplicationPackage>::from_reader(
        serde_json::json!(
            {
                "$schema": "https://raw.githubusercontent.com/input-output-hk/hermes/main/hermes/schemas/hermes_module_metadata.schema.json",
                "name": "Test app",
                "version": "V1.0.0",
                "description": "Some description",
                "src": ["https://github.com/input-output-hk/hermes"],
                "copyright": ["Copyright Ⓒ 2024, IOG Singapore."],
                "license": [{"spdx": "MIT"}]
            }
        ).to_string().as_bytes(),
    ).unwrap();
    let icon = b"icon_image_svg_content".to_vec();

    let mut modules = Vec::with_capacity(modules_num);
    for _ in 0..modules_num {
        modules.push(module::tests::prepare_default_package_content());
    }

    ApplicationPackageFiles {
        metadata,
        icon,
        modules,
    }
}

#[allow(clippy::unwrap_used)]
fn prepare_package_dir(
    app_name: String, override_module_name: &[String], build_date: DateTime<Utc>, dir: &TempDir,
    app_package_files: &mut ApplicationPackageFiles,
) -> Manifest {
    let metadata_path = dir.path().join("metadata.json");
    let icon_path = dir.path().join("icon.png");

    std::fs::write(
        &metadata_path,
        app_package_files.metadata.to_bytes().unwrap().as_slice(),
    )
    .unwrap();

    std::fs::write(&icon_path, app_package_files.icon.as_slice()).unwrap();

    let mut modules = Vec::new();
    for (i, module_package_files) in app_package_files.modules.iter_mut().enumerate() {
        let default_module_name = format!("module_{i}");
        let mut module_package_path = dir.path().join(&default_module_name);
        module_package_path.set_extension(ModulePackage::FILE_EXTENSION);

        let module_manifest = module::tests::prepare_package_dir(
            default_module_name.clone(),
            dir,
            module_package_files,
        );

        let package =
            ModulePackage::build_from_manifest(&module_manifest, dir.path(), None, build_date)
                .unwrap();

        // WASM module package during the build process updates metadata file
        // to have a corresponded values update `module_package_files`.
        module_package_files.metadata = package.get_metadata().unwrap();

        modules.push(ManifestModule {
            name: override_module_name.get(i).cloned(),
            package: ResourceBuilder::Fs(module_package_path),
            config: None,
            share: None,
        });
    }

    Manifest {
        name: app_name,
        icon: ResourceBuilder::Fs(icon_path),
        metadata: ResourceBuilder::Fs(metadata_path),
        modules,
        www: None,
        share: None,
    }
}

#[test]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
fn from_dir_test() {
    let dir = TempDir::new().unwrap();

    let modules_num = 4;
    let mut app_package_files = prepare_default_package_files(modules_num);

    // override module names for first 2 modules
    let override_module_name = vec!["test_module_1".into(), "test_module_2".into()];
    let build_date = DateTime::default();
    let manifest = prepare_package_dir(
        "app".to_string(),
        &override_module_name,
        build_date,
        &dir,
        &mut app_package_files,
    );

    let package =
        ApplicationPackage::build_from_manifest(&manifest, dir.path(), None, build_date).unwrap();

    assert!(package.validate(true).is_ok());

    // check metadata JSON file
    app_package_files.metadata.set_name(&manifest.name);
    app_package_files.metadata.set_build_date(build_date);

    let package_metadata = package.get_metadata().unwrap();
    assert_eq!(app_package_files.metadata, package_metadata);

    // check icon file
    assert!(package.get_icon_file().is_ok());

    // check WASM modules
    let modules = package.get_modules().unwrap();
    assert_eq!(modules.len(), app_package_files.modules.len());

    for module_info in modules {
        // taking not overridden module name
        let package_module_name = module_info.get_metadata().unwrap().get_name().unwrap();
        // searching by this name from the prepared app package files
        let (i, module_files) = app_package_files
            .modules
            .iter_mut()
            .enumerate()
            .find(|(_, module)| module.metadata.get_name().unwrap() == *package_module_name)
            .unwrap();

        // taking overridden module name (optional)
        let manifest_module_name = manifest.modules[i].name.clone();
        assert_eq!(
            module_info.get_name(),
            manifest_module_name.unwrap_or(module_files.metadata.get_name().unwrap())
        );
        module_info.check_module_integrity(module_files);
    }
}

#[test]
#[allow(clippy::unwrap_used)]
fn author_sing_test() {
    let dir = TempDir::new().unwrap();

    let modules_num = 4;
    let mut app_package_files = prepare_default_package_files(modules_num);

    // override module names for first 2 modules
    let override_module_name = vec!["test module 1".into(), "test module 2".into()];
    let build_date = DateTime::default();
    let manifest = prepare_package_dir(
        "app".to_string(),
        &override_module_name,
        build_date,
        &dir,
        &mut app_package_files,
    );

    let package =
        ApplicationPackage::build_from_manifest(&manifest, dir.path(), None, build_date).unwrap();

    assert!(package.validate(true).is_ok());
    assert!(package.validate(false).is_err());
    assert!(package.get_author_signature().unwrap().is_none());

    let private_key = PrivateKey::from_str(&private_key_str()).unwrap();
    let certificate = Certificate::from_str(&certificate_str()).unwrap();

    // sign wasm modules packages first
    for module_info in package.get_modules().unwrap() {
        module_info.sign(&private_key, &certificate).unwrap();
    }

    package.author_sign(&private_key, &certificate).unwrap();
    package.author_sign(&private_key, &certificate).unwrap();

    assert!(package.get_author_signature().unwrap().is_some());

    assert!(
        package.validate(false).is_err(),
        "Missing certificate in the storage."
    );

    certificate::storage::add_certificate(certificate).unwrap();
    assert!(package.validate(false).is_ok());

    // corrupt payload with the modifying metadata.json file
    app_package_files.metadata.set_name("New name");
    package
        .0
        .remove_file(ApplicationPackage::METADATA_FILE.into())
        .unwrap();
    package
        .0
        .copy_resource_file(
            &BytesResource::new(
                ApplicationPackage::METADATA_FILE.to_string(),
                app_package_files.metadata.to_bytes().unwrap(),
            ),
            ApplicationPackage::METADATA_FILE.into(),
        )
        .unwrap();

    assert!(
        package.validate(false).is_err(),
        "Corrupted signature payload."
    );
}
