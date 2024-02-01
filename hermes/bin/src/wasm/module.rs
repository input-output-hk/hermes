//! WASM module implementation.
//! Wrapper over the `wasmtime::Module` struct with some specific validation and
//! configuration setup.

use std::error::Error;

use wasmtime::{
    Instance as WasmModuleInstance, Linker as WasmLinker, Module as WasmModule, Store as WasmStore,
    WasmParams, WasmResults,
};

use super::engine::Engine;

///
pub(crate) trait LinkImport<ContextT> {
    ///
    fn link(&self, linker: &mut WasmLinker<ContextT>) -> Result<(), Box<dyn Error>>;
}

/// WASM module struct
pub(crate) struct Module<ContextType> {
    /// `wasmtime::Instance` module instance
    instance: WasmModuleInstance,
    /// `wasmtime::Store`
    store: WasmStore<ContextType>,
}

impl<ContextType: 'static> Module<ContextType> {
    /// Instantiate WASM module
    ///
    /// # Errors
    /// - `Error::Wasm`: WASM instantiation error
    #[allow(dead_code)]
    pub(crate) fn new(
        engine: &Engine, ctx: ContextType, module_bytes: &[u8],
        imports: &[Box<dyn LinkImport<ContextType>>],
    ) -> Result<Self, Box<dyn Error>> {
        let module = WasmModule::new(engine, module_bytes)?;
        let mut store = WasmStore::new(engine, ctx);

        let mut linker = WasmLinker::new(engine);
        for import in imports {
            import.link(&mut linker)?;
        }
        let instance = linker.instantiate(&mut store, &module)?;

        Ok(Self { instance, store })
    }

    /// Call WASM module's function
    ///
    /// # Errors
    /// - `Error::Wasm`: WASM call error
    #[allow(dead_code)]
    pub(crate) fn call_func<Args, Ret>(
        &mut self, name: &str, args: Args,
    ) -> Result<Ret, Box<dyn Error>>
    where
        Args: WasmParams,
        Ret: WasmResults,
    {
        let func = self.instance.get_typed_func(&mut self.store, name)?;
        Ok(func.call(&mut self.store, args)?)
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
    fn module_test() {
        let engine = Engine::new().expect("");

        let wat = r#"
                    (module
                        (func $hello_2 (import "" "hello"))
                        (func $hello_1 (import "" "hello"))
                        (func (export "call_hello")
                            call $hello_1
                        )
                    )"#;

        let hello_func: Box<dyn LinkImport<i32>> = Box::new(ImportHelloFunc);
        let imports = [hello_func];

        let mut module = Module::new(&engine, 0, wat.as_bytes(), &imports).expect("");
        module.call_func::<(), ()>("call_hello", ()).expect("");
    }
}
