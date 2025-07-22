//! Application builder from the application package.

use super::ApplicationPackage;
use crate::{
    app::Application,
    vfs::{PermissionLevel, Vfs, VfsBootstrapper},
};

/// Build application from the application package.
pub(crate) fn build_app<P: AsRef<std::path::Path>>(
    package: &ApplicationPackage, vfs_dir_path: P,
) -> anyhow::Result<Application> {
    let app_name = package.get_app_name()?;
    let mut bootstrapper = VfsBootstrapper::new(vfs_dir_path, app_name.clone());
    mount_to_vfs(package, &mut bootstrapper)?;
    let vfs = bootstrapper.bootstrap()?;

    let mut modules = Vec::new();
    for module_info in package.get_modules()? {
        let module = module_info.get_component()?;
        modules.push(module);
    }
    let app = Application::new(app_name, vfs, modules);

    Ok(app)
}

/// Mount `ApplicationPackage` content to the `Vfs`
fn mount_to_vfs(
    package: &ApplicationPackage, bootstrapper: &mut VfsBootstrapper,
) -> anyhow::Result<()> {
    let root_path = "/".to_string();
    bootstrapper.with_mounted_file(
        root_path.clone(),
        package.get_icon_file()?,
        PermissionLevel::Read,
    );
    bootstrapper.with_mounted_file(
        root_path.clone(),
        package.get_metadata_file()?,
        PermissionLevel::Read,
    );
    if let Some(share_dir) = package.get_share_dir() {
        bootstrapper.with_mounted_dir(root_path.clone(), share_dir, PermissionLevel::Read);
    }
    if let Some(www_dir) = package.get_www_dir() {
        bootstrapper.with_mounted_dir(root_path, www_dir, PermissionLevel::Read);
    }

    for module_info in package.get_modules()? {
        let lib_module_dir_path = format!("{}/{}", Vfs::LIB_DIR, module_info.get_name());
        bootstrapper.with_dir_to_create(lib_module_dir_path.clone(), PermissionLevel::Read);

        bootstrapper.with_mounted_file(
            lib_module_dir_path.clone(),
            module_info.get_metadata_file()?,
            PermissionLevel::Read,
        );
        bootstrapper.with_mounted_file(
            lib_module_dir_path.clone(),
            module_info.get_component_file()?,
            PermissionLevel::Read,
        );
        if let Some(config_schema) = module_info.get_config_schema_file() {
            bootstrapper.with_mounted_file(
                lib_module_dir_path.clone(),
                config_schema,
                PermissionLevel::Read,
            );
        }
        if let Some(config) = module_info.get_config_file() {
            bootstrapper.with_mounted_file(
                lib_module_dir_path.clone(),
                config,
                PermissionLevel::Read,
            );
        }
        if let Some(settings_schema) = module_info.get_settings_schema_file() {
            bootstrapper.with_mounted_file(
                lib_module_dir_path.clone(),
                settings_schema,
                PermissionLevel::Read,
            );
        }
        if let Some(share_dir) = module_info.get_share_dir() {
            bootstrapper.with_mounted_dir(lib_module_dir_path, share_dir, PermissionLevel::Read);
        }
    }
    Ok(())
}
