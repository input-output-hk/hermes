//! WASM module implementation.
//! Wrapper over the `wasmtime::Module`, `wasmtime::Instance` etc. structs which
//! define a WASM module abstrction with capability to interact with it.
//!
//! All implementation based on [wasmtime](https://crates.io/crates/wasmtime) crate dependency.

use std::error::Error;

use wasmtime::{
    Config as WasmConfig, Engine as WasmEngine, InstancePre as WasmModuleInstance,
    Linker as WasmLinker, Module as WasmModule, Store as WasmStore, WasmParams, WasmResults,
};

use super::context::Context;

/// Interface for linking WASM imports
pub(crate) trait LinkImport<Context> {
    /// Link imports to the `wasmtime::Linker`
    fn link(&self, linker: &mut WasmLinker<Context>) -> Result<(), Box<dyn Error>>;
}

/// Structure defines an abstaction over the WASM module
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
    instance: WasmModuleInstance<Context>,

    /// `wasmtime::Engine` entity
    engine: WasmEngine,

    /// `Context` entity
    context: Context,
}

impl Module {
    /// Instantiate WASM module
    ///
    /// # Errors
    ///  - `wasmtime::Error`: WASM call error
    #[allow(dead_code)]
    pub(crate) fn new(
        context: Context, module_bytes: &[u8], imports: &[Box<dyn LinkImport<Context>>],
    ) -> Result<Self, Box<dyn Error>> {
        let mut config = WasmConfig::new();
        config.wasm_component_model(true);
        config.consume_fuel(false);

        let engine = WasmEngine::new(&config)?;

        let module = WasmModule::new(&engine, module_bytes)?;

        let mut linker = WasmLinker::new(&engine);
        for import in imports {
            import.link(&mut linker)?;
        }
        let instance = linker.instantiate_pre(&module)?;

        Ok(Self {
            instance,
            engine,
            context,
        })
    }

    /// Call WASM module's function.
    /// For each call creates a brand new `wasmtime::Store` instance, which means that
    /// is has an initial state, based on the provided context for each call.
    ///
    /// # Errors
    /// - `wasmtime::Error`: WASM call error
    #[allow(dead_code)]
    pub(crate) fn call_func<Args, Ret>(
        &mut self, name: &str, args: Args,
    ) -> Result<Ret, Box<dyn Error>>
    where
        Args: WasmParams,
        Ret: WasmResults,
    {
        let mut store = WasmStore::new(&self.engine, self.context.clone());
        let instantiated_instance = self.instance.instantiate(&mut store)?;
        let func = instantiated_instance.get_typed_func(&mut store, name)?;
        Ok(func.call(&mut store, args)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ImportHelloFunc;

    impl LinkImport<Context> for ImportHelloFunc {
        fn link(&self, linker: &mut WasmLinker<Context>) -> Result<(), Box<dyn Error>> {
            linker.func_wrap("", "hello", || {
                println!("hello");
            })?;
            Ok(())
        }
    }

    #[test]
    /// Tests that after instantiation of `Module` it's state does not change after each
    /// `Module::call_func` execution
    fn preserve_module_state_test() {
        let wat = r#"
                    (module
                        (import "" "hello" (func $hello_0))
                        (export "call_hello" (func $call_hello))

                        (func $call_hello (result i32)
                            global.get $global_val
                            i32.const 1
                            i32.add
                            global.set $global_val
                            global.get $global_val
                        )

                        (global $global_val (mut i32) (i32.const 0))
                    )"#;

        let mut module =
            Module::new(Context, wat.as_bytes(), &[Box::new(ImportHelloFunc)]).expect("");

        for _ in 0..10 {
            let res: i32 = module.call_func("call_hello", ()).expect("");
            assert_eq!(res, 1);
        }
    }
}
