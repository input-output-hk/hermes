//! WASM module implementation.
//! Wrapper over the `wasmtime::Module`, `wasmtime::Instance` etc. structs which
//! define a WASM module abstraction with capability to interact with it.
//!
//! All implementation based on [wasmtime](https://crates.io/crates/wasmtime) crate dependency.

use wasmtime::{
    component::{
        Component as WasmModule, Instance as WasmInstance, InstancePre as WasmInstancePre,
        Linker as WasmLinker,
    },
    Store as WasmStore,
};

use super::{context::Context, engine::Engine};
use crate::event::HermesEventPayload;

/// Interface for linking WASM imports
pub(crate) trait Host<Context> {
    /// Link imports to the `wasmtime::Linker`
    fn link_imports(linker: &mut WasmLinker<Context>) -> anyhow::Result<()>;
}

/// Interface for WASM module instance
pub(crate) trait Instance: Sized {
    /// Instantiate WASM module instance
    fn instantiate(
        store: &mut WasmStore<Context>, pre_instance: &WasmInstancePre<Context>,
    ) -> anyhow::Result<Self>;
}

impl Instance for WasmInstance {
    fn instantiate(
        mut store: &mut WasmStore<Context>, pre_instance: &WasmInstancePre<Context>,
    ) -> anyhow::Result<Self> {
        let instance = pre_instance.instantiate(&mut store)?;
        Ok(instance)
    }
}

/// Structure defines an abstraction over the WASM module instance.
/// It holds the state of the WASM module along with its context data.
/// It is used to interact with the WASM module.
pub(crate) struct ModuleInstance<I: Instance> {
    /// `wasmtime::Store` entity
    #[allow(dead_code)]
    pub(crate) store: WasmStore<Context>,
    /// `Instance` entity
    #[allow(dead_code)]
    pub(crate) instance: I,
}

impl<I: Instance> ModuleInstance<I> {
    /// Instantiates WASM module
    pub(crate) fn new(
        mut store: WasmStore<Context>, pre_instance: &WasmInstancePre<Context>,
    ) -> anyhow::Result<Self> {
        let instance = I::instantiate(&mut store, pre_instance)?;
        Ok(Self { store, instance })
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
pub(crate) struct Module<H: Host<Context>> {
    /// `wasmtime::InstancePre` entity
    ///
    /// A reason why it is used a `wasmtime::InstancePre` instead of `wasmtime::Instance`
    /// partially described in this [RFC](https://github.com/bytecodealliance/rfcs/blob/main/accepted/shared-host-functions.md).
    /// It separates and optimizes the linkage of the imports to the WASM runtime from the
    /// module actual initialization process.
    pre_instance: WasmInstancePre<Context>,

    /// `Engine` entity
    engine: Engine,

    /// `Context` entity
    context: Context,

    /// `Host` type
    _host: std::marker::PhantomData<H>,
}

impl<H: Host<Context>> Module<H> {
    /// Instantiate WASM module
    ///
    /// # Errors
    ///  - `wasmtime::Error`: WASM call error
    #[allow(dead_code)]
    pub(crate) fn new(
        engine: Engine, app_name: String, module_bytes: &[u8],
    ) -> anyhow::Result<Self> {
        let module = WasmModule::new(&engine, module_bytes)?;

        let mut linker = WasmLinker::new(&engine);
        H::link_imports(&mut linker)?;
        let pre_instance = linker.instantiate_pre(&module)?;

        Ok(Self {
            pre_instance,
            engine,
            context: Context::new(app_name),
            _host: std::marker::PhantomData,
        })
    }

    /// Executes a Hermes event by calling some WASM function.
    /// This function abstraction over actual execution of the WASM function,
    /// actuall definition is inside `HermesEventPayload` trait implementation.
    ///
    /// For each call creates a brand new `wasmtime::Store` instance, which means that
    /// is has an initial state, based on the provided context for each call.
    ///
    /// # Errors
    /// - `wasmtime::Error`: WASM call error
    #[allow(dead_code)]
    pub(crate) fn execute_event<I: Instance>(
        &mut self, event: &impl HermesEventPayload<ModuleInstance<I>>,
    ) -> anyhow::Result<()> {
        self.context.use_for(event.event_name().to_string());

        let store = WasmStore::new(&self.engine, self.context.clone());
        let mut instance = ModuleInstance::new(store, &self.pre_instance)?;
        event.execute(&mut instance)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHost;
    impl Host<Context> for TestHost {
        fn link_imports(_linker: &mut WasmLinker<Context>) -> anyhow::Result<()> {
            Ok(())
        }
    }

    struct TestEvent;
    impl HermesEventPayload<ModuleInstance<WasmInstance>> for TestEvent {
        fn event_name(&self) -> &str {
            "inc-global"
        }

        fn execute(&self, instance: &mut ModuleInstance<WasmInstance>) -> anyhow::Result<()> {
            let func = instance
                .instance
                .get_typed_func::<(), (i32,)>(&mut instance.store, "inc-global")?;
            let (res,) = func.call(&mut instance.store, ())?;
            assert_eq!(res, 1);
            Ok(())
        }
    }

    #[test]
    /// Tests that after instantiation of `Module` its state does not change after each
    /// `Module::call_func` execution
    fn preserve_module_state_test() {
        let engine = Engine::new().expect("");
        let wat = r#"
        (component
            (core module $Module
                (export "inc-global" (func $inc_global))

                (func $inc_global (result i32)
                    global.get $global_val
                    i32.const 1
                    i32.add
                    global.set $global_val
                    global.get $global_val
                )

                (global $global_val (mut i32) (i32.const 0))
            )
            (core instance $module (instantiate (module $Module)))
            (func $inc_global (result s32) (canon lift (core func $module "inc-global")))
            (export "inc-global" (func $inc_global))
        )"#;

        let mut module =
            Module::<TestHost>::new(engine, "app".to_string(), wat.as_bytes()).expect("");

        for _ in 0..10 {
            module.execute_event(&TestEvent).expect("");
        }
    }
}
