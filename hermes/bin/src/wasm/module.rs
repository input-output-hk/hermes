//! WASM module implementation.
//! Wrapper over the `wasmtime::Module`, `wasmtime::Instance` etc. structs which
//! define a WASM module abstraction with capability to interact with it.
//!
//! All implementation based on [wasmtime](https://crates.io/crates/wasmtime) crate dependency.

use std::sync::atomic::{AtomicU32, Ordering};

use rusty_ulid::Ulid;
use wasmtime::{
    component::{Component as WasmModule, InstancePre as WasmInstancePre, Linker as WasmLinker},
    Store as WasmStore,
};

use crate::{
    event::HermesEventPayload, runtime_extensions::bindings, state::HermesRuntimeState,
    wasm::engine::Engine,
};

/// Bad WASM module error
#[derive(thiserror::Error, Debug)]
#[error("Bad WASM module, err: {0}")]
struct BadWASMModuleError(String);

/// Structure defines an abstraction over the WASM module instance.
/// It holds the state of the WASM module along with its context data.
/// It is used to interact with the WASM module.
pub(crate) struct ModuleInstance {
    /// `wasmtime::Store` entity
    pub(crate) store: WasmStore<HermesRuntimeState>,
    /// `Instance` entity
    pub(crate) instance: bindings::Hermes,
}

/// Module id type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ModuleId(pub(crate) Ulid);

/// Structure defines an abstraction over the WASM module
/// It instantiates the module with the provided context data,
/// links all provided imports to the module instance,
/// handles an internal state of the WASM module.
///
/// The primary goal for it is to make a WASM state *immutable* along WASM module
/// execution. It means that `Module::call_func` execution does not have as side effect
/// for the WASM module's state, it becomes unchanged.
pub(crate) struct Module {
    /// `wasmtime::InstancePre` entity
    ///
    /// A reason why it is used a `wasmtime::InstancePre` instead of `wasmtime::Instance`
    /// partially described in this [RFC](https://github.com/bytecodealliance/rfcs/blob/main/accepted/shared-host-functions.md).
    /// It separates and optimizes the linkage of the imports to the WASM runtime from the
    /// module actual initialization process.
    pre_instance: WasmInstancePre<HermesRuntimeState>,

    /// `Engine` entity
    engine: Engine,

    /// Module id
    id: ModuleId,

    /// Module's execution counter
    exc_counter: AtomicU32,
}

impl Module {
    /// Instantiate WASM module
    ///
    /// # Errors
    ///  - `BadModuleError`
    ///  - `BadEngineConfigError`
    pub(crate) fn new(module_bytes: &[u8]) -> anyhow::Result<Self> {
        let engine = Engine::new()?;
        let module = WasmModule::new(&engine, module_bytes)
            .map_err(|e| BadWASMModuleError(e.to_string()))?;

        let mut linker = WasmLinker::new(&engine);
        bindings::Hermes::add_to_linker(&mut linker, |state: &mut HermesRuntimeState| state)
            .map_err(|e| BadWASMModuleError(e.to_string()))?;
        let pre_instance = linker
            .instantiate_pre(&module)
            .map_err(|e| BadWASMModuleError(e.to_string()))?;

        Ok(Self {
            pre_instance,
            engine,
            id: ModuleId(Ulid::generate()),
            exc_counter: AtomicU32::new(0),
        })
    }

    /// Get the module id
    #[allow(dead_code)]
    pub(crate) fn id(self) -> ModuleId {
        self.id
    }

    /// Executes a Hermes event by calling some WASM function.
    /// This function abstraction over actual execution of the WASM function,
    /// actual definition is inside `HermesEventPayload` trait implementation.
    ///
    /// For each call creates a brand new `wasmtime::Store` instance, which means that
    /// is has an initial state, based on the provided context for each call.
    ///
    /// # Errors
    /// - `BadModuleError`
    #[allow(dead_code)]
    pub(crate) fn execute_event(
        &self, event: &dyn HermesEventPayload, state: HermesRuntimeState,
    ) -> anyhow::Result<()> {
        let mut store = WasmStore::new(&self.engine, state);
        let (instance, _) = bindings::Hermes::instantiate_pre(&mut store, &self.pre_instance)
            .map_err(|e| BadWASMModuleError(e.to_string()))?;

        event.execute(&mut ModuleInstance { store, instance })?;

        // Using the highest memory ordering constraint.
        // It provides a highest consistency guarantee and in some cases could decrease
        // perfomance.
        // We could revise ordering approach for this case in future.
        self.exc_counter.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    // struct TestHost;
    // impl Host for TestHost {
    //     fn link_imports(_linker: &mut WasmLinker<Context>) -> anyhow::Result<()> {
    //         Ok(())
    //     }
    // }

    // struct TestEvent;
    // impl HermesEventPayload<ModuleInstance<WasmInstance>> for TestEvent {
    //     fn event_name(&self) -> &str {
    //         "inc-global"
    //     }

    //     fn execute(&self, instance: &mut ModuleInstance<WasmInstance>) ->
    // anyhow::Result<()> {         let func = instance
    //             .instance
    //             .get_typed_func::<(), (i32,)>(&mut instance.store, "inc-global")?;
    //         let (res,) = func.call(&mut instance.store, ())?;
    //         assert_eq!(res, 1);
    //         Ok(())
    //     }
    // }

    // #[test]
    // /// Tests that after instantiation of `Module` its state does not change after each
    // /// `Module::call_func` execution
    // fn preserve_module_state_test() {
    //     let wat = r#"
    //     (component
    //         (core module $Module
    //             (export "inc-global" (func $inc_global))

    //             (func $inc_global (result i32)
    //                 global.get $global_val
    //                 i32.const 1
    //                 i32.add
    //                 global.set $global_val
    //                 global.get $global_val
    //             )

    //             (global $global_val (mut i32) (i32.const 0))
    //         )
    //         (core instance $module (instantiate (module $Module)))
    //         (func $inc_global (result s32) (canon lift (core func $module
    // "inc-global")))         (export "inc-global" (func $inc_global))
    //     )"#;

    //     let mut module =
    //         Module::new("app".to_string(), wat.as_bytes()).expect("cannot load a WASM
    // module");

    //     for _ in 0..10 {
    //         module
    //             .execute_event(&TestEvent)
    //             .expect("cannot execute `TestEvent` event");
    //     }
    // }
}
