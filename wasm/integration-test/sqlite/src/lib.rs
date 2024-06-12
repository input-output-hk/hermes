//! Hermes SQLite module integration test with WASM runtime.
//! Generate `hermes.rs` with `earthly +gen-bindings` before writing the test.

// Allow everything since this is generated code.
#[allow(clippy::all, unused)]
mod hermes;

use hermes::{
    exports::hermes::integration_test::event::TestResult,
    hermes::{
        cardano::api::{BlockSrc, CardanoBlock, CardanoBlockchainId, CardanoTxn},
        cron::api::CronTagged,
        kv_store::api::KvValues,
        sqlite,
    },
    wasi::http::types::{IncomingRequest, ResponseOutparam},
};

struct TestItem {
    name: &'static str,
    executor: fn() -> bool,
}

const TESTS: &'static [TestItem] = &[
    TestItem {
        name: "open-database-persistent-simple",
        executor: || {
            let Ok(sqlite) = sqlite::api::open(false, false) else {
                return false;
            };

            let result = sqlite.close();

            result.is_ok()
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

            let result_a = sqlite_a.close();
            let result_b = sqlite_b.close();
            let result_c = sqlite_c.close();

            result_a.is_ok() && result_b.is_ok() && result_c.is_ok()
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

const BENCHES: &'static [TestItem] = &[
    TestItem {
        // FIXME: right now, according to the config, still sharing the same file. If you need to add a new case for bench related to a file, you need to clean up the old one.
        name: "bench-persistent-insert",
        executor: || { helper::bench_insert(false) },
    },
    TestItem {
        name: "bench-memory-insert",
        executor: || { helper::bench_insert(true) },
    },
];

struct TestComponent;

impl hermes::exports::hermes::integration_test::event::Guest for TestComponent {
    fn test(test: u32, run: bool) -> Option<TestResult> {
        TESTS.get(test as usize).map(|item| TestResult {
            name: String::from(item.name),
            status: {
                if run {
                    (item.executor)()
                } else {
                    true
                }
            },
        })
    }

    fn bench(test: u32, run: bool) -> Option<TestResult> {
        BENCHES.get(test as usize).map(|item| TestResult {
            name: String::from(item.name),
            status: {
                if run {
                    (item.executor)()
                } else {
                    true
                }
            },
        })
    }
}

impl hermes::exports::hermes::cardano::event_on_block::Guest for TestComponent {
    fn on_cardano_block(_blockchain: CardanoBlockchainId, _block: CardanoBlock, _source: BlockSrc) {
    }
}

impl hermes::exports::hermes::cardano::event_on_rollback::Guest for TestComponent {
    fn on_cardano_rollback(_blockchain: CardanoBlockchainId, _slot: u64) {}
}

impl hermes::exports::hermes::cardano::event_on_txn::Guest for TestComponent {
    fn on_cardano_txn(
        _blockchain: CardanoBlockchainId,
        _slot: u64,
        _txn_index: u32,
        _txn: CardanoTxn,
    ) {
    }
}

impl hermes::exports::hermes::cron::event::Guest for TestComponent {
    fn on_cron(_event: CronTagged, _last: bool) -> bool {
        false
    }
}

impl hermes::exports::hermes::init::event::Guest for TestComponent {
    fn init() -> bool {
        true
    }
}

impl hermes::exports::hermes::kv_store::event::Guest for TestComponent {
    fn kv_update(_key: String, _value: KvValues) {}
}

impl hermes::exports::wasi::http::incoming_handler::Guest for TestComponent {
    fn handle(_request: IncomingRequest, _response_out: ResponseOutparam) {}
}

hermes::export!(TestComponent with_types_in hermes);

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
