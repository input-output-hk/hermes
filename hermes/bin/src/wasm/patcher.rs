//! Patcher for WASM files.
//! It injects functions to get memory size and read raw memory bytes into the WASM file.

// TODO[RC]: Patcher is not yet wired into the Hermes.
#![allow(unused)]

use std::path::Path;

use regex::Regex;

/// Magic string to avoid name collisions with existing functions.
const MAGIC: &str = r"vmucqq2137emxpatzkmuyy1szcpx23lp-hermes-";

/// Regex to detect the function definitions in the core module.
const CORE_FUNC_REGEX: &str = r"\(func\s+\$[^\s()]+[^)]*\(;";

/// Regex to detect the aliases of core functions in the component part.
const COMPONENT_CORE_FUNC_REGEX: &str = r#"\(alias\s+core\s+export\s+0\s+"[^"]+"\s+\(core\s+func"#;

/// Regex to detect the function definitions in the component part.
const COMPONENT_FUNC_REGEX: &str = r"\(func\s+\(;?\d+;?\)\s+\(type\s+\d+\)";

/// A template for the injected types in the core module.
const CORE_INJECTED_TYPES: &str = r"
    (type (;{CORE_TYPE_ID_1};) (func (result i32)))
    (type (;{CORE_TYPE_ID_2};) (func (param i32) (result i64)))
    ";

/// A template for the injected functions in the core module.
const CORE_INJECTED_FUNCTIONS: &str = r"
    (func ${MAGIC}get-memory-size (type {CORE_TYPE_ID_1}) (result i32)
        memory.size
    )
    (func ${MAGIC}get-memory-raw-bytes (type {CORE_TYPE_ID_2}) (param i32) (result i64)
        local.get 0
        i64.load
    )
    ";

/// A template for the injected exports in the core module.
const CORE_INJECTED_EXPORTS: &str = r#"
    (export "{MAGIC}get-memory-size" (func ${MAGIC}get-memory-size))
    (export "{MAGIC}get-memory-raw-bytes" (func ${MAGIC}get-memory-raw-bytes))
    "#;

/// A template for the injected types, functions and exports in the component part.
const COMPONENT_INJECTIONS: &str = r#"
    (type (;{COMPONENT_TYPE_ID_1};) (func (result s32)))
    (alias core export 0 "{MAGIC}get-memory-size" (core func))
    (func (type {COMPONENT_TYPE_ID_1}) (canon lift (core func {COMPONENT_CORE_FUNC_ID_1})))
    (export "{MAGIC}get-memory-size" (func {COMPONENT_FUNC_ID_1}))
    "#;

/// Holds the extracted core module and component part of a WASM.
#[derive(Debug)]
struct WasmInternals {
    /// The core module part of the WASM.
    core_module: String,
    /// The component part of the WASM.
    component_part: String,
}

/// Represents a single match of a WAT element.
struct WatMatch {
    /// The position of the match in the string.
    pos: usize,
    /// The length of the matched string.
    len: usize,
}

/// A matcher for WAT elements, either by exact string or by regex.
enum WatElementMatcher {
    /// Matches an exact string.
    Exact(&'static str),
    /// Matches a regex pattern.
    Regex(Regex),
}

impl From<&'static str> for WatElementMatcher {
    fn from(s: &'static str) -> Self {
        WatElementMatcher::Exact(s)
    }
}

impl From<Regex> for WatElementMatcher {
    fn from(re: Regex) -> Self {
        WatElementMatcher::Regex(re)
    }
}

impl WatElementMatcher {
    /// Finds the first match of the matcher in the given string.
    #[allow(clippy::arithmetic_side_effects)]
    fn first_match<S: AsRef<str>>(
        &self,
        s: S,
    ) -> Option<WatMatch> {
        match self {
            WatElementMatcher::Exact(sub) => {
                s.as_ref().find(sub).map(|pos| {
                    WatMatch {
                        pos,
                        len: sub.len(),
                    }
                })
            },
            WatElementMatcher::Regex(re) => {
                re.find(s.as_ref()).map(|m| {
                    WatMatch {
                        pos: m.start(),
                        len: m.end() - m.start(),
                    }
                })
            },
        }
    }
}

