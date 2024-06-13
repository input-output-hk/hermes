use crate::hermes::hermes::sqlite;

pub(crate) struct TestItem {
    pub(crate) name: &'static str,
    pub(crate) executor: fn() -> bool,
}

pub(crate) const TESTS: &[TestItem] = &[
    TestItem {
        name: "open-database-persistent-simple",
        executor: || {
            let Ok(sqlite) = sqlite::api::open(false, false) else {
                return false;
            };

            sqlite.close().is_ok()
        },
    },
    TestItem {
        name: "open-database-persistent-multiple",
        executor: || {
            let Ok(sqlite_a) = sqlite::api::open(false, false) else {
                return false;
            };
            let Ok(sqlite_b) = sqlite::api::open(false, false) else {
                return false;
            };
            let Ok(sqlite_c) = sqlite::api::open(false, false) else {
                return false;
            };

            sqlite_a.close().is_ok() && sqlite_b.close().is_ok() && sqlite_c.close().is_ok()
        },
    },
    TestItem {
        name: "open-database-persistent-multiple-alt",
        executor: || {
            let Ok(sqlite_a) = sqlite::api::open(false, false) else {
                return false;
            };
            let result_a = sqlite_a.close();

            let Ok(sqlite_b) = sqlite::api::open(false, false) else {
                return false;
            };
            let result_b = sqlite_b.close();

            let Ok(sqlite_c) = sqlite::api::open(false, false) else {
                return false;
            };
            let result_c = sqlite_c.close();

            result_a.is_ok() && result_b.is_ok() && result_c.is_ok()
        },
    },
    TestItem {
        name: "open-database-memory-simple",
        executor: || {
            let Ok(sqlite) = sqlite::api::open(false, true) else {
                return false;
            };
            let result = sqlite.close();

            result.is_ok()
        },
    },
    TestItem {
        name: "execute-create-schema-simple",
        executor: || {
            let Ok(sqlite) = sqlite::api::open(false, true) else {
                return false;
            };

            let create_table_sql = r"
              CREATE TABLE IF NOT EXISTS people (
                  id INTEGER PRIMARY KEY,
                  name TEXT,
                  age INTEGER
              );
          ";

            let Ok(()) = sqlite.execute(create_table_sql) else {
                return false;
            };

            let result = sqlite.close();

            result.is_ok()
        },
    },
    TestItem {
        name: "prepare-simple",
        executor: || {
            let Ok(sqlite) = sqlite::api::open(false, true) else {
                return false;
            };

            let Ok(stmt) = sqlite.prepare("SELECT 1;") else {
                return false;
            };

            let finalize_result = stmt.finalize();
            let close_result = sqlite.close();

            finalize_result.is_ok() && close_result.is_ok()
        },
    },
    TestItem {
        name: "prepare-simple-without-cleaning",
        executor: || {
            let Ok(sqlite) = sqlite::api::open(false, true) else {
                return false;
            };

            let Ok(_) = sqlite.prepare("SELECT 1;") else {
                return false;
            };

            true
        },
    },
    TestItem {
        name: "text-value-simple",
        executor: || {
            let Ok(sqlite) = sqlite::api::open(false, true) else {
                return false;
            };

            // prepare and insert value
            let create_table_sql = r"
              CREATE TABLE dummy(id INTEGER PRIMARY KEY, value TEXT);
          ";
            let insert_sql = "INSERT INTO dummy(value) VALUES(?);";

            let value = sqlite::api::Value::Text(String::from("Hello, World!"));

            let Ok(()) = sqlite.execute(create_table_sql) else {
                return false;
            };
            let Ok(stmt) = sqlite.prepare(insert_sql) else {
                return false;
            };
            let Ok(()) = stmt.bind(1, &value) else {
                return false;
            };
            let Ok(()) = stmt.step() else {
                return false;
            };
            let Ok(()) = stmt.finalize() else {
                return false;
            };

            // retrieve value
            let retrieve_sql = "SELECT value FROM dummy WHERE id = 1;";

            let Ok(stmt) = sqlite.prepare(retrieve_sql) else {
                return false;
            };
            let Ok(()) = stmt.step() else {
                return false;
            };
            let Ok(retrieved_value) = stmt.column(0) else {
                return false;
            };
            let Ok(()) = stmt.finalize() else {
                return false;
            };

            let Ok(()) = sqlite.close() else {
                return false;
            };

            matches!((value, retrieved_value), (sqlite::api::Value::Text(a), sqlite::api::Value::Text(b)) if a == b)
        },
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

mod helper {
    use crate::sqlite;

    pub(super) fn bench_insert(memory: bool) -> bool {
        let Ok(sqlite) = sqlite::api::open(false, memory) else {
            return false;
        };

        let create_table_sql = r"
          CREATE TABLE dummy(id INTEGER PRIMARY KEY, value TEXT);
      ";
        let insert_sql = "INSERT INTO dummy(value) VALUES(?);";

        let value = sqlite::api::Value::Text(String::from("Hello, World!"));

        let Ok(()) = sqlite.execute(create_table_sql) else {
            return false;
        };

        for _ in 0..100 {
            let Ok(stmt) = sqlite.prepare(insert_sql) else {
                return false;
            };
            let Ok(()) = stmt.bind(1, &value) else {
                return false;
            };
            let Ok(()) = stmt.step() else {
                return false;
            };
            let Ok(()) = stmt.finalize() else {
                return false;
            };
        }

        sqlite.close().is_ok()
    }
}
