//! Hermes application package tests.

use std::io::Write;

use module::{
    tests::{
        check_package_dir_integrity, prepare_package_dir_dir, ModulePackageContent,
        PackageDirContent,
    },
    Config,
};
use temp_dir::TempDir;

use super::*;
use crate::{
    hdf5::resources::ResourceBuilder,
    packaging::sign::{
        certificate::{self, tests::certificate_str},
        keys::tests::private_key_str,
    },
};

struct ApplicationPackageContent {
    metadata: Metadata<ApplicationPackage>,
    icon: Vec<u8>,
    modules: Vec<AppModulePackageContent>,
    share: PackageDirContent,
    www: PackageDirContent,
}

struct AppModulePackageContent {
    module: ModulePackageContent,
    share: PackageDirContent,
    config: Config,
}

#[allow(clippy::unwrap_used)]
fn prepare_default_package_content(modules_num: usize) -> ApplicationPackageContent {
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
    for i in 0..modules_num {
        let module = module::tests::prepare_default_package_content();
        let share = PackageDirContent {
            child_dir_name: format!("module_{i}_share_child"),
            file: ("file.txt".to_string(), b"file content".to_vec()),
        };
        let config = Config::from_reader(
            serde_json::json!({
                "config_prop": format!("module_{i}_config_prop")

            })
            .to_string()
            .as_bytes(),
            module.config_schema.validator(),
        )
        .unwrap();

        modules.push(AppModulePackageContent {
            module,
            share,
            config,
        });
    }

    let share = PackageDirContent {
        child_dir_name: "share_child".to_string(),
        file: ("file.txt".to_string(), b"file content".to_vec()),
    };
    let www = PackageDirContent {
        child_dir_name: "www_child".to_string(),
        file: ("file.txt".to_string(), b"file content".to_vec()),
    };

    ApplicationPackageContent {
        metadata,
        icon,
        modules,
        share,
        www,
    }
}

#[allow(clippy::unwrap_used)]
fn prepare_package_dir(
    app_name: String, override_module_name: &[String], build_date: DateTime<Utc>,
    dir: &std::path::Path, app_package_content: &mut ApplicationPackageContent,
) -> Manifest {
    let app_dir = dir.join(&app_name);
    let metadata_path = app_dir.join("metadata.json");
    let icon_path = app_dir.join("icon.png");
    let share_path = app_dir.join("share");
    let www_path = app_dir.join("www");

    std::fs::create_dir(&app_dir).unwrap();
    std::fs::write(
        &metadata_path,
        app_package_content.metadata.to_bytes().unwrap().as_slice(),
    )
    .unwrap();

    std::fs::write(&icon_path, app_package_content.icon.as_slice()).unwrap();

    std::fs::create_dir(&share_path).unwrap();
    prepare_package_dir_dir(&share_path, &app_package_content.share);

    std::fs::create_dir(&www_path).unwrap();
    prepare_package_dir_dir(&www_path, &app_package_content.www);

    let mut modules = Vec::new();
    for (i, module_package_files) in app_package_content.modules.iter_mut().enumerate() {
        let default_module_name = format!("module_{i}");
        let mut module_package_path = dir.join(&default_module_name);
        module_package_path.set_extension(ModulePackage::FILE_EXTENSION);

        let module_manifest = module::tests::prepare_module_package_dir(
            default_module_name.clone(),
            dir,
            &module_package_files.module,
        );

        let package =
            ModulePackage::build_from_manifest(&module_manifest, dir, None, build_date).unwrap();

        // WASM module package during the build process updates metadata file
        // to have a corresponded values update `module_package_files`.
        module_package_files.module.metadata = package.get_metadata().unwrap();

        let app_module_share_path = app_dir.join(format!("app_module_{i}_share"));
        std::fs::create_dir(&app_module_share_path).unwrap();
        prepare_package_dir_dir(&app_module_share_path, &module_package_files.share);

        let config_path = app_dir.join(format!("app_module_{i}_config.json"));
        std::fs::write(
            &config_path,
            module_package_files.config.to_bytes().unwrap().as_slice(),
        )
        .unwrap();

        modules.push(ManifestModule {
            name: override_module_name.get(i).cloned(),
            package: ResourceBuilder::Fs(module_package_path),
            config: Some(ResourceBuilder::Fs(config_path)),
            share: Some(ResourceBuilder::Fs(app_module_share_path)),
        });
    }

    Manifest {
        name: app_name,
        icon: ResourceBuilder::Fs(icon_path),
        metadata: ResourceBuilder::Fs(metadata_path),
        modules,
        www: Some(ResourceBuilder::Fs(www_path)),
        share: Some(ResourceBuilder::Fs(share_path)),
    }
}