/// Patcher for WASM files.
pub(crate) struct Patcher {
    /// The WAT representation of the WASM file.
    wat: String,
}

impl Patcher {
    /// Creates a new patcher from a file path.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, anyhow::Error> {
        let wat = wasmprinter::print_file(path)?;
        Self::validate_core_module(&wat)?;
        Ok(Self { wat })
    }

    /// Creates a new patcher from a WAT string.
    pub fn from_str<S: AsRef<str>>(wat: S) -> Result<Self, anyhow::Error> {
        let _syntax_check = wat::parse_str(wat.as_ref())?;
        Self::validate_core_module(&wat)?;
        Ok(Self {
            wat: wat.as_ref().to_string(),
        })
    }

    /// Validates that the WAT contains exactly one core module.
    fn validate_core_module<S: AsRef<str>>(wat: S) -> Result<(), anyhow::Error> {
        let core_module_count = Self::get_item_count("(core module (;", &wat)?;
        if core_module_count != 1 {
            return Err(anyhow::anyhow!(
                "expected exactly one core module, found {core_module_count}"
            ));
        }
        Ok(())
    }

    /// Patches the WAT by injecting functions to get memory size and read raw memory
    /// bytes.
    #[allow(clippy::arithmetic_side_effects)]
    pub fn patch(&self) -> Result<String, anyhow::Error> {
        let WasmInternals {
            mut core_module,
            mut component_part,
        } = self.core_and_component()?;

        let next_core_type_index = Self::get_next_core_type_index(&core_module)?;

        let core_type_1_index = next_core_type_index.to_string();
        let core_type_2_index = (next_core_type_index + 1).to_string();

        let core_type_injection = CORE_INJECTED_TYPES
            .replace("{CORE_TYPE_ID_1}", &core_type_1_index)
            .replace("{CORE_TYPE_ID_2}", &core_type_2_index);

        let core_func_injection = CORE_INJECTED_FUNCTIONS
            .replace("{MAGIC}", MAGIC)
            .replace("{CORE_TYPE_ID_1}", &core_type_1_index)
            .replace("{CORE_TYPE_ID_2}", &core_type_2_index);

        let core_export_injection = CORE_INJECTED_EXPORTS.replace("{MAGIC}", MAGIC);

        let next_core_func_index = Self::get_next_core_func_index(&core_module);

        let next_component_type_index = Self::get_next_component_type_index(&component_part)?;
        let component_type_1_index = next_component_type_index.to_string();

        let next_component_core_func_index =
            Self::get_next_component_core_func_index(&component_part)?;
        let component_core_func_1_index = next_component_core_func_index.to_string();

        let next_component_func_index = Self::get_next_component_func_index(&component_part)?;
        let component_func_1_index = (next_component_func_index * 2).to_string(); // *2 because "export" shares the same index space with "func"

        let component_injections = COMPONENT_INJECTIONS
            .replace("{MAGIC}", MAGIC)
            .replace("{COMPONENT_TYPE_ID_1}", &component_type_1_index)
            .replace("{COMPONENT_CORE_FUNC_ID_1}", &component_core_func_1_index)
            .replace("{COMPONENT_FUNC_ID_1}", &component_func_1_index);

        core_module.push_str(&core_type_injection);
        core_module.push_str(&core_func_injection);
        core_module.push_str(&core_export_injection);
        component_part.push_str(&component_injections);

        Ok(format!(
            "
            (component 
                {core_module}
            )
            
            {component_part}
            )"
        ))
    }

    /// Counts the occurrences of a specific element in the given WAT.
    // TODO[RC]: It would be more robust to use a proper parser here (`wasmparser`).
    #[allow(clippy::arithmetic_side_effects)]
    fn get_item_count<I, S>(
        item: I,
        wat: S,
    ) -> Result<u32, anyhow::Error>
    where
        I: Into<WatElementMatcher>,
        S: AsRef<str>,
    {
        let mut start = 0;
        let mut count = 0;

        let matcher: WatElementMatcher = item.into();

        while let Some(WatMatch { pos, len }) = matcher.first_match(
            wat.as_ref()
                .get(start..)
                .ok_or_else(|| anyhow::anyhow!("malformed wat"))?,
        ) {
            count += 1;
            start += pos + len;
        }

        Ok(count)
    }

    /// Gets the next available component type index.
    fn get_next_component_type_index<S: AsRef<str>>(component: S) -> Result<u32, anyhow::Error> {
        Self::get_item_count("type (;", component.as_ref())
    }

    /// Gets the next available core type index.
    fn get_next_core_type_index<S: AsRef<str>>(core_module: S) -> Result<u32, anyhow::Error> {
        Self::get_item_count("type (;", core_module.as_ref())
    }

    /// Gets the next available index of the core function alias in the component part.
    #[allow(clippy::expect_used)] // regex is hardcoded and should be valid
    fn get_next_component_core_func_index<S: AsRef<str>>(
        core_module: S
    ) -> Result<u32, anyhow::Error> {
        Self::get_item_count(
            Regex::new(COMPONENT_CORE_FUNC_REGEX).expect("this should be a proper regex"),
            core_module.as_ref(),
        )
    }

    /// Gets the next available component function index.
    #[allow(clippy::expect_used)] // regex is hardcoded and should be valid
    fn get_next_component_func_index<S: AsRef<str>>(core_module: S) -> Result<u32, anyhow::Error> {
        Self::get_item_count(
            Regex::new(COMPONENT_FUNC_REGEX).expect("this should be a proper regex"),
            core_module.as_ref(),
        )
    }

    /// Gets the next available core function index.
    #[allow(clippy::expect_used)] // regex is hardcoded and should be valid
    fn get_next_core_func_index<S: AsRef<str>>(core_module: S) -> Result<u32, anyhow::Error> {
        Self::get_item_count(
            Regex::new(CORE_FUNC_REGEX).expect("this should be a proper regex"),
            core_module.as_ref(),
        )
    }

    /// Extracts the core module and component part from the WAT.
    #[allow(clippy::arithmetic_side_effects)]
    fn core_and_component(&self) -> Result<WasmInternals, anyhow::Error> {
        let module_start = self
            .wat
            .find("(core module")
            .ok_or_else(|| anyhow::anyhow!("no core module"))?;
        let mut module_end = module_start;

        let mut count = 1;
        for ch in self
            .wat
            .get((module_start + 1)..)
            .ok_or_else(|| anyhow::anyhow!("malformed wat"))?
            .chars()
        {
            module_end += 1;
            if ch == '(' {
                count += 1;
            } else if ch == ')' {
                count -= 1;
                if count == 0 {
                    break;
                }
            }
        }

        let core_module = &self
            .wat
            .get(module_start..=module_end)
            .ok_or_else(|| anyhow::anyhow!("malformed wat"))?;
        let core_last_parenthesis = core_module
            .rfind(')')
            .ok_or_else(|| anyhow::anyhow!("no closing parenthesis in core part"))?;
        let component_part = &self
            .wat
            .get((module_end + 1)..)
            .ok_or_else(|| anyhow::anyhow!("malformed wat"))?;
        let component_last_parenthesis = component_part
            .rfind(')')
            .ok_or_else(|| anyhow::anyhow!("no closing parenthesis in component part"))?;

        Ok(WasmInternals {
            core_module: core_module
                .get(..core_last_parenthesis)
                .ok_or_else(|| anyhow::anyhow!("malformed core module"))?
                .to_string(),
            component_part: component_part
                .get(..component_last_parenthesis)
                .ok_or_else(|| anyhow::anyhow!("malformed component part"))?
                .to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use wasmtime::{
        component::{bindgen, Linker},
        Engine, Store,
    };
    use wasmtime_wasi::{p2::add_to_linker_sync, ResourceTable, WasiCtx, WasiCtxBuilder, WasiView};

    use crate::wasm::patcher::{Patcher, WasmInternals, MAGIC};

    const COMPONENT_SINGLE_CORE_MODULE: &str =
        "tests/test_wasm_files/component_single_core_module.wasm";
    const COMPONENT_MULTIPLE_CORE_MODULES: &str =
        "tests/test_wasm_files/component_multiple_core_modules.wasm";

    const MAKESHIFT_CORRECT_WAT: &str = r#"
        (component
            (core module (;0;)
                (type (;0;) (func))
                (type (;1;) (func (result i32)))
                (type (;2;) (func (param i32 i32) (result i32)))
                (func $two (;1;) (type 1) (result i32)
                    i32.const 2
                )
            )
            (core instance (;0;) (instantiate 0))
            (alias core export 0 "memory" (core memory (;0;)))
            (type (;0;) (func (result u8)))
            (alias core export 0 "two" (core func (;0;)))
            (func (;0;) (type 0) (canon lift (core func 0)))
            (export (;1;) "two" (func 0))
            (@producers
                (processed-by "wit-component" "0.229.0")
            )
        )
    "#;

    const MAKESHIFT_INCORRECT_WAT: &str = r#"
        (component
            (core module (;0;)
                (type (;0;) (func))
                (type (;1;) (func (result i32)))
                (type (;2;) (func (param i32 i32) (result i32)))
                (func $two (;1;) (type 1) (result i32)
                    i32.const 2
    "#;

    fn strip_whitespaces(s: &str) -> String {
        s.chars().filter(|c| !c.is_whitespace()).collect()
    }

    #[test]
    fn builds_from_path() {
        assert!(Patcher::from_file(COMPONENT_SINGLE_CORE_MODULE).is_ok());
    }

    #[test]
    fn builds_from_string() {
        assert!(Patcher::from_str(MAKESHIFT_CORRECT_WAT).is_ok());
    }

    #[test]
    fn fails_on_incorrect_wat() {
        assert!(Patcher::from_str(MAKESHIFT_INCORRECT_WAT).is_err());
    }

    #[test]
    fn extracts_wasm_internals() {
        let patcher = Patcher::from_str(MAKESHIFT_CORRECT_WAT).expect("should create patcher");
        let WasmInternals {
            core_module,
            component_part,
        } = patcher.core_and_component().expect("should extract parts");

        const EXPECTED_CORE: &str = r#"
            (core module (;0;)
                (type (;0;) (func))
                (type (;1;) (func (result i32)))
                (type (;2;) (func (param i32 i32) (result i32)))
                (func $two (;1;) (type 1) (result i32)
                    i32.const 2
                )
            "#;

        const EXPECTED_COMPONENT: &str = r#"
            (core instance (;0;) (instantiate 0))
            (alias core export 0 "memory" (core memory (;0;)))
            (type (;0;) (func (result u8)))
            (alias core export 0 "two" (core func (;0;)))
            (func (;0;) (type 0) (canon lift (core func 0)))
            (export (;1;) "two" (func 0))
            (@producers
                (processed-by "wit-component" "0.229.0")
            )
            "#;

        assert_eq!(
            strip_whitespaces(&core_module),
            strip_whitespaces(EXPECTED_CORE)
        );
        assert_eq!(
            strip_whitespaces(&component_part),
            strip_whitespaces(EXPECTED_COMPONENT)
        );
    }

    #[test]
    fn gets_next_core_type_index() {
        const CORE_1: &str = r#"
            (core module (;0;)
                (func $two (;1;) (type 1) (result i32)
                    i32.const 2
                )
            )
            "#;
        let index = Patcher::get_next_core_type_index(&CORE_1).expect("should get index");
        assert_eq!(index, 0);

        const CORE_2: &str = r#"
            (core module (;0;)
                (type (;0;) (func))
                (type (;1;) (func (result i32)))
                (type (;2;) (func (param i32 i32) (result i32)))
                (func $two (;1;) (type 1) (result i32)
                    i32.const 2
                )
            )
            "#;
        let index = Patcher::get_next_core_type_index(&CORE_2).expect("should get index");
        assert_eq!(index, 3);

        const CORE_3: &str = r#"
            (core module (;0;)
                (type (;0;) (func))
                (type (;1;) (func (result i32)))
                (type (;2;) (func (param i32 i32) (result i32)))
                (type (;3;) (func))
                (type (;4;) (func))
                (type (;5;) (func))
                (type (;6;) (func))
                (func $two (;1;) (type 1) (result i32)
                    i32.const 2
                )
            )
            "#;
        let index = Patcher::get_next_core_type_index(&CORE_3).expect("should get index");
        assert_eq!(index, 7);
    }

    #[test]
    fn gets_next_component_type_index() {
        const COMPONENT_1: &str = r#"
            (core instance (;0;) (instantiate 0))
            (alias core export 0 "memory" (core memory (;0;)))
            (alias core export 0 "two" (core func (;0;)))
            (func (;0;) (type 0) (canon lift (core func 0)))
            (export (;1;) "two" (func 0))
            (@producers
                (processed-by "wit-component" "0.229.0")
            )
            "#;
        let index = Patcher::get_next_component_type_index(&COMPONENT_1).expect("should get index");
        assert_eq!(index, 0);

        const COMPONENT_2: &str = r#"
            (core instance (;0;) (instantiate 0))
            (alias core export 0 "memory" (core memory (;0;)))
            (type (;0;) (func (result u8)))
            (alias core export 0 "two" (core func (;0;)))
            (type (;1;) (func (result u8)))
            (func (;0;) (type 0) (canon lift (core func 0)))
            (export (;1;) "two" (func 0))
            (@producers
                (processed-by "wit-component" "0.229.0")
            )
            "#;
        let index = Patcher::get_next_component_type_index(&COMPONENT_2).expect("should get index");
        assert_eq!(index, 2);

        const COMPONENT_3: &str = r#"
            (core instance (;0;) (instantiate 0))
            (alias core export 0 "memory" (core memory (;0;)))
            (type (;0;) (func (result u8)))
            (alias core export 0 "two" (core func (;0;)))
            (type (;1;) (func (result u8)))
            (func (;0;) (type 0) (canon lift (core func 0)))
            (export (;1;) "two" (func 0))
            (type (;2;) (func (result u8)))
            (type (;3;) (func (result u8)))
            (type (;4;) (func (result u8)))
            (@producers
                (processed-by "wit-component" "0.229.0")
            )
            "#;
        let index = Patcher::get_next_component_type_index(&COMPONENT_3).expect("should get index");
        assert_eq!(index, 5);
    }

    #[test]
    fn gets_next_core_func_index() {
        const CORE_1: &str = r#"
            (core module (;0;)
                (type (;0;) (func))
                (type (;1;) (func (result i32)))
            )
            "#;
        let index = Patcher::get_next_core_func_index(&CORE_1).expect("should get index");
        assert_eq!(index, 0);

        const CORE_2: &str = r#"
            (core module (;0;)
                (type (;0;) (func))
                (type (;1;) (func (result i32)))
                (type (;2;) (func (param i32 i32) (result i32)))
                (func $two (;1;) (type 1) (result i32)
                    i32.const 2
                )
            )
            "#;
        let index = Patcher::get_next_core_func_index(&CORE_2).expect("should get index");
        assert_eq!(index, 1);

        const CORE_3: &str = r#"
            (core module (;0;)
                (type (;0;) (func))
                (type (;1;) (func (result i32)))
                (type (;2;) (func (param i32 i32) (result i32)))
                (type (;3;) (func))
                (type (;4;) (func))
                (type (;5;) (func))
                (type (;6;) (func))
                (func $two1 (;1;) (type 1) (result i32)
                    i32.const 2
                )
                (func $two2 (;2;) (type 1) (result i32)
                    i32.const 2
                )
                (func $two3 (;3;) (type 1) (result i32)
                    i32.const 2
                )
                (func $two4 (;4;) (type 1) (result i32)
                    i32.const 2
                )
            )
            "#;
        let index = Patcher::get_next_core_func_index(&CORE_3).expect("should get index");
        assert_eq!(index, 4);
    }

    #[test]
    fn gets_next_component_func_index() {
        const COMPONENT_1: &str = r#"
            (core instance (;0;) (instantiate 0))
            (alias core export 0 "memory" (core memory (;0;)))
            (type (;0;) (func (result u8)))
            (func (;0;) (type 0) (canon lift (core func 0)))
            (func (;1;) (type 0) (canon lift (core func 0)))
            (func (;2;) (type 0) (canon lift (core func 0)))
            (export (;1;) "two" (func 0))
            (@producers
                (processed-by "wit-component" "0.229.0")
            )
            "#;
        let index =
            Patcher::get_next_component_core_func_index(&COMPONENT_1).expect("should get index");
        assert_eq!(index, 0);

        const COMPONENT_2: &str = r#"
            (core instance (;0;) (instantiate 0))
            (alias core export 0 "memory" (core memory (;0;)))
            (type (;0;) (func (result u8)))
            (alias core export 0 "two" (core func (;0;)))
            (func (;0;) (type 0) (canon lift (core func 0)))
            (func (;1;) (type 0) (canon lift (core func 0)))
            (func (;2;) (type 0) (canon lift (core func 0)))
            (export (;1;) "two" (func 0))
            (@producers
                (processed-by "wit-component" "0.229.0")
            )
            "#;
        let index =
            Patcher::get_next_component_core_func_index(&COMPONENT_2).expect("should get index");
        assert_eq!(index, 1);

        const COMPONENT_3: &str = r#"
            (core instance (;0;) (instantiate 0))
            (alias core export 0 "memory" (core memory (;0;)))
            (type (;0;) (func (result u8)))
            (alias core export 0 "two" (core func (;0;)))
            (func (;0;) (type 0) (canon lift (core func 0)))
            (func (;1;) (type 0) (canon lift (core func 0)))
            (func (;2;) (type 0) (canon lift (core func 0)))
            (alias core export 0 "three" (core func (;0;)))
            (alias core export 0 "four" (core func (;0;)))
            (alias core export 0 "five" (core func (;0;)))
            (export (;1;) "two" (func 0))
            (@producers
                (processed-by "wit-component" "0.229.0")
            )
            "#;
        let index =
            Patcher::get_next_component_core_func_index(&COMPONENT_3).expect("should get index");
        assert_eq!(index, 4);
    }

    #[test]
    fn patched_wat_can_be_encoded() {
        let patcher = Patcher::from_str(MAKESHIFT_CORRECT_WAT).expect("should create patcher");
        let result = patcher.patch().expect("should patch");
        let encoded = wat::parse_str(&result);
        assert!(encoded.is_ok());

        let patcher =
            Patcher::from_file(COMPONENT_SINGLE_CORE_MODULE).expect("should create patcher");
        let result = patcher.patch().expect("should patch");
        let encoded = wat::parse_str(&result);
        assert!(encoded.is_ok());
    }

    #[test]
    fn injected_get_memory_size_works() {
        // Step 1: Patch the WASM file
        let patcher =
            Patcher::from_file(COMPONENT_SINGLE_CORE_MODULE).expect("should create patcher");
        let result = patcher.patch().expect("should patch");
        let encoded = wat::parse_str(&result).expect("should encode");

        // Step 2: Instantiate the patched WASM
        struct MyCtx {
            table: ResourceTable,
            wasi: WasiCtx,
        }

        impl WasiView for MyCtx {
            fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
                wasmtime_wasi::WasiCtxView {
                    ctx: &mut self.wasi,
                    table: &mut self.table,
                }
            }
        }

        let engine = Engine::default();
        let component =
            wasmtime::component::Component::new(&engine, encoded).expect("should create component");
        let mut linker = Linker::new(&engine);
        add_to_linker_sync(&mut linker).expect("should add to linker");
        let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_env().build();
        let ctx = MyCtx {
            table: ResourceTable::new(),
            wasi,
        };
        let mut store = Store::new(&engine, ctx);
        let instance = linker
            .instantiate(&mut store, &component)
            .expect("should instantiate");

        // Step 3: Call the injected function
        let get_memory_size_func = format!("{}get-memory-size", MAGIC);

        let get_memory_size = instance
            .get_func(&mut store, get_memory_size_func)
            .expect("should get func")
            .typed::<(), (i32,)>(&store)
            .expect("should be a typed func");
        let memory_size_in_pages = get_memory_size.call(&mut store, ()).expect("should call").0;
        get_memory_size
            .post_return(&mut store)
            .expect("should post return");

        // Step 4: Check if the returned value matches the original WASM memory size
        let source_wat =
            wasmprinter::print_file(COMPONENT_SINGLE_CORE_MODULE).expect("should read");
        let expected_memory_entry = format!("(memory (;0;) {})", memory_size_in_pages);

        assert!(source_wat.contains(&expected_memory_entry));
    }

    #[test]
    fn incorrect_wasm_returns_error() {
        let patcher = Patcher::from_file(COMPONENT_MULTIPLE_CORE_MODULES);
        assert!(patcher.is_err());
    }
}
