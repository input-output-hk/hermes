//! Tests inside this folder should be executed sequentially,
//! since multiple instances of Hermes app would conflict
//! causing test failures during execution because
//! they require same resources to be locked.

mod failed_module_init;
mod http_request_rte;
mod parallel_module_execution;