#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
fn check_app_integrity(
    app_content: &ApplicationPackageContent, app_package: &ApplicationPackage, manifest: &Manifest,
) {
    // check metadata JSON file
    let package_metadata = app_package.get_metadata().unwrap();
    assert_eq!(app_content.metadata, package_metadata);

    // check icon file
    assert!(app_package.get_icon_file().is_ok());

    // check www directory
    let www_dir = app_package.get_www_dir().unwrap();
    check_package_dir_integrity(&www_dir, &app_content.www);

    // check share directory
    let share_dir = app_package.get_share_dir().unwrap();
    check_package_dir_integrity(&share_dir, &app_content.share);

    // check WASM modules
    let modules = app_package.get_modules().unwrap();
    assert_eq!(modules.len(), app_content.modules.len());

    for module_info in modules {
        // taking not overridden module name
        let package_module_name = module_info.get_metadata().unwrap().get_name().unwrap();
        // searching by this name from the prepared app package files
        let (i, module_files) = app_content
            .modules
            .iter()
            .enumerate()
            .find(|(_, module)| module.module.metadata.get_name().unwrap() == *package_module_name)
            .unwrap();

        // taking overridden module name (optional)
        let manifest_module_name = manifest.modules[i].name.clone();
        assert_eq!(
            module_info.get_name(),
            manifest_module_name.unwrap_or(module_files.module.metadata.get_name().unwrap())
        );
        module_info.check_module_package_integrity(&module_files.module);

        // check overridden module config JSON file
        let config_info = module_info.get_config_info().unwrap().unwrap();
        assert_eq!(module_files.config, config_info.val.unwrap());

        // check overridden module share directory
        let share_dir = module_info.get_share_dir().unwrap();
        check_package_dir_integrity(&share_dir, &module_files.share);
    }
}

#[test]
#[allow(clippy::unwrap_used)]
fn from_dir_test() {
    let dir = TempDir::new().unwrap();

    let modules_num = 4;
    let mut app_package_content = prepare_default_package_content(modules_num);

    // override module names for first 2 modules
    let override_module_name = vec!["test_module_1".into(), "test_module_2".into()];
    let build_date = DateTime::default();
    let manifest = prepare_package_dir(
        "app".to_string(),
        &override_module_name,
        build_date,
        dir.path(),
        &mut app_package_content,
    );

    let package =
        ApplicationPackage::build_from_manifest(&manifest, dir.path(), None, build_date).unwrap();

    assert!(package.validate(true).is_ok());

    // Application package during the build process updates metadata file
    // to have a corresponded values update `app_package_content`.
    app_package_content.metadata.set_name(&manifest.name);
    app_package_content.metadata.set_build_date(build_date);

    // check app package integrity
    check_app_integrity(&app_package_content, &package, &manifest);
}

