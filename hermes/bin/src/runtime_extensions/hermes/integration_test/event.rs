//! Integration test runtime extension event handler implementation for test purpose only.

use crate::{event_queue::event::HermesEventPayload, runtime_extensions::bindings::exports::hermes::integration_test::event::TestResult};

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
      module.instance.hermes_integration_test_event().call_test(
          &mut module.store,
          self.test,
          self.run,
      )?;
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
      let a: Option<TestResult> = module.instance.hermes_integration_test_event().call_bench(
          &mut module.store,
          self.test,
          self.run,
      )?;
      dbg!(a);
      Ok(())
  }
}
