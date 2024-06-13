use crate::hermes::hermes::sqlite::{
  self,
  api::Errno
};

pub(crate) struct TestItem {
    pub(crate) name: &'static str,
    pub(crate) executor: fn() -> Result<(), Errno>,
}

pub(crate) const TESTS: &[TestItem] = &[
    TestItem {
        name: "open-database-persistent-simple",
        executor: || {
            let sqlite = sqlite::api::open(false, false)?;

            sqlite.close()
        },
    },
    TestItem {
        name: "open-database-persistent-multiple",
        executor: || {
            let sqlite_a = sqlite::api::open(false, false)?;
            let sqlite_b = sqlite::api::open(false, false)?;
            let sqlite_c = sqlite::api::open(false, false)?;

            sqlite_a.close()?;
            sqlite_b.close()?;
            sqlite_c.close()
        },
    },
    TestItem {
        name: "open-database-persistent-multiple-alt",
        executor: || {
            let sqlite_a = sqlite::api::open(false, false)?;
            sqlite_a.close()?;

            let sqlite_b = sqlite::api::open(false, false)?;
            sqlite_b.close()?;

            let sqlite_c = sqlite::api::open(false, false)?;
            sqlite_c.close()
        },
    },
    TestItem {
        name: "open-database-memory-simple",
        executor: || {
            let sqlite = sqlite::api::open(false, true)?;
            sqlite.close()
        },
    },
    TestItem {
        name: "execute-create-schema-simple",
        executor: || {
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
        },
    },
    TestItem {
        name: "prepare-simple",
        executor: || {
            let sqlite = sqlite::api::open(false, true)?;
            let stmt = sqlite.prepare("SELECT 1;")?;

            stmt.finalize()?;
            sqlite.close()
        },
    },
    TestItem {
        name: "prepare-simple-without-cleaning",
        executor: || {
            let sqlite = sqlite::api::open(false, true)?;
            sqlite.prepare("SELECT 1;")?;

            Ok(())
        },
    },
    TestItem {
        name: "text-value-simple",
        executor: || {
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

            if matches!((value, retrieved_value), (sqlite::api::Value::Text(a), sqlite::api::Value::Text(b)) if a == b) {
              Ok(())
            } else {
              Err(Errno::Sqlite(1))
            }
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

    pub(super) fn bench_insert(memory: bool) -> Result<(), sqlite::api::Errno> {
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
