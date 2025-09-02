//! WASM module implementation.
//! Wrapper over the `wasmtime::Module`, `wasmtime::Instance` etc. structs which
//! define a WASM module abstraction with capability to interact with it.
//!
//! All implementation based on [wasmtime](https://crates.io/crates/wasmtime) crate dependency.

use std::{
    io::Read,
    sync::atomic::{AtomicU32, Ordering},
};

use rusty_ulid::Ulid;
use wasmtime::{
    component::{
        self, Component as WasmModule, InstancePre as WasmInstancePre, Linker as WasmLinker,
    },
    Store as WasmStore,
};

use crate::{
    event::HermesEventPayload,
    runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::{self, LinkOptions},
    wasm::engine::Engine,
};

/// Structure defines an abstraction over the WASM module instance.
/// It holds the state of the WASM module along with its context data.
/// It is used to interact with the WASM module.
#[allow(clippy::module_name_repetitions)]
pub struct ModuleInstance {
    /// `wasmtime::Store` entity
    pub(crate) store: WasmStore<HermesRuntimeContext>,
    /// `Instance` entity
    pub(crate) instance: component::Instance,
}

/// Module id type
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct ModuleId(pub(crate) Ulid);

impl std::fmt::Display for ModuleId {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Structure defines an abstraction over the WASM module
/// It instantiates the module with the provided context data,
/// links all provided imports to the module instance,
/// handles an internal state of the WASM module.
///
/// The primary goal for it is to make a WASM state *immutable* along WASM module
/// execution. It means that `Module::call_func` execution does not have as side effect
/// for the WASM module's state, it becomes unchanged.
pub struct Module {
    /// `wasmtime::InstancePre` entity
    ///
    /// A reason why it is used a `wasmtime::InstancePre` instead of `wasmtime::Instance`
    /// partially described in this [RFC](https://github.com/bytecodealliance/rfcs/blob/main/accepted/shared-host-functions.md).
    /// It separates and optimizes the linkage of the imports to the WASM runtime from the
    /// module actual initialization process.
    pre_instance: WasmInstancePre<HermesRuntimeContext>,

    /// `Engine` entity
    engine: Engine,

    /// Module id
    id: ModuleId,

    /// Module's execution counter
    exc_counter: AtomicU32,
}

impl Module {
    /// Instantiate WASM module from bytes
    ///
    /// # Errors
    ///  - `BadWASMModuleError`
    ///  - `BadEngineConfigError`
    pub fn from_bytes(module_bytes: &[u8]) -> anyhow::Result<Self> {
        let engine = Engine::new()?;
        let wasm_module = WasmModule::new(&engine, module_bytes)
            .map_err(|e| anyhow::anyhow!("Bad WASM module:\n {}", e.to_string()))?;

        let mut linker = WasmLinker::new(&engine);
        linker
            .allow_shadowing(true)
            .define_unknown_imports_as_traps(&wasm_module)?;
        bindings::Hermes::add_to_linker::<_, HermesRuntimeContext>(
            &mut linker,
            &LinkOptions::default(),
            |state: &mut HermesRuntimeContext| state,
        )
        .map_err(|e| anyhow::anyhow!("Bad WASM module:\n {}", e.to_string()))?;
        let pre_instance = linker
            .instantiate_pre(&wasm_module)
            .map_err(|e| anyhow::anyhow!("Bad WASM module:\n {}", e.to_string()))?;

        Ok(Self {
            pre_instance,
            engine,
            id: ModuleId(Ulid::generate()),
            exc_counter: AtomicU32::new(0),
        })
    }

    /// Instantiate WASM module reader
    ///
    /// # Errors
    ///  - `BadWASMModuleError`
    ///  - `BadEngineConfigError`
    ///  - `io::Error`
    pub fn from_reader(mut reader: impl Read) -> anyhow::Result<Self> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        Self::from_bytes(&bytes)
    }

    /// Get the module id
    pub(crate) fn id(&self) -> &ModuleId {
        &self.id
    }

    /// Get the module's execution counter
    pub(crate) fn exec_counter(&self) -> u32 {
        // Using the highest memory ordering constraint.
        // It provides a highest consistency guarantee and in some cases could decrease
        // performance.
        // We could revise ordering approach for this case in future.
        self.exc_counter.load(Ordering::SeqCst)
    }

