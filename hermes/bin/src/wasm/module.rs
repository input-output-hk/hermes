//! WASM module implementation.
//! Wrapper over the `wasmtime::Module`, `wasmtime::Instance` etc. structs which
//! define a WASM module abstraction with capability to interact with it.
//!
//! All implementation based on [wasmtime](https://crates.io/crates/wasmtime) crate dependency.

use std::error::Error;

use wasmtime::{
    InstancePre as WasmInstancePre, Linker as WasmLinker, Module as WasmModule, Store as WasmStore,
    WasmParams, WasmResults,
};

use super::{context::Context, engine::Engine};

/// Interface for linking WASM imports
pub(crate) trait LinkImport<Context> {
    /// Link imports to the `wasmtime::Linker`
    fn link(&self, linker: &mut WasmLinker<Context>) -> Result<(), Box<dyn Error>>;
}

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
    pre_instance: WasmInstancePre<Context>,

    /// `Engine` entity
    engine: Engine,

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
        engine: Engine, app_name: String, module_bytes: &[u8],
        imports: &[Box<dyn LinkImport<Context>>],
    ) -> Result<Self, Box<dyn Error>> {
        let module = WasmModule::new(&engine, module_bytes)?;

        let mut linker = WasmLinker::new(&engine);
        for import in imports {
            import.link(&mut linker)?;
        }
        let pre_instance = linker.instantiate_pre(&module)?;

        Ok(Self {
            pre_instance,
            engine,
            context: Context::new(app_name),
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
        self.context.use_for(name.to_string());

        let mut store = WasmStore::new(&self.engine, self.context.clone());
        let instance = self.pre_instance.instantiate(&mut store)?;
        let func = instance.get_typed_func(&mut store, name)?;
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
    /// Tests that after instantiation of `Module` its state does not change after each
    /// `Module::call_func` execution
    fn preserve_module_state_test() {
        let engine = Engine::new().expect("");
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

        let mut module = Module::new(engine, "app".to_string(), wat.as_bytes(), &[Box::new(
            ImportHelloFunc,
        )])
        .expect("");

        for _ in 0..10 {
            let res: i32 = module.call_func("call_hello", ()).expect("");
            assert_eq!(res, 1);
        }
    }
}
