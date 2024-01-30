//! WASM module implementation.
//! Wrapper over the `wasmtime::Module` struct with some specific validation and
//! configuration setup.

use std::{collections::HashSet, marker::PhantomData};

use wasmtime::{
    Func as WasmFunc, FuncType as WasmFuncType, Instance as WasmModuleInstance,
    Linker as WasmLinker, Module as WasmModule, Store as WasmStore, WasmParams, WasmResults,
};

use super::{engine::Engine, Error};

/// WASM [import](https://openhome.cc/eGossip/WebAssembly/ImportExport.html) function entity struct
#[derive(Debug, Clone)]
pub(crate) struct ImportFunc {
    /// Module name
    module: String,
    /// Function name
    name: String,
    /// Function signature definition
    func: WasmFunc,
}

/// WASM [export](https://openhome.cc/eGossip/WebAssembly/ImportExport.html) function entity struct
#[derive(Debug, Clone)]
pub(crate) struct ExportFunc {
    /// Function name
    name: String,
    /// Function signature definition
    func: WasmFuncType,
}

/// WASM module struct
#[derive(Clone)]
pub(crate) struct Module<ContextType> {
    /// `wasmtime::Instance` module instance
    instance: WasmModuleInstance,
    /// `wasmtime::Store` context type
    _ctx_type: PhantomData<ContextType>,
}

impl<ContextType> Module<ContextType> {
    /// Check module exports with the given list
    /// It is a 1 on 1 check, so every module export must be in the list and wise versa.
    ///
    /// # Errors
    /// - `Error::ExportsMismatch`. Exports mismatch
    /// - `Error::ExportNotAFunc`. Export module entity is not a function
    fn check_exports(module: &WasmModule, exports: &[ExportFunc]) -> Result<(), Error> {
        let mut module_exports = HashSet::new();
        for module_export in module.exports() {
            if let Some(module_export_func) = module_export.ty().func() {
                module_exports
                    .insert((module_export.name().to_string(), module_export_func.clone()));
            } else {
                return Err(Error::ExportNotAFunc(module_export.name().to_string()));
            }
        }

        let expected_exports: HashSet<_> = exports
            .iter()
            .map(|export| (export.name.clone(), export.func.clone()))
            .collect();

        if expected_exports == module_exports {
            Ok(())
        } else {
            Err(Error::ExportsMismatch)
        }
    }

    /// Check module exports with the given list
    /// It is a 1 on 1 check, so every module export must be in the list and wise versa.
    ///
    /// # Errors
    /// - `Error::ImportsMismatch`. Imports mismatch
    /// - `Error::ImportNotAFunc`. Import module entity is not a function
    fn check_imports(
        store: &mut WasmStore<ContextType>, module: &WasmModule, imports: &[ImportFunc],
    ) -> Result<(), Error> {
        let mut module_imports = HashSet::new();
        for module_import in module.imports() {
            if let Some(module_import_func) = module_import.ty().func() {
                module_imports.insert((
                    module_import.module().to_string(),
                    module_import.name().to_string(),
                    module_import_func.clone(),
                ));
            } else {
                return Err(Error::ImportNotAFunc(
                    module_import.name().to_string(),
                    module_import.module().to_string(),
                ));
            }
        }

        let expected_imports: HashSet<_> = imports
            .iter()
            .map(|import| {
                (
                    import.module.clone(),
                    import.name.clone(),
                    import.func.ty(&mut *store),
                )
            })
            .collect();

        if expected_imports == module_imports {
            Ok(())
        } else {
            Err(Error::ImportsMismatch)
        }
    }

