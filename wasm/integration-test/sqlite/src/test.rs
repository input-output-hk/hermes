use crate::hermes::sqlite;

type TestResult = Result<(), sqlite::api::Errno>;

pub(crate) struct TestItem {
    pub(crate) name: &'static str,
    pub(crate) executor: fn() -> TestResult,
}

pub(crate) const TESTS: &[TestItem] = &[
    TestItem {
        name: "open-database-persistent-simple",
        executor: item::open_database_persistent_simple,
    },
    TestItem {
        name: "open-database-persistent-multiple",
        executor: item::open_database_persistent_multiple,
    },
    TestItem {
        name: "open-database-persistent-multiple-alt",
        executor: item::open_database_persistent_multiple_alt,
    },
    TestItem {
        name: "open-database-memory-simple",
        executor: item::open_database_memory_simple,
    },
    TestItem {
        name: "execute-create-schema-simple",
        executor: item::execute_create_schema_simple,
    },
    TestItem {
        name: "prepare-simple",
        executor: item::prepare_simple,
    },
    TestItem {
        name: "prepare-simple-without-cleaning",
        executor: item::prepare_simple_without_cleaning,
    },
    TestItem {
        name: "text-value-simple",
        executor: item::text_value_simple,
    },
];

pub(crate) const BENCHES: &[TestItem] = &[
    TestItem {
        // FIXME: right now, according to the config, still sharing the same file. If you need to add a new case for a bench related to a file, you need to clean up the old one.
        name: "bench-persistent-insert",
        executor: || helper::bench_insert(false),
    },
    TestItem {
        name: "bench-memory-insert",
        executor: || helper::bench_insert(true),
    },
];

mod item {
    use super::{TestResult, sqlite};

    pub(super) fn open_database_persistent_simple() -> TestResult {
        let sqlite = sqlite::api::open(false, false)?;

        sqlite.close()
    }

    pub(super) fn open_database_persistent_multiple() -> TestResult {
        let sqlite_a = sqlite::api::open(false, false)?;
        let sqlite_b = sqlite::api::open(false, false)?;
        let sqlite_c = sqlite::api::open(false, false)?;

        sqlite_a.close()?;
        sqlite_b.close()?;
        sqlite_c.close()
    }

    pub(super) fn open_database_persistent_multiple_alt() -> TestResult {
        let sqlite_a = sqlite::api::open(false, false)?;
        sqlite_a.close()?;

        let sqlite_b = sqlite::api::open(false, false)?;
        sqlite_b.close()?;

        let sqlite_c = sqlite::api::open(false, false)?;
        sqlite_c.close()
    }

    pub(super) fn open_database_memory_simple() -> TestResult {
        let sqlite = sqlite::api::open(false, true)?;
        sqlite.close()
    }

    pub(super) fn execute_create_schema_simple() -> TestResult {
        let sqlite = sqlite::api::open(false, true)?;

        let create_table_sql = r"
            CREATE TABLE IF NOT EXISTS people (
                id INTEGER PRIMARY KEY,
                name TEXT,
                age INTEGER
            );
        ";

        sqlite.execute(create_table_sql)?;
        sqlite.close()
    }

    pub(super) fn prepare_simple() -> TestResult {
        let sqlite = sqlite::api::open(false, true)?;
        let stmt = sqlite.prepare("SELECT 1;")?;

        stmt.finalize()?;
        sqlite.close()
    }

    pub(super) fn prepare_simple_without_cleaning() -> TestResult {
        let sqlite = sqlite::api::open(false, true)?;
        sqlite.prepare("SELECT 1;")?;

        Ok(())
    }

    pub(super) fn text_value_simple() -> TestResult {
        let sqlite = sqlite::api::open(false, true)?;

        // prepare and insert value
        let create_table_sql = r"
            CREATE TABLE dummy(id INTEGER PRIMARY KEY, value TEXT);
        ";
        let insert_sql = "INSERT INTO dummy(value) VALUES(?);";

        let value = sqlite::api::Value::Text(String::from("Hello, World!"));

        sqlite.execute(create_table_sql)?;

        let stmt = sqlite.prepare(insert_sql)?;
        stmt.bind(1, &value)?;
        stmt.step()?;
        stmt.finalize()?;

        // retrieve value
        let retrieve_sql = "SELECT value FROM dummy WHERE id = 1;";

        let stmt = sqlite.prepare(retrieve_sql)?;
        stmt.step()?;
        let retrieved_value = stmt.column(0)?;
        stmt.finalize()?;

        sqlite.close()?;

        match (value, retrieved_value) {
            (sqlite::api::Value::Text(a), sqlite::api::Value::Text(b)) if a == b => Ok(()),
            _ => Err(sqlite::api::Errno::Sqlite(1)),
        }
    }
}

mod helper {
    use super::{TestResult, sqlite};

    pub(super) fn bench_insert(memory: bool) -> TestResult {
        let sqlite = sqlite::api::open(false, memory)?;

        let create_table_sql = r"
          CREATE TABLE dummy(id INTEGER PRIMARY KEY, value TEXT);
        ";
        let insert_sql = "INSERT INTO dummy(value) VALUES(?);";

        let value = sqlite::api::Value::Text(String::from("Hello, World!"));

        sqlite.execute(create_table_sql)?;

        for _ in 0..100 {
            let stmt = sqlite.prepare(insert_sql)?;
            stmt.bind(1, &value)?;
            stmt.step()?;
            stmt.finalize()?;
        }

        sqlite.close()
    }
}
