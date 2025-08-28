use crate::{
    database::{data::RbacDbData, SQLITE},
    hermes,
    utils::log::{log_error, log_info},
};
// FIXME: cleanup
const FILE_NAME: &str = "rbac-registration/src/database/select.rs";

pub(crate) const RBAC_SELECT_DATA: &str = r#"
    SELECT * FROM rbac_registration;
"#;

pub(crate) fn select_rbac_registration() {
    const FUNCTION_NAME: &str = "select_rbac_registration";

    let mut data = vec![];

    let stmt = match SQLITE.prepare(RBAC_SELECT_DATA) {
        Ok(stmt) => stmt,
        Err(e) => {
            log_error(
                FILE_NAME,
                FUNCTION_NAME,
                "hermes::sqlite::api::prepare",
                &format!("Failed to prepare select statement: {e}"),
                None,
            );
            return;
        },
    };

    loop {
        match stmt.step() {
            Ok(hermes::hermes::sqlite::api::StepResult::Row) => {
                data.push(stmt.column(0));
                log_info(
                    FILE_NAME,
                    FUNCTION_NAME,
                    "hermes::sqlite::api::step",
                    &format!("🦄 Row {data:?}"),
                    None,
                );
            },
            Ok(hermes::hermes::sqlite::api::StepResult::Done) => {
                log_info(FILE_NAME, "", &format!("🦄 Done"), "", None);
                break;
            },
            Err(e) => {
                log_error(
                    FILE_NAME,
                    FUNCTION_NAME,
                    "hermes::sqlite::api::step",
                    &format!("Failed to step: {e}"),
                    None,
                );
            },
        }
    }
    stmt.finalize();
}