    /// Instantiate WASM module
    ///
    /// # Errors
    /// - `Error::ExportsMismatch`. Exports mismatch
    /// - `Error::ImportsMismatch`. Imports mismatch
    /// - `Error::ExportNotAFunc`. Export module entity is not a function
    /// - `Error::ImportNotAFunc`. Import module entity is not a function
    /// - `Error::Wasm`: WASM instantiation error
    #[allow(dead_code)]
    pub(crate) fn new(
        engine: &Engine, store: &mut WasmStore<ContextType>, module_bytes: &[u8],
        imports: &[ImportFunc], exports: &[ExportFunc],
    ) -> Result<Self, Error> {
        let module = WasmModule::new(engine, module_bytes)?;

        Self::check_exports(&module, exports)?;
        Self::check_imports(store, &module, imports)?;

        let mut linker = WasmLinker::new(engine);
        for import in imports {
            linker.define(&mut *store, &import.module, &import.name, import.func)?;
        }
        let instance = linker.instantiate(store, &module)?;

        Ok(Self {
            instance,
            _ctx_type: PhantomData,
        })
    }

    /// Call WASM module's function
    ///
    /// # Errors
    /// - `Error::Wasm`: WASM call error
    #[allow(dead_code)]
    pub(crate) fn call_func<Args, Ret>(
        &mut self, store: &mut WasmStore<ContextType>, name: &str, args: Args,
    ) -> Result<Ret, Error>
    where
        Args: WasmParams,
        Ret: WasmResults,
    {
        let func = self.instance.get_typed_func(&mut *store, name)?;
        Ok(func.call(store, args)?)
    }
}

#[cfg(test)]
mod tests {
    use wasmtime::Caller;

    use super::*;

    #[test]
    fn module_test_1() {
        let engine = Engine::new().expect("");
        let mut store = WasmStore::new(&engine, 0);
        let wat = "(module)";

        let imports = [];
        let exports = [];
        assert!(Module::new(&engine, &mut store, wat.as_bytes(), &imports, &exports).is_ok());

        let imports = [ImportFunc {
            module: String::new(),
            name: "hello".to_string(),
            func: WasmFunc::wrap(&mut store, || {
                println!("Hello!");
            }),
        }];
        let exports = [];
        assert!(Module::new(&engine, &mut store, wat.as_bytes(), &imports, &exports).is_err());

        let imports = [];
        let exports = [ExportFunc {
            name: "call_hello".to_string(),
            func: WasmFuncType::new([], []),
        }];
        assert!(Module::new(&engine, &mut store, wat.as_bytes(), &imports, &exports).is_err());

        let imports = [ImportFunc {
            module: String::new(),
            name: "hello".to_string(),
            func: WasmFunc::wrap(&mut store, || {
                println!("Hello!");
            }),
        }];
        let exports = [ExportFunc {
            name: "call_hello".to_string(),
            func: WasmFuncType::new([], []),
        }];
        assert!(Module::new(&engine, &mut store, wat.as_bytes(), &imports, &exports).is_err());
    }

    #[test]
    fn module_test_2() {
        let engine = Engine::new().expect("");
        let mut store = WasmStore::new(&engine, 0);
        let wat = r#"
                    (module
                        (func $hello_1 (import "" "hello"))
                        (func $hello_2 (import "" "hello"))
                        (func $hello_3 (import "" "hello_1"))
                    )"#;

        let imports = [
            ImportFunc {
                module: String::new(),
                name: "hello".to_string(),
                func: WasmFunc::wrap(&mut store, || {
                    println!("Hello!");
                }),
            },
            ImportFunc {
                module: String::new(),
                name: "hello_1".to_string(),
                func: WasmFunc::wrap(&mut store, || {
                    println!("Hello_1!");
                }),
            },
        ];
        let exports = [];
        assert!(Module::new(&engine, &mut store, wat.as_bytes(), &imports, &exports).is_ok());

        let imports = [
            ImportFunc {
                module: String::new(),
                name: "hello".to_string(),
                func: WasmFunc::wrap(&mut store, || {
                    println!("Hello!");
                }),
            },
            ImportFunc {
                module: String::new(),
                name: "hello_1".to_string(),
                func: WasmFunc::wrap(&mut store, || {
                    println!("Hello_1!");
                }),
            },
        ];
        let exports = [ExportFunc {
            name: "call_hello".to_string(),
            func: WasmFuncType::new([], []),
        }];
        assert!(Module::new(&engine, &mut store, wat.as_bytes(), &imports, &exports).is_err());

        let imports = [];
        let exports = [ExportFunc {
            name: "call_hello".to_string(),
            func: WasmFuncType::new([], []),
        }];
        assert!(Module::new(&engine, &mut store, wat.as_bytes(), &imports, &exports).is_err());

        let imports = [];
        let exports = [];
        assert!(Module::new(&engine, &mut store, wat.as_bytes(), &imports, &exports).is_err());
    }

