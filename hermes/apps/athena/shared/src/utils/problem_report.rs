//! Problem report utils.

pub use catalyst_types::problem_report::ProblemReport;

/// Converts problem report to JSON.
#[must_use]
pub fn problem_report_to_json(report: &ProblemReport) -> Option<String> {
    if !report.is_problematic() {
        return None;
    }

    let entries: Vec<_> = report
        .entries()
        .map(|elem| {
            elem.map(|entry| {
                let mut obj = serde_json::json!({
                    "msg": entry.context(),
                });

                if let Some(map) = obj.as_object_mut() {
                    match entry.kind() {
                        catalyst_types::problem_report::Kind::MissingField { field } => {
                            map.insert("kind".to_string(), serde_json::json!("MissingField"));
                            map.insert("field".to_string(), serde_json::json!(field));
                        },
                        catalyst_types::problem_report::Kind::UnknownField { field, value } => {
                            map.insert("kind".to_string(), serde_json::json!("UnknownField"));
                            map.insert("field".to_string(), serde_json::json!(field));
                            map.insert("value".to_string(), serde_json::json!(value));
                        },
                        catalyst_types::problem_report::Kind::InvalidValue {
                            field,
                            value,
                            constraint,
                        } => {
                            map.insert("kind".to_string(), serde_json::json!("InvalidValue"));
                            map.insert("field".to_string(), serde_json::json!(field));
                            map.insert("value".to_string(), serde_json::json!(value));
                            map.insert("constraint".to_string(), serde_json::json!(constraint));
                        },
                        catalyst_types::problem_report::Kind::InvalidEncoding {
                            field,
                            encoded,
                            expected,
                        } => {
                            map.insert("kind".to_string(), serde_json::json!("InvalidEncoding"));
                            map.insert("field".to_string(), serde_json::json!(field));
                            map.insert("encoded".to_string(), serde_json::json!(encoded));
                            map.insert("expected".to_string(), serde_json::json!(expected));
                        },
                        catalyst_types::problem_report::Kind::FunctionalValidation {
                            explanation,
                        } => {
                            map.insert(
                                "kind".to_string(),
                                serde_json::json!("FunctionalValidation"),
                            );
                            map.insert("explanation".to_string(), serde_json::json!(explanation));
                        },
                        catalyst_types::problem_report::Kind::DuplicateField {
                            field,
                            description,
                        } => {
                            map.insert("kind".to_string(), serde_json::json!("DuplicateField"));
                            map.insert("field".to_string(), serde_json::json!(field));
                            map.insert("description".to_string(), serde_json::json!(description));
                        },
                        catalyst_types::problem_report::Kind::ConversionError {
                            field,
                            value,
                            expected_type,
                        } => {
                            map.insert("kind".to_string(), serde_json::json!("ConversionError"));
                            map.insert("field".to_string(), serde_json::json!(field));
                            map.insert("value".to_string(), serde_json::json!(value));
                            map.insert(
                                "expected_type".to_string(),
                                serde_json::json!(expected_type),
                            );
                        },
                        catalyst_types::problem_report::Kind::Other { description } => {
                            map.insert("kind".to_string(), serde_json::json!("Other"));
                            map.insert("description".to_string(), serde_json::json!(description));
                        },
                    }
                }

                obj
            })
        })
        .collect();

    let report_json = serde_json::json!({
        "context": report.context(),
        "entries": entries
    });

    serde_json::to_string(&report_json).ok()
}
