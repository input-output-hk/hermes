//! WASM module implementation.
//! Wrapper over the `wasmtime::Module` struct with some specific validation and
//! configuration setup.

use std::error::Error;

use wasmtime::{
    Config as WasmConfig, Engine as WasmEngine, InstancePre as WasmModuleInstance,
    Linker as WasmLinker, Module as WasmModule, Store as WasmStore, WasmParams, WasmResults,
};

///
pub(crate) trait LinkImport<ContextT> {
    ///
    fn link(&self, linker: &mut WasmLinker<ContextT>) -> Result<(), Box<dyn Error>>;
}

/// WASM module struct
pub(crate) struct Module<ContextType: Clone> {
    /// `wasmtime::Instance` module instance
    instance: WasmModuleInstance<ContextType>,

    /// `wasmtime::Engine` entity
    engine: WasmEngine,

    ///
    context: ContextType,
}

impl<ContextType: Clone> Module<ContextType> {
    /// Instantiate WASM module
    ///
    /// # Errors
    ///  - `wasmtime::Error`: WASM call error
    #[allow(dead_code)]
    pub(crate) fn new(
        context: ContextType, module_bytes: &[u8], imports: &[Box<dyn LinkImport<ContextType>>],
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
    /// For each call it create a brand new `wasmtime::Store` instance, which means that
    /// is has a clean state for each call.
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
        let mut store: WasmStore<ContextType> = WasmStore::new(&self.engine, self.context.clone());
        let instantiated_instance = self.instance.instantiate(&mut store)?;
        let func = instantiated_instance.get_typed_func(&mut store, name)?;
        Ok(func.call(&mut store, args)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ImportHelloFunc;

    impl LinkImport<i32> for ImportHelloFunc {
        fn link(&self, linker: &mut WasmLinker<i32>) -> Result<(), Box<dyn Error>> {
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

        let mut module = Module::new(0, wat.as_bytes(), &[Box::new(ImportHelloFunc)]).expect("");

        for _ in 0..10 {
            let res: i32 = module.call_func("call_hello", ()).expect("");
            assert_eq!(res, 1);
        }
    }
}