    /// Executes a Hermes event by calling some WASM function.
    /// This function abstraction over actual execution of the WASM function,
    /// actual definition is inside `HermesEventPayload` trait implementation.
    ///
    /// For each call creates a brand new `wasmtime::Store` instance, which means that
    /// is has an initial state, based on the provided context for each call.
    ///
    /// # Errors:
    /// - `BadWASMModuleError`
    pub(crate) fn execute_event(
        &self,
        event: &dyn HermesEventPayload,
        state: HermesRuntimeContext,
    ) -> anyhow::Result<()> {
        let mut store = WasmStore::new(&self.engine, state);
        let instance = self
            .pre_instance
            .clone()
            .instantiate(&mut store)
            .map_err(|e| anyhow::anyhow!("Bad WASM module:\n {}", e.to_string()))?;

        event.execute(&mut ModuleInstance { store, instance })?;

        // Using the highest memory ordering constraint.
        // It provides a highest consistency guarantee and in some cases could decrease
        // performance.
        // We could revise ordering approach for this case in future.
        self.exc_counter.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[allow(missing_docs)]
#[cfg(feature = "bench")]
pub mod bench {
    use super::*;
    use crate::{
        app::ApplicationName, cli::Cli, runtime_context::HermesRuntimeContext,
        runtime_extensions::bindings::unchecked_exports, vfs::VfsBootstrapper,
    };

    unchecked_exports::define! {
        /// Extends [`wasmtime::component::Instance`] with guest functions for init.
        trait ComponentInstanceExt {
            #[wit("hermes:init/event", "init")]
            fn hermes_init_event_init() -> bool;
        }
    }

    /// Benchmark for executing the `init` event of the Hermes dummy component.
    /// It aims to measure the overhead of the WASM module and WASM state initialization
    /// process.
    pub fn module_hermes_component_bench(b: &mut criterion::Bencher) {
        struct Event;
        impl HermesEventPayload for Event {
            fn event_name(&self) -> &'static str {
                "init"
            }

            fn execute(
                &self,
                instance: &mut ModuleInstance,
            ) -> anyhow::Result<()> {
                instance
                    .instance
                    .hermes_init_event_init(&mut instance.store)?;
                Ok(())
            }
        }

        let module =
            Module::from_bytes(include_bytes!("../../../../wasm/stub-module/stub.wasm")).unwrap();

        let app_name = ApplicationName("integration-test".to_owned());

        let hermes_home_dir = Cli::hermes_home().unwrap();

        let vfs = std::sync::Arc::new(
            VfsBootstrapper::new(hermes_home_dir, app_name.clone().to_string())
                .bootstrap()
                .unwrap(),
        );

        b.iter(|| {
            module
                .execute_event(
                    &Event,
                    HermesRuntimeContext::new(
                        app_name.to_owned(),
                        module.id().clone(),
                        "init".to_string(),
                        0,
                        vfs.clone(),
                    ),
                )
                .unwrap();
        });
    }

    /// Benchmark for executing the `foo` WASM function of the tiny component.
    /// The general flow of how WASM module is instantiated and executed is the same as in
    /// the previous one `module_hermes_component_bench`.
    /// It aims to compare how the size of the component affects on the execution time.
    pub fn module_small_component_bench(b: &mut criterion::Bencher) {
        let wat = r#"
            (component
                (core module $Module
                    (export "foo" (func $foo))
                    (func $foo (result i32)
                        i32.const 1
                    )
                )
                (core instance $module (instantiate (module $Module)))
                (func $foo (result s32) (canon lift (core func $module "foo")))
                (export "foo" (func $foo))
            )"#;

        let engine = Engine::new().unwrap();
        let module = WasmModule::new(&engine, wat.as_bytes()).unwrap();
        let linker = WasmLinker::new(&engine);
        let pre_instance = linker.instantiate_pre(&module).unwrap();

        b.iter(|| {
            let mut store = WasmStore::new(&engine, ());
            let instance = pre_instance.instantiate(&mut store).unwrap();
            let func = instance
                .get_typed_func::<(), (i32,)>(&mut store, "foo")
                .unwrap();
            let (res,) = func.call(&mut store, ()).unwrap();
            assert_eq!(res, 1);
        });
    }

    /// Benchmark for executing the `foo` WASM function of the tiny component.
    /// BUT with the changed execution flow. Here the WASM module and WASM state is
    /// instantiated ONCE during the whole execution process.
    pub fn module_small_component_full_pre_load_bench(b: &mut criterion::Bencher) {
        let wat = r#"
            (component
                (core module $Module
                    (export "foo" (func $foo))
                    (func $foo (result i32)
                        i32.const 1
                    )
                )
                (core instance $module (instantiate (module $Module)))
                (func $foo (result s32) (canon lift (core func $module "foo")))
                (export "foo" (func $foo))
            )"#;

        let engine = Engine::new().unwrap();
        let module = WasmModule::new(&engine, wat.as_bytes()).unwrap();
        let linker = WasmLinker::new(&engine);
        let mut store = WasmStore::new(&engine, ());
        let instance = linker.instantiate(&mut store, &module).unwrap();
        let func = instance
            .get_typed_func::<(), (i32,)>(&mut store, "foo")
            .unwrap();

        b.iter(|| {
            let (res,) = func.call(&mut store, ()).unwrap();
            assert_eq!(res, 1);
            func.post_return(&mut store).unwrap();
        });
    }
}
