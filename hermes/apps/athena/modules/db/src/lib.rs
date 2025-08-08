//! # DB Component.

#[allow(clippy::all, unused)]
mod hermes;
mod stub;

/// Simple HTTP proxy component for demonstration purposes.
struct DbComponent;

fn log(
    level: hermes::hermes::logging::api::Level,
    message: &str,
) {
    let message = format!("DB Component: {}", message);
    hermes::hermes::logging::api::log(level, None, None, None, None, None, &message, None);
}

impl hermes::exports::hermes::init::event::Guest for DbComponent {
    fn init() -> bool {
        log(
            hermes::hermes::logging::api::Level::Info,
            "Initializing DB component...",
        );

        let sqlite = match hermes::hermes::sqlite::api::open(false, false) {
            Ok(sqlite) => sqlite,
            Err(e) => {
                log(
                    hermes::hermes::logging::api::Level::Error,
                    &format!("Failed to open database: {}", e),
                );
                return false;
            },
        };

        log(
            hermes::hermes::logging::api::Level::Info,
            "DB opened or created, creating table...",
        );
        let create_table_sql = r"
            CREATE TABLE IF NOT EXISTS people (
                id INTEGER PRIMARY KEY,
                name TEXT,
                age INTEGER
            );
        ";
        if let Err(e) = sqlite.execute(create_table_sql) {
            log(hermes::hermes::logging::api::Level::Error, e.to_string().as_str());
        }

        log(
            hermes::hermes::logging::api::Level::Info,
            "Putting data into table...",
        );
        let insert_sql = r#"
            INSERT INTO people (name, age) VALUES ('Athena', 1);
        "#;
        if let Err(e) = sqlite.execute(insert_sql) {
            log(hermes::hermes::logging::api::Level::Error, e.to_string().as_str());
        }

        log(
            hermes::hermes::logging::api::Level::Info,
            "Done and dusted!",
        );
        true
    }
}

hermes::export!(DbComponent with_types_in hermes);