#[test]
#[allow(clippy::unwrap_used)]
fn author_sing_test() {
    let dir = TempDir::new().unwrap();

    let modules_num = 4;
    let mut app_package_content = prepare_default_package_content(modules_num);

    let build_date = DateTime::default();
    let manifest = prepare_package_dir(
        "app".to_string(),
        &[],
        build_date,
        dir.path(),
        &mut app_package_content,
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
}

#[allow(clippy::unwrap_used)]
fn author_sign_package(package: &ApplicationPackage) {
    let private_key = PrivateKey::from_str(&private_key_str()).unwrap();
    let certificate = Certificate::from_str(&certificate_str()).unwrap();

    // sign wasm modules packages first
    for module_info in package.get_modules().unwrap() {
        module_info.sign(&private_key, &certificate).unwrap();
    }

    package.author_sign(&private_key, &certificate).unwrap();

    certificate::storage::add_certificate(certificate).unwrap();
    assert!(package.validate(false).is_ok());
}

#[test]
#[allow(clippy::unwrap_used)]
fn corrupted_metadata_test() {
    let dir = TempDir::new().unwrap();

    let modules_num = 1;
    let mut app_package_content = prepare_default_package_content(modules_num);

    let build_date = DateTime::default();
    let manifest = prepare_package_dir(
        "app".to_string(),
        &[],
        build_date,
        dir.path(),
        &mut app_package_content,
    );

    let package =
        ApplicationPackage::build_from_manifest(&manifest, dir.path(), None, build_date).unwrap();

    author_sign_package(&package);

    {
        package
            .0
            .remove_file(ApplicationPackage::METADATA_FILE.into())
            .unwrap();
        assert!(
            package.validate(false).is_err(),
            "Missing required metadata file."
        );
    }

    {
        let new_metadata = Metadata::<ApplicationPackage>::from_reader(
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
        assert_ne!(app_package_content.metadata, new_metadata);

        package
            .0
            .copy_resource_file(
                &BytesResource::new(
                    ApplicationPackage::METADATA_FILE.to_string(),
                    new_metadata.to_bytes().unwrap(),
                ),
                ApplicationPackage::METADATA_FILE.into(),
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
fn corrupted_icon_test() {
    let dir = TempDir::new().unwrap();

    let modules_num = 1;
    let mut app_package_content = prepare_default_package_content(modules_num);

    let build_date = DateTime::default();
    let manifest = prepare_package_dir(
        "app".to_string(),
        &[],
        build_date,
        dir.path(),
        &mut app_package_content,
    );

    let package =
        ApplicationPackage::build_from_manifest(&manifest, dir.path(), None, build_date).unwrap();

    author_sign_package(&package);

    {
        package
            .0
            .remove_file(ApplicationPackage::ICON_FILE.into())
            .unwrap();
        assert!(
            package.validate(false).is_err(),
            "Missing required metadata file."
        );
    }

    {
        let new_icon = b"new icon_image_svg_content";
        assert_ne!(app_package_content.icon.as_slice(), new_icon);

        package
            .0
            .copy_resource_file(
                &BytesResource::new(ApplicationPackage::ICON_FILE.to_string(), new_icon.to_vec()),
                ApplicationPackage::ICON_FILE.into(),
            )
            .unwrap();

        assert!(package.get_icon_file().is_ok());
        assert!(
            package.validate(false).is_err(),
            "Corrupted signature payload."
        );
    }
}

#[test]
#[allow(clippy::unwrap_used)]
fn corrupted_share_dir_test() {
    let dir = TempDir::new().unwrap();

    let modules_num = 1;
    let mut app_package_content = prepare_default_package_content(modules_num);

    let build_date = DateTime::default();
    let manifest = prepare_package_dir(
        "app".to_string(),
        &[],
        build_date,
        dir.path(),
        &mut app_package_content,
    );

    let package =
        ApplicationPackage::build_from_manifest(&manifest, dir.path(), None, build_date).unwrap();

    author_sign_package(&package);

    {
        package
            .0
            .remove_dir(ApplicationPackage::SRV_SHARE_DIR.into())
            .unwrap();
        assert!(package.get_share_dir().is_none());
        assert!(
            package.validate(false).is_err(),
            "Corrupted signature payload."
        );
    }

    {
        let share_dir = package
            .0
            .create_dir(ApplicationPackage::SRV_SHARE_DIR.into())
            .unwrap();
        let new_file_name = "new_file";
        let new_file_content = b"new file content";
        let mut new_file = share_dir.create_file(new_file_name.into()).unwrap();
        new_file.write_all(new_file_content).unwrap();
        assert_ne!(app_package_content.share.file.0.as_str(), new_file_name);
        assert_ne!(
            app_package_content.share.file.1.as_slice(),
            new_file_content
        );

        assert!(package.get_share_dir().is_some());
        assert!(
            package.validate(false).is_err(),
            "Corrupted signature payload."
        );
    }
}

#[test]
#[allow(clippy::unwrap_used)]
fn corrupted_www_dir_test() {
    let dir = TempDir::new().unwrap();

    let modules_num = 1;
    let mut app_package_content = prepare_default_package_content(modules_num);

    let build_date = DateTime::default();
    let manifest = prepare_package_dir(
        "app".to_string(),
        &[],
        build_date,
        dir.path(),
        &mut app_package_content,
    );

    let package =
        ApplicationPackage::build_from_manifest(&manifest, dir.path(), None, build_date).unwrap();

    author_sign_package(&package);

    {
        package
            .0
            .remove_dir(ApplicationPackage::SRV_WWW_DIR.into())
            .unwrap();
        assert!(package.get_www_dir().is_none());
        assert!(
            package.validate(false).is_err(),
            "Corrupted signature payload."
        );
    }

    {
        let www_dir = package
            .0
            .create_dir(ApplicationPackage::SRV_WWW_DIR.into())
            .unwrap();
        let new_file_name = "new_file";
        let new_file_content = b"new file content";
        let mut new_file = www_dir.create_file(new_file_name.into()).unwrap();
        new_file.write_all(new_file_content).unwrap();
        assert_ne!(app_package_content.www.file.0.as_str(), new_file_name);
        assert_ne!(app_package_content.www.file.1.as_slice(), new_file_content);

        assert!(package.get_www_dir().is_some());
        assert!(
            package.validate(false).is_err(),
            "Corrupted signature payload."
        );
    }
}

#[test]
#[allow(clippy::unwrap_used)]
fn corrupted_module_config_test() {
    let dir = TempDir::new().unwrap();

    let modules_num = 1;
    let mut app_package_content = prepare_default_package_content(modules_num);

    let build_date = DateTime::default();
    let manifest = prepare_package_dir(
        "app".to_string(),
        &[],
        build_date,
        dir.path(),
        &mut app_package_content,
    );

    let package =
        ApplicationPackage::build_from_manifest(&manifest, dir.path(), None, build_date).unwrap();

    author_sign_package(&package);

    let modules = package.get_modules().unwrap();
    assert_eq!(modules.len(), 1);
    let module_info = modules.first().unwrap();

    {
        package
            .0
            .remove_file(
                format!(
                    "{}/{}/{}",
                    ApplicationPackage::USR_LIB_DIR,
                    module_info.get_name(),
                    ApplicationPackage::MODULE_CONFIG_FILE
                )
                .into(),
            )
            .unwrap();
        assert!(
            package.validate(false).is_err(),
            "Corrupted signature payload."
        );
    }

    {
        let config_info = module_info.get_config_info().unwrap().unwrap();

        let new_config = Config::from_reader(
            serde_json::json!({
                "new_prop": "new value",
            })
            .to_string()
            .as_bytes(),
            config_info.schema.validator(),
        )
        .unwrap();
        assert_ne!(
            app_package_content.modules.first().unwrap().config,
            new_config
        );

        assert!(
            package.validate(false).is_err(),
            "Corrupted signature payload."
        );
    }
}

#[test]
#[allow(clippy::unwrap_used)]
fn corrupted_module_share_dir_test() {
    let dir = TempDir::new().unwrap();

    let modules_num = 1;
    let mut app_package_content = prepare_default_package_content(modules_num);

    let build_date = DateTime::default();
    let manifest = prepare_package_dir(
        "app".to_string(),
        &[],
        build_date,
        dir.path(),
        &mut app_package_content,
    );

    let package =
        ApplicationPackage::build_from_manifest(&manifest, dir.path(), None, build_date).unwrap();

    author_sign_package(&package);

    let modules = package.get_modules().unwrap();
    assert_eq!(modules.len(), 1);
    let module_info = modules.first().unwrap();

    {
        package
            .0
            .remove_dir(
                format!(
                    "{}/{}/{}",
                    ApplicationPackage::USR_LIB_DIR,
                    module_info.get_name(),
                    ApplicationPackage::MODULE_SHARE_DIR
                )
                .into(),
            )
            .unwrap();
        assert!(
            package.validate(false).is_err(),
            "Corrupted signature payload."
        );
    }

    {
        let share_dir = package
            .0
            .create_dir(
                format!(
                    "{}/{}/{}",
                    ApplicationPackage::USR_LIB_DIR,
                    module_info.get_name(),
                    ApplicationPackage::MODULE_SHARE_DIR
                )
                .into(),
            )
            .unwrap();
        let new_file_name = "new_file";
        let new_file_content = b"new file content";
        let mut new_file = share_dir.create_file(new_file_name.into()).unwrap();
        new_file.write_all(new_file_content).unwrap();
        assert_ne!(app_package_content.www.file.0.as_str(), new_file_name);
        assert_ne!(app_package_content.www.file.1.as_slice(), new_file_content);

        assert!(
            package.validate(false).is_err(),
            "Corrupted signature payload."
        );
    }
}
