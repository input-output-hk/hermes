use std::{collections::HashMap, error::Error, marker::PhantomData};

use wasmtime::{
    Func as WasmFunc, FuncType, Instance as WasmModuleInstance, Linker as WasmLinker,
    Module as WasmModule, Store as WasmStore, WasmParams, WasmResults,
};

use super::engine::Engine;

pub(crate) struct ImportFunc {
    module: String,
    name: String,
    func: WasmFunc,
}

pub(crate) struct ExportFunc {
    name: String,
    func: FuncType,
}

pub(crate) struct Module<ContextType> {
    instance: WasmModuleInstance,
    _ctx_type: PhantomData<ContextType>,
}

impl<ContextType> Module<ContextType> {
    fn check_exports(module: &WasmModule, exports: &[ExportFunc]) -> Result<(), Box<dyn Error>> {
        let mut indexed_exports = HashMap::new();
        for export in exports {
            if indexed_exports
                .insert(export.name.clone(), export.func.clone())
                .is_some()
            {
                return Err(format!("Duplicate export: {}", export.name).into());
            }
        }

        for module_export in module.exports() {
            if let Some(func) = indexed_exports.get(module_export.name()) {
                if let Some(ty) = module_export.ty().func() {
                    if func != ty {
                        return Err(format!(
                            "Export func signature mismatch, current: {ty:?}, provided: {func:?}",
                        )
                        .into());
                    }
                } else {
                    return Err(format!("Export not a function: {module_export:?}").into());
                }
            } else {
                return Err(format!("Export not found: {}", module_export.name()).into());
            }
        }

        Ok(())
    }

    fn check_imports(
        store: &mut WasmStore<ContextType>, module: &WasmModule, imports: &[ImportFunc],
    ) -> Result<(), Box<dyn Error>> {
        let mut indexed_imports = HashMap::new();
        for import in imports {
            let import_module: &mut HashMap<_, _> =
                indexed_imports.entry(import.module.clone()).or_default();
            if import_module
                .insert(import.name.clone(), import.func)
                .is_some()
            {
                return Err(format!(
                    "Duplicate import, module: {}, name: {}",
                    import.module, import.name,
                )
                .into());
            }
        }

        for module_import in module.imports() {
            if let Some(import_module) = indexed_imports.get(module_import.module()) {
                if let Some(func) = import_module.get(module_import.name()) {
                    if let Some(ty) = module_import.ty().func() {
                        if &func.ty(&mut *store) != ty {
                            return Err(format!(
                                "Export func signature mismatch, current: {ty:?}, provided: {func:?}",
                            )
                            .into());
                        }
                    } else {
                        return Err(format!(
                            "Import not a function, module: {}, name: {}",
                            module_import.module(),
                            module_import.name()
                        )
                        .into());
                    }
                } else {
                    return Err(format!(
                        "Import not found, module: {}, name: {}",
                        module_import.module(),
                        module_import.name()
                    )
                    .into());
                }
            } else {
                return Err(format!(
                    "Import not found, module: {}, name: {}",
                    module_import.module(),
                    module_import.name()
                )
                .into());
            }
        }

        Ok(())
    }

    pub(crate) fn new(
        engine: &Engine, store: &mut WasmStore<ContextType>, module_bytes: &[u8],
        imports: &[ImportFunc], exports: &[ExportFunc],
    ) -> Result<Self, Box<dyn Error>> {
        let module = WasmModule::new(engine, module_bytes)?;

        Self::check_exports(&module, exports)?;
        Self::check_imports(store, &module, imports)?;

        // let instance = WasmModuleInstance::new(
        //     &mut (*store),
        //     &module,
        //     &imports
        //         .iter()
        //         .map(|val| val.func.into())
        //         .collect::<Vec<_>>(),
        // )?;

        let mut linker = WasmLinker::new(engine);
        for import in imports {
            linker.define(&mut *store, &import.module, &import.name, import.func)?;
        }
        let instance = linker.instantiate(store, &module)?;

        Ok(Self {
            instance,
            _ctx_type: PhantomData::default(),
        })
    }

    pub(crate) fn call_func<Args, Ret>(
        &mut self, store: &mut WasmStore<ContextType>, name: &str, args: Args,
    ) -> Result<Ret, Box<dyn Error>>
    where
        Args: WasmParams,
        Ret: WasmResults,
    {
        let func = self.instance.get_typed_func(&mut (*store), name)?;
        Ok(func.call(&mut *store, args)?)
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

        let imports = [
            ImportFunc {
                module: "".to_string(),
                name: "hello".to_string(),
                func: WasmFunc::wrap(&mut store, |mut ctx: Caller<'_, i32>| {
                    *ctx.data_mut() += 1;
                    println!("Hello_0!");
                }),
            },
            ImportFunc {
                module: "".to_string(),
                name: "hello1".to_string(),
                func: WasmFunc::wrap(&mut store, |mut ctx: Caller<'_, i32>| {
                    *ctx.data_mut() += 2;
                    println!("Hello_1!");
                }),
            },
        ];
        let exports = [ExportFunc {
            name: "call_hello".to_string(),
            func: FuncType::new([], []),
        }];

        let mut module =
            Module::new(&engine, &mut store, wat.as_bytes(), &imports, &exports).expect("");
        module
            .call_func::<(), ()>(&mut store, "call_hello", ())
            .expect("");
        println!("store: {}", store.data());
    }
}
