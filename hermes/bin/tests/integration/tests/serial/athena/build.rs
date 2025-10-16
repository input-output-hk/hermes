use std::path::Path;

use temp_dir::TempDir;

use crate::utils;

#[allow(unused)]
fn build_athena() -> anyhow::Result<String> {
    let temp_dir = TempDir::new()?;
    let manifest_dir_path = Path::new(env!("CARGO_MANIFEST_DIR"));
    let hermes_root = manifest_dir_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("could not find parent directory for bin"))?;
    let athena_modules_path_buf = hermes_root.join("apps/athena/modules");
    let athena_modules_path = athena_modules_path_buf.as_path();

    let components = std::fs::read_dir(athena_modules_path)?
        .filter_map(|read_dir| {
            let file_or_folder = read_dir.ok()?;
            if file_or_folder.path().is_dir() {
                let canonical_path = file_or_folder.path().as_path().canonicalize().ok()?;
                let folder_name = canonical_path.file_name()?;
                Some(format!("{}", folder_name.display()))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    for component in &components {
        println!("building {component} component");
        utils::component::build_at_path(athena_modules_path, component, &temp_dir)
            .map_err(|err| anyhow::anyhow!("failed to build {component} component: {err}"))?;
    }

    let components = components
        .into_iter()
        .map(|component| component.replace('-', "_"))
        .collect::<Vec<_>>();

    for component in &components {
        let module = format!("{component}_module");
        println!("packaging {module} module for component {component}");
        utils::packaging::package_module(&temp_dir, component, &module)
            .map_err(|err| anyhow::anyhow!("failed to package {module} module: {err}"))?;
    }
    let modules = components
        .into_iter()
        .map(|component| format!("{component}_module"))
        .collect();

    println!("packaging Athena app");
    let app_file_name = utils::packaging::package_app_with_modules(&temp_dir, Some(modules))
        .map_err(|err| anyhow::anyhow!("failed to package athena app: {err}"))?;

    Ok(app_file_name)
}
