//! Integration test runtime extension event handler implementation for test purpose only.

#![allow(clippy::module_name_repetitions)]

use anyhow::Ok;
use crossbeam_queue::SegQueue;
use once_cell::sync::OnceCell;

use crate::{
    event_queue::event::HermesEventPayload,
    runtime_extensions::bindings::exports::hermes::integration_test::event::TestResult,
    wasm::module::Module,
};

/// Storing results from call test.
static TEST_RESULT_QUEUE: OnceCell<SegQueue<Option<TestResult>>> = OnceCell::new();
/// Storing results from call bench.
static BENCH_RESULT_QUEUE: OnceCell<SegQueue<Option<TestResult>>> = OnceCell::new();

/// On test event
pub struct OnTestEvent {
    /// The bench number to run/list.
    pub test: u32,
    /// True = Run the test, False = Just list the test name.
    pub run: bool,
}

impl HermesEventPayload for OnTestEvent {
    fn event_name(&self) -> &str {
        "test"
    }

    fn execute(&self, module: &mut crate::wasm::module::ModuleInstance) -> anyhow::Result<()> {
        let result: Option<TestResult> = module
            .instance
            .hermes_integration_test_event()
            .call_test(&mut module.store, self.test, self.run)?;
        TEST_RESULT_QUEUE.get_or_init(SegQueue::new).push(result);
        Ok(())
    }
}

/// On bench event
pub struct OnBenchEvent {
    /// The bench number to run/list.
    pub test: u32,
    /// True = Run the benchmark, False = Just list the test name.
    pub run: bool,
}

impl HermesEventPayload for OnBenchEvent {
    fn event_name(&self) -> &str {
        "bench"
    }

    fn execute(&self, module: &mut crate::wasm::module::ModuleInstance) -> anyhow::Result<()> {
        let result: Option<TestResult> = module
            .instance
            .hermes_integration_test_event()
            .call_bench(&mut module.store, self.test, self.run)?;
        BENCH_RESULT_QUEUE.get_or_init(SegQueue::new).push(result);
        Ok(())
    }
}

/// Executes an event from a module and returns a testing result.
///
/// # Errors
///
/// Fails to execute an event.
#[allow(dead_code)]
pub fn execute_event(
    module: &mut Module, test: u32, run: bool, bench: bool,
) -> anyhow::Result<Option<TestResult>> {
    let result = if bench {
        let on_bench_event = OnBenchEvent { test, run };
        module.execute_event(&on_bench_event)?;
        BENCH_RESULT_QUEUE.get_or_init(SegQueue::new).pop()
    } else {
        let on_test_event = OnTestEvent { test, run };
        module.execute_event(&on_test_event)?;
        TEST_RESULT_QUEUE.get_or_init(SegQueue::new).pop()
    };

    Ok(result.flatten())
}