    #[test]
    fn module_test_3() {
        let engine = Engine::new().expect("");
        let mut store = WasmStore::new(&engine, 0);
        let wat = r#"
                    (module
                        (func (export "call_hello"))
                    )"#;

        let imports = [];
        let exports = [ExportFunc {
            name: "call_hello".to_string(),
            func: WasmFuncType::new([], []),
        }];
        assert!(Module::new(&engine, &mut store, wat.as_bytes(), &imports, &exports).is_ok());

        let imports = [ImportFunc {
            module: String::new(),
            name: "hello".to_string(),
            func: WasmFunc::wrap(&mut store, || {
                println!("Hello!");
            }),
        }];
        let exports = [ExportFunc {
            name: "call_hello".to_string(),
            func: WasmFuncType::new([], []),
        }];
        assert!(Module::new(&engine, &mut store, wat.as_bytes(), &imports, &exports).is_err());

        let imports = [ImportFunc {
            module: String::new(),
            name: "hello".to_string(),
            func: WasmFunc::wrap(&mut store, || {
                println!("Hello!");
            }),
        }];
        let exports = [];
        assert!(Module::new(&engine, &mut store, wat.as_bytes(), &imports, &exports).is_err());

        let imports = [];
        let exports = [];
        assert!(Module::new(&engine, &mut store, wat.as_bytes(), &imports, &exports).is_err());
    }

    #[test]
    fn module_test_4() {
        let engine = Engine::new().expect("");
        let mut store = WasmStore::new(&engine, 0);
        let wat = r#"
                    (module
                        (func $hello_2 (import "" "hello"))
                        (func $hello_1 (import "" "hello"))
                        (func (export "call_hello"))
                    )"#;

        let imports = [ImportFunc {
            module: String::new(),
            name: "hello".to_string(),
            func: WasmFunc::wrap(&mut store, || {
                println!("Hello!");
            }),
        }];
        let exports = [ExportFunc {
            name: "call_hello".to_string(),
            func: WasmFuncType::new([], []),
        }];
        assert!(Module::new(&engine, &mut store, wat.as_bytes(), &imports, &exports).is_ok());

        let imports = [ImportFunc {
            module: String::new(),
            name: "hello".to_string(),
            func: WasmFunc::wrap(&mut store, || {
                println!("Hello!");
            }),
        }];
        let exports = [];
        assert!(Module::new(&engine, &mut store, wat.as_bytes(), &imports, &exports).is_err());

        let imports = [];
        let exports = [ExportFunc {
            name: "call_hello".to_string(),
            func: WasmFuncType::new([], []),
        }];
        assert!(Module::new(&engine, &mut store, wat.as_bytes(), &imports, &exports).is_err());

        let imports = [];
        let exports = [];
        assert!(Module::new(&engine, &mut store, wat.as_bytes(), &imports, &exports).is_err());
    }

    #[test]
    fn module_test() {
        let engine = Engine::new().expect("");
        let mut store = WasmStore::new(&engine, 0);

        let wat = r#"
                    (module
                        (func $hello_2 (import "" "hello"))
                        (func $hello_1 (import "" "hello"))
                        (func (export "call_hello")
                            call $hello_1
                        )
                    )"#;

        let imports = [ImportFunc {
            module: String::new(),
            name: "hello".to_string(),
            func: WasmFunc::wrap(&mut store, |mut ctx: Caller<'_, i32>| {
                *ctx.data_mut() += 1;
                println!("Hello_0!");
            }),
        }];
        let exports = [ExportFunc {
            name: "call_hello".to_string(),
            func: WasmFuncType::new([], []),
        }];

        let mut module =
            Module::new(&engine, &mut store, wat.as_bytes(), &imports, &exports).expect("");
        module
            .call_func::<(), ()>(&mut store, "call_hello", ())
            .expect("");
        println!("store: {}", store.data());
    }
}
