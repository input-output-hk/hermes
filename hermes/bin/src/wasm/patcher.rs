//! Patcher for WASM files.
//! It injects functions to get memory size and read raw memory bytes into the WASM file.

// TODO[RC]: Patcher is not yet wired into the Hermes.
#![allow(unused)]

use std::path::Path;

use regex::Regex;

/// Magic string to avoid name collisions with existing functions.
// cspell:disable-next-line
const MAGIC: &str = r"vmucqq2137emxpatzkmuyy1szcpx23lp-hermes-";

/// Regex to detect the function definitions in the core module.
const CORE_FUNC_REGEX: &str = r"\(func\s+\$[^\s()]+[^)]*\(;";

/// Regex to detect the aliases of core functions in the component part.
const COMPONENT_CORE_FUNC_REGEX: &str = r#"\(alias\s+core\s+export\s+0\s+"[^"]+"\s+\(core\s+func"#;

/// Regex to detect the function definitions in the component part.
const COMPONENT_FUNC_REGEX: &str = r"\(func\s+\(;?\d+;?\)\s+\(type\s+\d+\) \(canon";

/// A template for the injected types in the core module.
const CORE_INJECTED_TYPES: &str = r"
    (type (;{CORE_TYPE_ID_1};) (func (result i32)))
    (type (;{CORE_TYPE_ID_2};) (func (param i32) (result i64)))
    (type (;{CORE_TYPE_ID_3};) (func (param i32 i64)))
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
    (func ${MAGIC}set-memory-raw-bytes (type {CORE_TYPE_ID_3}) (param i32 i64)
      local.get 0
      local.get 1
      i64.store
    )
    ";

/// A template for the injected exports in the core module.
const CORE_INJECTED_EXPORTS: &str = r#"
    (export "{MAGIC}get-memory-size" (func ${MAGIC}get-memory-size))
    (export "{MAGIC}get-memory-raw-bytes" (func ${MAGIC}get-memory-raw-bytes))
    (export "{MAGIC}set-memory-raw-bytes" (func ${MAGIC}set-memory-raw-bytes))
    "#;

/// A template for the injected types, functions and exports in the component part.
const COMPONENT_INJECTIONS: &str = r#"
    (type (;{COMPONENT_TYPE_ID_1};) (func (result u32)))
    (alias core export 0 "{MAGIC}get-memory-size" (core func))
    (func (type {COMPONENT_TYPE_ID_1}) (canon lift (core func {COMPONENT_CORE_FUNC_ID_1})))
    (export "{MAGIC}get-memory-size" (func {COMPONENT_FUNC_ID_1}))

    (type (;{COMPONENT_TYPE_ID_2};) (func (param "val" u32) (result s64)))
    (alias core export 0 "{MAGIC}get-memory-raw-bytes" (core func))
    (func (type {COMPONENT_TYPE_ID_2}) (canon lift (core func {COMPONENT_CORE_FUNC_ID_2})))
    (export "{MAGIC}get-memory-raw-bytes" (func {COMPONENT_FUNC_ID_2}))

    (type (;{COMPONENT_TYPE_ID_3};) (func (param "val" u32) (param "val2" s64)))
    (alias core export 0 "{MAGIC}set-memory-raw-bytes" (core func))
    (func (type {COMPONENT_TYPE_ID_3}) (canon lift (core func {COMPONENT_CORE_FUNC_ID_3})))
    (export "{MAGIC}set-memory-raw-bytes" (func {COMPONENT_FUNC_ID_3}))
    "#;

/// Holds the extracted core module and component part of a WASM.
#[derive(Debug)]
struct WasmInternals {
    /// The core module part of the WASM.
    core_module: String,
    /// The component part of the WASM.
    component_part: String,
    /// The part of component specified before the core module.
    pre_core_component_part: String,
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
            mut pre_core_component_part,
        } = self.core_and_component()?;

        let next_core_type_index = Self::get_next_core_type_index(&core_module)?;

        let core_type_1_index = next_core_type_index.to_string();
        let core_type_2_index = (next_core_type_index + 1).to_string();
        let core_type_3_index = (next_core_type_index + 2).to_string();

        let core_type_injection = CORE_INJECTED_TYPES
            .replace("{CORE_TYPE_ID_1}", &core_type_1_index)
            .replace("{CORE_TYPE_ID_2}", &core_type_2_index)
            .replace("{CORE_TYPE_ID_3}", &core_type_3_index);

        let core_func_injection = CORE_INJECTED_FUNCTIONS
            .replace("{MAGIC}", MAGIC)
            .replace("{CORE_TYPE_ID_1}", &core_type_1_index)
            .replace("{CORE_TYPE_ID_2}", &core_type_2_index)
            .replace("{CORE_TYPE_ID_3}", &core_type_3_index);

        let core_export_injection = CORE_INJECTED_EXPORTS.replace("{MAGIC}", MAGIC);

        let next_core_func_index = Self::get_next_core_func_index(&core_module);

        let next_component_type_index =
            Self::get_next_component_type_index(&component_part, &pre_core_component_part)?;
        let component_type_1_index = next_component_type_index.to_string();
        let component_type_2_index = (next_component_type_index + 1).to_string();
        let component_type_3_index = (next_component_type_index + 2).to_string();

        let next_component_core_func_index =
            Self::get_next_component_core_func_index(&component_part)?;
        let component_core_func_1_index = next_component_core_func_index.to_string();
        let component_core_func_2_index = (next_component_core_func_index + 1).to_string();
        let component_core_func_3_index = (next_component_core_func_index + 2).to_string();

        let next_component_func_index = Self::get_next_component_func_index(&component_part)?;
        // *2 because "export" shares the same index space with "func"
        let component_func_1_index = (next_component_func_index * 2).to_string();
        let component_func_2_index = ((next_component_func_index + 1) * 2).to_string();
        let component_func_3_index = ((next_component_func_index + 2) * 2).to_string();

        let component_injections = COMPONENT_INJECTIONS
            .replace("{MAGIC}", MAGIC)
            .replace("{COMPONENT_TYPE_ID_1}", &component_type_1_index)
            .replace("{COMPONENT_CORE_FUNC_ID_1}", &component_core_func_1_index)
            .replace("{COMPONENT_FUNC_ID_1}", &component_func_1_index)
            .replace("{COMPONENT_TYPE_ID_2}", &component_type_2_index)
            .replace("{COMPONENT_CORE_FUNC_ID_2}", &component_core_func_2_index)
            .replace("{COMPONENT_FUNC_ID_2}", &component_func_2_index)
            .replace("{COMPONENT_TYPE_ID_3}", &component_type_3_index)
            .replace("{COMPONENT_CORE_FUNC_ID_3}", &component_core_func_3_index)
            .replace("{COMPONENT_FUNC_ID_3}", &component_func_3_index);

        core_module.push_str(&core_type_injection);
        core_module.push_str(&core_func_injection);
        core_module.push_str(&core_export_injection);
        component_part.push_str(&component_injections);

        let patched_wat = format!(
            "
            (component 
                {pre_core_component_part}
                {core_module}
            )
            
            {component_part}
            )"
        );
        Ok(patched_wat)
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
    fn get_next_component_type_index<S: AsRef<str>>(
        component: S,
        pre_core_component: S,
    ) -> Result<u32, anyhow::Error> {
        let mut processed_component = component.as_ref().to_string();
        while let Some(inner_component_start) = processed_component.find("(component") {
            let inner_component_end =
                Self::parse_until_section_end(inner_component_start, &processed_component)? + 1;
            processed_component.replace_range(inner_component_start..inner_component_end, "---");
        }

        println!("processed_component: {processed_component}");

        let mut processed_pre_component = pre_core_component.as_ref().to_string();
        while let Some(inner_instance_start) = processed_pre_component.find("(instance") {
            let inner_instance_end =
                Self::parse_until_section_end(inner_instance_start, &processed_pre_component)? + 1;
            processed_pre_component.replace_range(inner_instance_start..inner_instance_end, "---");
        }

        Ok(Self::get_item_count("type (;", processed_component)?
            + Self::get_item_count("type (;", processed_pre_component)?)
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

    fn parse_until_section_end<S: AsRef<str>>(
        start: usize,
        wat: S,
    ) -> Result<usize, anyhow::Error> {
        let mut end = start;
        let mut count = 1;
        for ch in wat
            .as_ref()
            .get((start + 1)..)
            .ok_or_else(|| anyhow::anyhow!("malformed wat"))?
            .chars()
        {
            end += 1;
            if ch == '(' {
                count += 1;
            } else if ch == ')' {
                count -= 1;
                if count == 0 {
                    break;
                }
            }
        }
        Ok(end)
    }

    /// Extracts the core module and component part from the WAT.
    #[allow(clippy::arithmetic_side_effects)]
    fn core_and_component(&self) -> Result<WasmInternals, anyhow::Error> {
        const COMPONENT_ITEM: &str = "(component";
        let module_start = self
            .wat
            .find("(core module")
            .ok_or_else(|| anyhow::anyhow!("no core module"))?;
        let mut module_end = Self::parse_until_section_end(module_start, &self.wat)?;

        let pre_component_str = self
            .wat
            .get(0..module_start)
            .ok_or_else(|| anyhow::anyhow!("malformed wat"))?
            .trim();
        let pre_core_component_part = if pre_component_str == "(component" {
            ""
        } else {
            let component_start = self
                .wat
                .find(COMPONENT_ITEM)
                .ok_or_else(|| anyhow::anyhow!("no component start"))?;
            self.wat
                .get((component_start + COMPONENT_ITEM.len())..module_start)
                .ok_or_else(|| anyhow::anyhow!("malformed wat"))?
        };

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
            pre_core_component_part: pre_core_component_part.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use wasmtime::{
        component::{bindgen, Instance, Linker},
        AsContextMut, Engine, Store,
    };
    use wasmtime_wasi::{p2::add_to_linker_sync, ResourceTable, WasiCtx, WasiCtxBuilder, WasiView};

    use crate::wasm::patcher::{Patcher, WasmInternals, MAGIC};

    const LINEAR_MEMORY_PAGE_SIZE_BYTES: u32 = 65536;

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

    const MAKESHIFT_CORRECT_WAT_WITH_PRE_CORE_COMPONENT: &str = r#"
        (component
            (type (;0;)
              (instance
                (type (;0;) string)
                (export (;1;) "cron-event-tag" (type (eq 0)))
                (type (;2;) string)
                (export (;3;) "cron-sched" (type (eq 2)))
                (type (;4;) (record (field "when" 3) (field "tag" 1)))
                (export (;5;) "cron-tagged" (type (eq 4)))
              )
            )
            (import "hermes:cron/api" (instance (;0;) (type 0)))
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

    const MAKESHIFT_INCORRECT_WAT: &str = r"
        (component
            (core module (;0;)
                (type (;0;) (func))
                (type (;1;) (func (result i32)))
                (type (;2;) (func (param i32 i32) (result i32)))
                (func $two (;1;) (type 1) (result i32)
                    i32.const 2
    ";

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
    fn extracts_wasm_internals_no_precore() {
        const EXPECTED_CORE: &str = r"
            (core module (;0;)
                (type (;0;) (func))
                (type (;1;) (func (result i32)))
                (type (;2;) (func (param i32 i32) (result i32)))
                (func $two (;1;) (type 1) (result i32)
                    i32.const 2
                )
            ";

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

        const EXPECTED_PRE_COMPONENT: &str = "";

        let patcher = Patcher::from_str(MAKESHIFT_CORRECT_WAT).expect("should create patcher");
        let WasmInternals {
            core_module,
            component_part,
            pre_core_component_part,
        } = patcher.core_and_component().expect("should extract parts");

        assert_eq!(
            strip_whitespaces(&core_module),
            strip_whitespaces(EXPECTED_CORE)
        );
        assert_eq!(
            strip_whitespaces(&component_part),
            strip_whitespaces(EXPECTED_COMPONENT)
        );
        assert_eq!(
            strip_whitespaces(&pre_core_component_part),
            strip_whitespaces(EXPECTED_PRE_COMPONENT)
        );
    }

    #[test]
    fn types_from_precore_are_included_when_patching() {
        let patcher = Patcher::from_str(MAKESHIFT_CORRECT_WAT_WITH_PRE_CORE_COMPONENT)
            .expect("should create patcher");
        let WasmInternals {
            core_module,
            component_part,
            pre_core_component_part,
        } = patcher.core_and_component().expect("should extract parts");

        println!("core_module: {core_module}");
        println!("component_part: {component_part}");
        println!("pre_core_component_part: {pre_core_component_part}");

        let patched_wat = patcher.patch().expect("should patch wat");
        println!("patched_wat: {patched_wat}");
    }

    #[test]
    fn extracts_wasm_internals_with_precore() {
        const EXPECTED_CORE: &str = r"
            (core module (;0;)
                (type (;0;) (func))
                (type (;1;) (func (result i32)))
                (type (;2;) (func (param i32 i32) (result i32)))
                (func $two (;1;) (type 1) (result i32)
                    i32.const 2
                )
            ";

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

        const EXPECTED_PRE_COMPONENT: &str = r#"
            (type (;0;)
              (instance
                (type (;0;) string)
                (export (;1;) "cron-event-tag" (type (eq 0)))
                (type (;2;) string)
                (export (;3;) "cron-sched" (type (eq 2)))
                (type (;4;) (record (field "when" 3) (field "tag" 1)))
                (export (;5;) "cron-tagged" (type (eq 4)))
              )
            )
            (import "hermes:cron/api" (instance (;0;) (type 0)))
        "#;

        let patcher = Patcher::from_str(MAKESHIFT_CORRECT_WAT_WITH_PRE_CORE_COMPONENT)
            .expect("should create patcher");
        let WasmInternals {
            core_module,
            component_part,
            pre_core_component_part,
        } = patcher.core_and_component().expect("should extract parts");

        assert_eq!(
            strip_whitespaces(&core_module),
            strip_whitespaces(EXPECTED_CORE)
        );
        assert_eq!(
            strip_whitespaces(&component_part),
            strip_whitespaces(EXPECTED_COMPONENT)
        );
        assert_eq!(
            strip_whitespaces(&pre_core_component_part),
            strip_whitespaces(EXPECTED_PRE_COMPONENT)
        );
    }

    #[test]
    fn gets_next_core_type_index() {
        const CORE_1: &str = r"
            (core module (;0;)
                (func $two (;1;) (type 1) (result i32)
                    i32.const 2
                )
            )
            ";

        const CORE_2: &str = r"
            (core module (;0;)
                (type (;0;) (func))
                (type (;1;) (func (result i32)))
                (type (;2;) (func (param i32 i32) (result i32)))
                (func $two (;1;) (type 1) (result i32)
                    i32.const 2
                )
            )
            ";

        const CORE_3: &str = r"
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
            ";

        let index = Patcher::get_next_core_type_index(CORE_1).expect("should get index");
        assert_eq!(index, 0);

        let index = Patcher::get_next_core_type_index(CORE_2).expect("should get index");
        assert_eq!(index, 3);

        let index = Patcher::get_next_core_type_index(CORE_3).expect("should get index");
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

        let index =
            Patcher::get_next_component_type_index(COMPONENT_1, "").expect("should get index");
        assert_eq!(index, 0);

        let index =
            Patcher::get_next_component_type_index(COMPONENT_2, "").expect("should get index");
        assert_eq!(index, 2);

        let index =
            Patcher::get_next_component_type_index(COMPONENT_3, "").expect("should get index");
        assert_eq!(index, 5);
    }

    #[test]
    fn gets_next_core_func_index() {
        const CORE_1: &str = r"
            (core module (;0;)
                (type (;0;) (func))
                (type (;1;) (func (result i32)))
            )
            ";

        const CORE_2: &str = r"
            (core module (;0;)
                (type (;0;) (func))
                (type (;1;) (func (result i32)))
                (type (;2;) (func (param i32 i32) (result i32)))
                (func $two (;1;) (type 1) (result i32)
                    i32.const 2
                )
            )
            ";

        const CORE_3: &str = r"
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
            ";

        let index = Patcher::get_next_core_func_index(CORE_1).expect("should get index");
        assert_eq!(index, 0);

        let index = Patcher::get_next_core_func_index(CORE_2).expect("should get index");
        assert_eq!(index, 1);

        let index = Patcher::get_next_core_func_index(CORE_3).expect("should get index");
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
            Patcher::get_next_component_core_func_index(COMPONENT_1).expect("should get index");
        assert_eq!(index, 0);

        let index =
            Patcher::get_next_component_core_func_index(COMPONENT_2).expect("should get index");
        assert_eq!(index, 1);

        let index =
            Patcher::get_next_component_core_func_index(COMPONENT_3).expect("should get index");
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
        let get_memory_size_func = format!("{MAGIC}get-memory-size");
        let get_memory_size = instance
            .get_func(&mut store, get_memory_size_func)
            .expect("should get func")
            .typed::<(), (u32,)>(&store)
            .expect("should be a typed func");
        let memory_size_in_pages = get_memory_size.call(&mut store, ()).expect("should call").0;
        get_memory_size
            .post_return(&mut store)
            .expect("should post return");

        // Step 4: Check if the returned value matches the original WASM memory size
        let source_wat =
            wasmprinter::print_file(COMPONENT_SINGLE_CORE_MODULE).expect("should read");
        let expected_memory_entry = format!("(memory (;0;) {memory_size_in_pages})");

        assert!(source_wat.contains(&expected_memory_entry));
    }

    #[test]
    fn injected_get_memory_raw_bytes_works() {
        // Step 1: Patch the WASM file
        let patcher =
            Patcher::from_file(COMPONENT_SINGLE_CORE_MODULE).expect("should create patcher");
        let result = patcher.patch().expect("should patch");
        let encoded = wat::parse_str(&result).expect("should encode");

        // Step 2: Instantiate the patched WASM
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

        // Step 3: Call the init function which will put some bytes into the linear memory.
        let init = instance
            .get_func(&mut store, "init")
            .expect("should get func")
            .typed::<(), (bool,)>(&store)
            .expect("should be a typed func");
        let init_result = init.call(&mut store, ()).expect("should call").0;
        init.post_return(&mut store).expect("should post return");
        assert!(init_result);

        // Step 4: Retrieve the linear memory size
        let get_memory_size_func = format!("{MAGIC}get-memory-size");
        let get_memory_size = instance
            .get_func(&mut store, get_memory_size_func)
            .expect("should get func")
            .typed::<(), (u32,)>(&store)
            .expect("should be a typed func");
        let memory_size_in_pages = get_memory_size.call(&mut store, ()).expect("should call").0;
        get_memory_size
            .post_return(&mut store)
            .expect("should post return");
        let memory_size_in_bytes = memory_size_in_pages * LINEAR_MEMORY_PAGE_SIZE_BYTES;

        // Step 5: Read the entire memory content
        let mut linear_memory = vec![];
        let get_memory_raw_bytes_func = format!("{MAGIC}get-memory-raw-bytes");
        let get_memory_raw_bytes = instance
            .get_func(&mut store, get_memory_raw_bytes_func)
            .expect("should get func")
            .typed::<(u32,), (i64,)>(&store)
            .expect("should be a typed func");
        for offset in (0u32..memory_size_in_bytes).step_by(8) {
            let raw_bytes = get_memory_raw_bytes
                .call(&mut store, (offset,))
                .expect("should call")
                .0;
            get_memory_raw_bytes
                .post_return(&mut store)
                .expect("should post return");
            // In WASM all values are read and written in little endian byte order
            // See: https://www.w3.org/TR/2019/REC-wasm-core-1-20191205/#memory-instructions
            linear_memory.extend(raw_bytes.to_le_bytes());
        }

        // Step 6: Check if the expected pattern is present in the linear memory.
        // The test component fills 1kb of memory with the pattern 0xAA, 0xBB, 0xCC, 0xDD, 0xAA,
        // 0xBB, 0xCC, 0xDD, ... This patterns starts at the 0x100004 offset, but since
        // this location is not fixed across compilations, we just check if the pattern is
        // present anywhere in the memory.
        let expected_pattern: Vec<u8> = std::iter::repeat_n([0xAA, 0xBB, 0xCC, 0xDD], 1024 / 4)
            .flatten()
            .collect();
        assert!(linear_memory
            .windows(1024)
            .any(|window| window == expected_pattern));
    }

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

    fn get_bytes_at_offset(
        instance: &Instance,
        mut store: impl AsContextMut,
        offset: u32,
    ) -> i64 {
        let get_memory_raw_bytes_func = format!("{MAGIC}get-memory-raw-bytes");
        let get_memory_raw_bytes = instance
            .get_func(&mut store, get_memory_raw_bytes_func)
            .expect("should get func")
            .typed::<(u32,), (i64,)>(&store)
            .expect("should be a typed func");
        let raw_bytes = get_memory_raw_bytes
            .call(&mut store, (offset,))
            .expect("should call")
            .0;
        get_memory_raw_bytes
            .post_return(store)
            .expect("should post return");
        raw_bytes
    }

    #[test]
    fn injected_set_memory_raw_bytes_works() {
        const OFFSET: u32 = 0xF000;

        // Step 1: Patch the WASM file
        let patcher =
            Patcher::from_file(COMPONENT_SINGLE_CORE_MODULE).expect("should create patcher");
        let result = patcher.patch().expect("should patch");
        let encoded = wat::parse_str(&result).expect("should encode");

        // Step 2: Instantiate the patched WASM
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

        let old_bytes_pre_offset = get_bytes_at_offset(&instance, &mut store, OFFSET - 8);
        let old_bytes_at_offset = get_bytes_at_offset(&instance, &mut store, OFFSET);
        let old_bytes_post_offset = get_bytes_at_offset(&instance, &mut store, OFFSET + 8);

        // Step 3: Set bytes at a specific offset
        let bytes: i64 = 0x1122_3344_5566_7788;
        let set_memory_raw_bytes_func = format!("{MAGIC}set-memory-raw-bytes");
        let set_memory_raw_bytes = instance
            .get_func(&mut store, set_memory_raw_bytes_func)
            .expect("should get func")
            .typed::<(u32, i64), ()>(&store)
            .expect("should be a typed func");
        set_memory_raw_bytes
            .call(&mut store, (OFFSET, bytes))
            .expect("should call");
        set_memory_raw_bytes
            .post_return(&mut store)
            .expect("should post return");

        // Step 4: Retrieve bytes before-, on- and after-offset.
        // Check that only the bytes at the offset match.
        let new_bytes_pre_offset = get_bytes_at_offset(&instance, &mut store, OFFSET - 8);
        let new_bytes_at_offset = get_bytes_at_offset(&instance, &mut store, OFFSET);
        let new_bytes_post_offset = get_bytes_at_offset(&instance, &mut store, OFFSET + 8);

        assert_eq!(old_bytes_pre_offset, new_bytes_pre_offset);
        assert_eq!(old_bytes_post_offset, new_bytes_post_offset);
        assert_ne!(old_bytes_at_offset, bytes);
        assert_eq!(new_bytes_at_offset, bytes);
    }

    #[test]
    fn component_part_with_nested_component_is_properly_patched() {
        const COMPONENT_WITH_INNER_COMPONENT_PART: &str = r#"
            (core instance (;0;) (instantiate 0))
            (alias core export 0 "memory" (core memory (;0;)))
            (type (;0;) (func (result bool)))
            (alias core export 0 "hermes:init/event#init" (core func (;0;)))
            (alias core export 0 "cabi_realloc" (core func (;1;)))
            (func (;0;) (type 0) (canon lift (core func 0)))
            (component (;0;)
                (type (;0;) (func (result bool)))
                (import "import-func-init" (func (;0;) (type 0)))
                (type (;1;) (func (result bool)))
                (export (;1;) "init" (func 0) (func (type 1)))
            )
            (instance (;0;) (instantiate 0
                (with "import-func-init" (func 0))
                )
            )
            (export (;1;) "hermes:init/event" (instance 0))
            (@producers
                (processed-by "wit-component" "0.229.0")
            )
        "#;

        const COMPONENT_WITH_TWO_INNER_COMPONENT_PARTS: &str = r#"
            (alias export 0 "cron-event-tag" (type (;1;)))
            (alias export 0 "cron-tagged" (type (;2;)))
            (core instance (;0;) (instantiate 0))
            (alias core export 0 "memory" (core memory (;0;)))
            (type (;3;) (func (result bool)))
            (alias core export 0 "hermes:init/event#init" (core func (;0;)))
            (alias core export 0 "cabi_realloc" (core func (;1;)))
            (func (;0;) (type 3) (canon lift (core func 0)))
            (component (;0;)
              (type (;0;) (func (result bool)))
              (import "import-func-init" (func (;0;) (type 0)))
              (type (;1;) (func (result bool)))
              (export (;1;) "init" (func 0) (func (type 1)))
            )
            (instance (;1;) (instantiate 0
                (with "import-func-init" (func 0))
              )
            )
            (export (;2;) "hermes:init/event" (instance 1))
            (type (;4;) (func (param "event" 2) (param "last" bool) (result bool)))
            (alias core export 0 "hermes:cron/event#on-cron" (core func (;2;)))
            (func (;1;) (type 4) (canon lift (core func 2) (memory 0) (realloc 1) string-encoding=utf8))
            (alias export 0 "cron-event-tag" (type (;5;)))
            (alias export 0 "cron-sched" (type (;6;)))
            (alias export 0 "cron-tagged" (type (;7;)))
            (component (;1;)
              (type (;0;) string)
              (import "import-type-cron-event-tag" (type (;1;) (eq 0)))
              (type (;2;) string)
              (import "import-type-cron-sched" (type (;3;) (eq 2)))
              (type (;4;) (record (field "when" 3) (field "tag" 1)))
              (import "import-type-cron-tagged" (type (;5;) (eq 4)))
              (import "import-type-cron-tagged0" (type (;6;) (eq 5)))
              (type (;7;) (func (param "event" 6) (param "last" bool) (result bool)))
              (import "import-func-on-cron" (func (;0;) (type 7)))
              (export (;8;) "cron-event-tag" (type 1))
              (export (;9;) "cron-tagged" (type 5))
              (type (;10;) (func (param "event" 9) (param "last" bool) (result bool)))
              (export (;1;) "on-cron" (func 0) (func (type 10)))
            )
            (instance (;3;) (instantiate 1
                (with "import-func-on-cron" (func 1))
                (with "import-type-cron-event-tag" (type 5))
                (with "import-type-cron-sched" (type 6))
                (with "import-type-cron-tagged" (type 7))
                (with "import-type-cron-tagged0" (type 2))
              )
            )
            (export (;4;) "hermes:cron/event" (instance 3))
            (@producers
              (processed-by "wit-component" "0.229.0")
            )
        "#;

        const COMPONENT_WITHOUT_INNER_COMPONENT_PART: &str = r#"
            (core instance (;0;) (instantiate 0))
            (alias core export 0 "memory" (core memory (;0;)))
            (type (;0;) (func (result bool)))
            (alias core export 0 "hermes:init/event#init" (core func (;0;)))
            (alias core export 0 "cabi_realloc" (core func (;1;)))
            (func (;0;) (type 0) (canon lift (core func 0)))
            (instance (;0;) (instantiate 0
                (with "import-func-init" (func 0))
                )
            )
            (export (;1;) "hermes:init/event" (instance 0))
            (@producers
                (processed-by "wit-component" "0.229.0")
            )
        "#;

        let next_component_type_index =
            Patcher::get_next_component_type_index(COMPONENT_WITH_INNER_COMPONENT_PART, "")
                .expect("should get index");
        assert_eq!(next_component_type_index, 1);

        let next_component_type_index =
            Patcher::get_next_component_type_index(COMPONENT_WITH_TWO_INNER_COMPONENT_PARTS, "")
                .expect("should get index");
        assert_eq!(next_component_type_index, 7);

        let next_component_type_index =
            Patcher::get_next_component_type_index(COMPONENT_WITHOUT_INNER_COMPONENT_PART, "")
                .expect("should get index");
        assert_eq!(next_component_type_index, 1);
    }

    #[test]
    fn incorrect_wasm_returns_error() {
        let patcher = Patcher::from_file(COMPONENT_MULTIPLE_CORE_MODULES);
        assert!(patcher.is_err());
    }

    #[test]
    fn patching_hermes_like_wat() {
        const HERMES_LIKE_WAT: &str = r#"
(component
  (type (;0;)
    (instance
      (type (;0;) string)
      (export (;1;) "cron-event-tag" (type (eq 0)))
      (type (;2;) string)
      (export (;3;) "cron-sched" (type (eq 2)))
      (type (;4;) (record (field "when" 3) (field "tag" 1)))
      (export (;5;) "cron-tagged" (type (eq 4)))
    )
  )
  (import "hermes:cron/api" (instance (;0;) (type 0)))
  (core module (;0;)
    (type (;0;) (func))
    (type (;1;) (func (result i32)))
    (type (;2;) (func (param i32 i32 i32 i32 i32) (result i32)))
    (type (;3;) (func (param i32 i32 i32 i32) (result i32)))
    (type (;4;) (func (param i32) (result i32)))
    (type (;5;) (func (param i32 i32 i32) (result i32)))
    (type (;6;) (func (param i32)))
    (type (;7;) (func (param i32 i32) (result i32)))
    (type (;8;) (func (param i32 i32)))
    (table (;0;) 3 3 funcref)
    (memory (;0;) 17)
    (global $__stack_pointer (;0;) (mut i32) i32.const 1048576)
    (global $GOT.data.internal.__memory_base (;1;) i32 i32.const 0)
    (export "memory" (memory 0))
    (export "hermes:init/event#init" (func $hermes:init/event#init))
    (export "hermes:cron/event#on-cron" (func $hermes:cron/event#on-cron))
    (export "cabi_realloc" (func $cabi_realloc))
    (elem (;0;) (i32.const 1) func $_ZN15memory_snapshot8bindings40__link_custom_section_describing_imports17h7006a9ae6faf0e0aE $cabi_realloc)
    (func $__wasm_call_ctors (;0;) (type 0))
    (func $hermes:init/event#init (;1;) (type 1) (result i32)
      (local i32)
      block ;; label = @1
        global.get $GOT.data.internal.__memory_base
        i32.const 1048600
        i32.add
        i32.load8_u
        br_if 0 (;@1;)
        global.get $GOT.data.internal.__memory_base
        local.set 0
        call $__wasm_call_ctors
        local.get 0
        i32.const 1048600
        i32.add
        i32.const 1
        i32.store8
      end
      i32.const 1
    )
    (func $hermes:cron/event#on-cron (;2;) (type 2) (param i32 i32 i32 i32 i32) (result i32)
      (local i32)
      block ;; label = @1
        global.get $GOT.data.internal.__memory_base
        i32.const 1048600
        i32.add
        i32.load8_u
        br_if 0 (;@1;)
        global.get $GOT.data.internal.__memory_base
        local.set 5
        call $__wasm_call_ctors
        local.get 5
        i32.const 1048600
        i32.add
        i32.const 1
        i32.store8
      end
      block ;; label = @1
        local.get 1
        i32.eqz
        br_if 0 (;@1;)
        local.get 0
        call $free
      end
      block ;; label = @1
        local.get 3
        i32.eqz
        br_if 0 (;@1;)
        local.get 2
        call $free
      end
      i32.const 1
    )
    (func $_ZN15memory_snapshot8bindings40__link_custom_section_describing_imports17h7006a9ae6faf0e0aE (;3;) (type 0))
    (func $_ZN3std3sys3pal6wasip27helpers14abort_internal17h4d8504fa71998c67E.llvm.13011707518019611084 (;4;) (type 0)
      call $abort
      unreachable
    )
    (func $cabi_realloc (;5;) (type 3) (param i32 i32 i32 i32) (result i32)
      (local i32)
      global.get $__stack_pointer
      i32.const 16
      i32.sub
      local.tee 4
      global.set $__stack_pointer
      block ;; label = @1
        block ;; label = @2
          block ;; label = @3
            block ;; label = @4
              local.get 1
              br_if 0 (;@4;)
              local.get 3
              i32.eqz
              br_if 3 (;@1;)
              block ;; label = @5
                block ;; label = @6
                  local.get 2
                  i32.const 8
                  i32.gt_u
                  br_if 0 (;@6;)
                  local.get 2
                  local.get 3
                  i32.le_u
                  br_if 1 (;@5;)
                end
                local.get 4
                i32.const 0
                i32.store offset=8
                local.get 4
                i32.const 8
                i32.add
                local.get 2
                i32.const 4
                local.get 2
                i32.const 4
                i32.gt_u
                select
                local.get 3
                call $posix_memalign
                br_if 3 (;@2;)
                local.get 4
                i32.load offset=8
                local.set 2
                br 2 (;@3;)
              end
              local.get 3
              call $malloc
              local.set 2
              br 1 (;@3;)
            end
            block ;; label = @4
              block ;; label = @5
                local.get 2
                i32.const 8
                i32.gt_u
                br_if 0 (;@5;)
                local.get 2
                local.get 3
                i32.le_u
                br_if 1 (;@4;)
              end
              local.get 4
              i32.const 0
              i32.store offset=12
              local.get 4
              i32.const 12
              i32.add
              local.get 2
              i32.const 4
              local.get 2
              i32.const 4
              i32.gt_u
              select
              local.get 3
              call $posix_memalign
              br_if 2 (;@2;)
              local.get 4
              i32.load offset=12
              local.tee 2
              i32.eqz
              br_if 2 (;@2;)
              block ;; label = @5
                local.get 3
                local.get 1
                local.get 3
                local.get 1
                i32.lt_u
                select
                local.tee 3
                i32.eqz
                br_if 0 (;@5;)
                local.get 2
                local.get 0
                local.get 3
                memory.copy
              end
              local.get 0
              call $free
              br 3 (;@1;)
            end
            local.get 0
            local.get 3
            call $realloc
            local.set 2
          end
          local.get 2
          br_if 1 (;@1;)
        end
        call $_ZN3std3sys3pal6wasip27helpers14abort_internal17h4d8504fa71998c67E.llvm.13011707518019611084
        unreachable
      end
      local.get 4
      i32.const 16
      i32.add
      global.set $__stack_pointer
      local.get 2
    )
    (func $malloc (;6;) (type 4) (param i32) (result i32)
      local.get 0
      call $dlmalloc
    )
    (func $dlmalloc (;7;) (type 4) (param i32) (result i32)
      (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
      global.get $__stack_pointer
      i32.const 16
      i32.sub
      local.tee 1
      global.set $__stack_pointer
      block ;; label = @1
        block ;; label = @2
          block ;; label = @3
            block ;; label = @4
              block ;; label = @5
                block ;; label = @6
                  block ;; label = @7
                    block ;; label = @8
                      block ;; label = @9
                        block ;; label = @10
                          block ;; label = @11
                            block ;; label = @12
                              block ;; label = @13
                                i32.const 0
                                i32.load offset=1048628
                                local.tee 2
                                br_if 0 (;@13;)
                                block ;; label = @14
                                  i32.const 0
                                  i32.load offset=1049076
                                  local.tee 3
                                  br_if 0 (;@14;)
                                  i32.const 0
                                  i64.const -1
                                  i64.store offset=1049088 align=4
                                  i32.const 0
                                  i64.const 281474976776192
                                  i64.store offset=1049080 align=4
                                  i32.const 0
                                  local.get 1
                                  i32.const 8
                                  i32.add
                                  i32.const -16
                                  i32.and
                                  i32.const 1431655768
                                  i32.xor
                                  local.tee 3
                                  i32.store offset=1049076
                                  i32.const 0
                                  i32.const 0
                                  i32.store offset=1049096
                                  i32.const 0
                                  i32.const 0
                                  i32.store offset=1049048
                                end
                                i32.const 1114112
                                i32.const 1049104
                                i32.lt_u
                                br_if 1 (;@12;)
                                i32.const 0
                                local.set 2
                                i32.const 1114112
                                i32.const 1049104
                                i32.sub
                                i32.const 89
                                i32.lt_u
                                br_if 0 (;@13;)
                                i32.const 0
                                local.set 4
                                i32.const 0
                                i32.const 1049104
                                i32.store offset=1049052
                                i32.const 0
                                i32.const 1049104
                                i32.store offset=1048620
                                i32.const 0
                                local.get 3
                                i32.store offset=1048640
                                i32.const 0
                                i32.const -1
                                i32.store offset=1048636
                                i32.const 0
                                i32.const 1114112
                                i32.const 1049104
                                i32.sub
                                local.tee 3
                                i32.store offset=1049056
                                i32.const 0
                                local.get 3
                                i32.store offset=1049040
                                i32.const 0
                                local.get 3
                                i32.store offset=1049036
                                loop ;; label = @14
                                  local.get 4
                                  i32.const 1048664
                                  i32.add
                                  local.get 4
                                  i32.const 1048652
                                  i32.add
                                  local.tee 3
                                  i32.store
                                  local.get 3
                                  local.get 4
                                  i32.const 1048644
                                  i32.add
                                  local.tee 5
                                  i32.store
                                  local.get 4
                                  i32.const 1048656
                                  i32.add
                                  local.get 5
                                  i32.store
                                  local.get 4
                                  i32.const 1048672
                                  i32.add
                                  local.get 4
                                  i32.const 1048660
                                  i32.add
                                  local.tee 5
                                  i32.store
                                  local.get 5
                                  local.get 3
                                  i32.store
                                  local.get 4
                                  i32.const 1048680
                                  i32.add
                                  local.get 4
                                  i32.const 1048668
                                  i32.add
                                  local.tee 3
                                  i32.store
                                  local.get 3
                                  local.get 5
                                  i32.store
                                  local.get 4
                                  i32.const 1048676
                                  i32.add
                                  local.get 3
                                  i32.store
                                  local.get 4
                                  i32.const 32
                                  i32.add
                                  local.tee 4
                                  i32.const 256
                                  i32.ne
                                  br_if 0 (;@14;)
                                end
                                i32.const 1114112
                                i32.const -52
                                i32.add
                                i32.const 56
                                i32.store
                                i32.const 0
                                i32.const 0
                                i32.load offset=1049092
                                i32.store offset=1048632
                                i32.const 0
                                i32.const 1049104
                                i32.const -8
                                i32.const 1049104
                                i32.sub
                                i32.const 15
                                i32.and
                                local.tee 4
                                i32.add
                                local.tee 2
                                i32.store offset=1048628
                                i32.const 0
                                i32.const 1114112
                                i32.const 1049104
                                i32.sub
                                local.get 4
                                i32.sub
                                i32.const -56
                                i32.add
                                local.tee 4
                                i32.store offset=1048616
                                local.get 2
                                local.get 4
                                i32.const 1
                                i32.or
                                i32.store offset=4
                              end
                              block ;; label = @13
                                block ;; label = @14
                                  local.get 0
                                  i32.const 236
                                  i32.gt_u
                                  br_if 0 (;@14;)
                                  block ;; label = @15
                                    i32.const 0
                                    i32.load offset=1048604
                                    local.tee 6
                                    i32.const 16
                                    local.get 0
                                    i32.const 19
                                    i32.add
                                    i32.const 496
                                    i32.and
                                    local.get 0
                                    i32.const 11
                                    i32.lt_u
                                    select
                                    local.tee 5
                                    i32.const 3
                                    i32.shr_u
                                    local.tee 3
                                    i32.shr_u
                                    local.tee 4
                                    i32.const 3
                                    i32.and
                                    i32.eqz
                                    br_if 0 (;@15;)
                                    block ;; label = @16
                                      block ;; label = @17
                                        local.get 4
                                        i32.const 1
                                        i32.and
                                        local.get 3
                                        i32.or
                                        i32.const 1
                                        i32.xor
                                        local.tee 5
                                        i32.const 3
                                        i32.shl
                                        local.tee 3
                                        i32.const 1048644
                                        i32.add
                                        local.tee 4
                                        local.get 3
                                        i32.const 1048652
                                        i32.add
                                        i32.load
                                        local.tee 3
                                        i32.load offset=8
                                        local.tee 0
                                        i32.ne
                                        br_if 0 (;@17;)
                                        i32.const 0
                                        local.get 6
                                        i32.const -2
                                        local.get 5
                                        i32.rotl
                                        i32.and
                                        i32.store offset=1048604
                                        br 1 (;@16;)
                                      end
                                      local.get 4
                                      local.get 0
                                      i32.store offset=8
                                      local.get 0
                                      local.get 4
                                      i32.store offset=12
                                    end
                                    local.get 3
                                    i32.const 8
                                    i32.add
                                    local.set 4
                                    local.get 3
                                    local.get 5
                                    i32.const 3
                                    i32.shl
                                    local.tee 5
                                    i32.const 3
                                    i32.or
                                    i32.store offset=4
                                    local.get 3
                                    local.get 5
                                    i32.add
                                    local.tee 3
                                    local.get 3
                                    i32.load offset=4
                                    i32.const 1
                                    i32.or
                                    i32.store offset=4
                                    br 14 (;@1;)
                                  end
                                  local.get 5
                                  i32.const 0
                                  i32.load offset=1048612
                                  local.tee 7
                                  i32.le_u
                                  br_if 1 (;@13;)
                                  block ;; label = @15
                                    local.get 4
                                    i32.eqz
                                    br_if 0 (;@15;)
                                    block ;; label = @16
                                      block ;; label = @17
                                        local.get 4
                                        local.get 3
                                        i32.shl
                                        i32.const 2
                                        local.get 3
                                        i32.shl
                                        local.tee 4
                                        i32.const 0
                                        local.get 4
                                        i32.sub
                                        i32.or
                                        i32.and
                                        i32.ctz
                                        local.tee 3
                                        i32.const 3
                                        i32.shl
                                        local.tee 4
                                        i32.const 1048644
                                        i32.add
                                        local.tee 0
                                        local.get 4
                                        i32.const 1048652
                                        i32.add
                                        i32.load
                                        local.tee 4
                                        i32.load offset=8
                                        local.tee 8
                                        i32.ne
                                        br_if 0 (;@17;)
                                        i32.const 0
                                        local.get 6
                                        i32.const -2
                                        local.get 3
                                        i32.rotl
                                        i32.and
                                        local.tee 6
                                        i32.store offset=1048604
                                        br 1 (;@16;)
                                      end
                                      local.get 0
                                      local.get 8
                                      i32.store offset=8
                                      local.get 8
                                      local.get 0
                                      i32.store offset=12
                                    end
                                    local.get 4
                                    local.get 5
                                    i32.const 3
                                    i32.or
                                    i32.store offset=4
                                    local.get 4
                                    local.get 3
                                    i32.const 3
                                    i32.shl
                                    local.tee 3
                                    i32.add
                                    local.get 3
                                    local.get 5
                                    i32.sub
                                    local.tee 0
                                    i32.store
                                    local.get 4
                                    local.get 5
                                    i32.add
                                    local.tee 8
                                    local.get 0
                                    i32.const 1
                                    i32.or
                                    i32.store offset=4
                                    block ;; label = @16
                                      local.get 7
                                      i32.eqz
                                      br_if 0 (;@16;)
                                      local.get 7
                                      i32.const -8
                                      i32.and
                                      i32.const 1048644
                                      i32.add
                                      local.set 5
                                      i32.const 0
                                      i32.load offset=1048624
                                      local.set 3
                                      block ;; label = @17
                                        block ;; label = @18
                                          local.get 6
                                          i32.const 1
                                          local.get 7
                                          i32.const 3
                                          i32.shr_u
                                          i32.shl
                                          local.tee 9
                                          i32.and
                                          br_if 0 (;@18;)
                                          i32.const 0
                                          local.get 6
                                          local.get 9
                                          i32.or
                                          i32.store offset=1048604
                                          local.get 5
                                          local.set 9
                                          br 1 (;@17;)
                                        end
                                        local.get 5
                                        i32.load offset=8
                                        local.set 9
                                      end
                                      local.get 9
                                      local.get 3
                                      i32.store offset=12
                                      local.get 5
                                      local.get 3
                                      i32.store offset=8
                                      local.get 3
                                      local.get 5
                                      i32.store offset=12
                                      local.get 3
                                      local.get 9
                                      i32.store offset=8
                                    end
                                    local.get 4
                                    i32.const 8
                                    i32.add
                                    local.set 4
                                    i32.const 0
                                    local.get 8
                                    i32.store offset=1048624
                                    i32.const 0
                                    local.get 0
                                    i32.store offset=1048612
                                    br 14 (;@1;)
                                  end
                                  i32.const 0
                                  i32.load offset=1048608
                                  local.tee 10
                                  i32.eqz
                                  br_if 1 (;@13;)
                                  local.get 10
                                  i32.ctz
                                  i32.const 2
                                  i32.shl
                                  i32.const 1048908
                                  i32.add
                                  i32.load
                                  local.tee 8
                                  i32.load offset=4
                                  i32.const -8
                                  i32.and
                                  local.get 5
                                  i32.sub
                                  local.set 3
                                  local.get 8
                                  local.set 0
                                  block ;; label = @15
                                    loop ;; label = @16
                                      block ;; label = @17
                                        local.get 0
                                        i32.load offset=16
                                        local.tee 4
                                        br_if 0 (;@17;)
                                        local.get 0
                                        i32.load offset=20
                                        local.tee 4
                                        i32.eqz
                                        br_if 2 (;@15;)
                                      end
                                      local.get 4
                                      i32.load offset=4
                                      i32.const -8
                                      i32.and
                                      local.get 5
                                      i32.sub
                                      local.tee 0
                                      local.get 3
                                      local.get 0
                                      local.get 3
                                      i32.lt_u
                                      local.tee 0
                                      select
                                      local.set 3
                                      local.get 4
                                      local.get 8
                                      local.get 0
                                      select
                                      local.set 8
                                      local.get 4
                                      local.set 0
                                      br 0 (;@16;)
                                    end
                                  end
                                  local.get 8
                                  i32.load offset=24
                                  local.set 2
                                  block ;; label = @15
                                    local.get 8
                                    i32.load offset=12
                                    local.tee 4
                                    local.get 8
                                    i32.eq
                                    br_if 0 (;@15;)
                                    local.get 8
                                    i32.load offset=8
                                    local.tee 0
                                    local.get 4
                                    i32.store offset=12
                                    local.get 4
                                    local.get 0
                                    i32.store offset=8
                                    br 13 (;@2;)
                                  end
                                  block ;; label = @15
                                    block ;; label = @16
                                      local.get 8
                                      i32.load offset=20
                                      local.tee 0
                                      i32.eqz
                                      br_if 0 (;@16;)
                                      local.get 8
                                      i32.const 20
                                      i32.add
                                      local.set 9
                                      br 1 (;@15;)
                                    end
                                    local.get 8
                                    i32.load offset=16
                                    local.tee 0
                                    i32.eqz
                                    br_if 4 (;@11;)
                                    local.get 8
                                    i32.const 16
                                    i32.add
                                    local.set 9
                                  end
                                  loop ;; label = @15
                                    local.get 9
                                    local.set 11
                                    local.get 0
                                    local.tee 4
                                    i32.const 20
                                    i32.add
                                    local.set 9
                                    local.get 4
                                    i32.load offset=20
                                    local.tee 0
                                    br_if 0 (;@15;)
                                    local.get 4
                                    i32.const 16
                                    i32.add
                                    local.set 9
                                    local.get 4
                                    i32.load offset=16
                                    local.tee 0
                                    br_if 0 (;@15;)
                                  end
                                  local.get 11
                                  i32.const 0
                                  i32.store
                                  br 12 (;@2;)
                                end
                                i32.const -1
                                local.set 5
                                local.get 0
                                i32.const -65
                                i32.gt_u
                                br_if 0 (;@13;)
                                local.get 0
                                i32.const 19
                                i32.add
                                local.tee 4
                                i32.const -16
                                i32.and
                                local.set 5
                                i32.const 0
                                i32.load offset=1048608
                                local.tee 10
                                i32.eqz
                                br_if 0 (;@13;)
                                i32.const 31
                                local.set 7
                                block ;; label = @14
                                  local.get 0
                                  i32.const 16777196
                                  i32.gt_u
                                  br_if 0 (;@14;)
                                  local.get 5
                                  i32.const 38
                                  local.get 4
                                  i32.const 8
                                  i32.shr_u
                                  i32.clz
                                  local.tee 4
                                  i32.sub
                                  i32.shr_u
                                  i32.const 1
                                  i32.and
                                  local.get 4
                                  i32.const 1
                                  i32.shl
                                  i32.sub
                                  i32.const 62
                                  i32.add
                                  local.set 7
                                end
                                i32.const 0
                                local.get 5
                                i32.sub
                                local.set 3
                                block ;; label = @14
                                  block ;; label = @15
                                    block ;; label = @16
                                      block ;; label = @17
                                        local.get 7
                                        i32.const 2
                                        i32.shl
                                        i32.const 1048908
                                        i32.add
                                        i32.load
                                        local.tee 0
                                        br_if 0 (;@17;)
                                        i32.const 0
                                        local.set 4
                                        i32.const 0
                                        local.set 9
                                        br 1 (;@16;)
                                      end
                                      i32.const 0
                                      local.set 4
                                      local.get 5
                                      i32.const 0
                                      i32.const 25
                                      local.get 7
                                      i32.const 1
                                      i32.shr_u
                                      i32.sub
                                      local.get 7
                                      i32.const 31
                                      i32.eq
                                      select
                                      i32.shl
                                      local.set 8
                                      i32.const 0
                                      local.set 9
                                      loop ;; label = @17
                                        block ;; label = @18
                                          local.get 0
                                          i32.load offset=4
                                          i32.const -8
                                          i32.and
                                          local.get 5
                                          i32.sub
                                          local.tee 6
                                          local.get 3
                                          i32.ge_u
                                          br_if 0 (;@18;)
                                          local.get 6
                                          local.set 3
                                          local.get 0
                                          local.set 9
                                          local.get 6
                                          br_if 0 (;@18;)
                                          i32.const 0
                                          local.set 3
                                          local.get 0
                                          local.set 9
                                          local.get 0
                                          local.set 4
                                          br 3 (;@15;)
                                        end
                                        local.get 4
                                        local.get 0
                                        i32.load offset=20
                                        local.tee 6
                                        local.get 6
                                        local.get 0
                                        local.get 8
                                        i32.const 29
                                        i32.shr_u
                                        i32.const 4
                                        i32.and
                                        i32.add
                                        i32.const 16
                                        i32.add
                                        i32.load
                                        local.tee 11
                                        i32.eq
                                        select
                                        local.get 4
                                        local.get 6
                                        select
                                        local.set 4
                                        local.get 8
                                        i32.const 1
                                        i32.shl
                                        local.set 8
                                        local.get 11
                                        local.set 0
                                        local.get 11
                                        br_if 0 (;@17;)
                                      end
                                    end
                                    block ;; label = @16
                                      local.get 4
                                      local.get 9
                                      i32.or
                                      br_if 0 (;@16;)
                                      i32.const 0
                                      local.set 9
                                      i32.const 2
                                      local.get 7
                                      i32.shl
                                      local.tee 4
                                      i32.const 0
                                      local.get 4
                                      i32.sub
                                      i32.or
                                      local.get 10
                                      i32.and
                                      local.tee 4
                                      i32.eqz
                                      br_if 3 (;@13;)
                                      local.get 4
                                      i32.ctz
                                      i32.const 2
                                      i32.shl
                                      i32.const 1048908
                                      i32.add
                                      i32.load
                                      local.set 4
                                    end
                                    local.get 4
                                    i32.eqz
                                    br_if 1 (;@14;)
                                  end
                                  loop ;; label = @15
                                    local.get 4
                                    i32.load offset=4
                                    i32.const -8
                                    i32.and
                                    local.get 5
                                    i32.sub
                                    local.tee 6
                                    local.get 3
                                    i32.lt_u
                                    local.set 8
                                    block ;; label = @16
                                      local.get 4
                                      i32.load offset=16
                                      local.tee 0
                                      br_if 0 (;@16;)
                                      local.get 4
                                      i32.load offset=20
                                      local.set 0
                                    end
                                    local.get 6
                                    local.get 3
                                    local.get 8
                                    select
                                    local.set 3
                                    local.get 4
                                    local.get 9
                                    local.get 8
                                    select
                                    local.set 9
                                    local.get 0
                                    local.set 4
                                    local.get 0
                                    br_if 0 (;@15;)
                                  end
                                end
                                local.get 9
                                i32.eqz
                                br_if 0 (;@13;)
                                local.get 3
                                i32.const 0
                                i32.load offset=1048612
                                local.get 5
                                i32.sub
                                i32.ge_u
                                br_if 0 (;@13;)
                                local.get 9
                                i32.load offset=24
                                local.set 11
                                block ;; label = @14
                                  local.get 9
                                  i32.load offset=12
                                  local.tee 4
                                  local.get 9
                                  i32.eq
                                  br_if 0 (;@14;)
                                  local.get 9
                                  i32.load offset=8
                                  local.tee 0
                                  local.get 4
                                  i32.store offset=12
                                  local.get 4
                                  local.get 0
                                  i32.store offset=8
                                  br 11 (;@3;)
                                end
                                block ;; label = @14
                                  block ;; label = @15
                                    local.get 9
                                    i32.load offset=20
                                    local.tee 0
                                    i32.eqz
                                    br_if 0 (;@15;)
                                    local.get 9
                                    i32.const 20
                                    i32.add
                                    local.set 8
                                    br 1 (;@14;)
                                  end
                                  local.get 9
                                  i32.load offset=16
                                  local.tee 0
                                  i32.eqz
                                  br_if 4 (;@10;)
                                  local.get 9
                                  i32.const 16
                                  i32.add
                                  local.set 8
                                end
                                loop ;; label = @14
                                  local.get 8
                                  local.set 6
                                  local.get 0
                                  local.tee 4
                                  i32.const 20
                                  i32.add
                                  local.set 8
                                  local.get 4
                                  i32.load offset=20
                                  local.tee 0
                                  br_if 0 (;@14;)
                                  local.get 4
                                  i32.const 16
                                  i32.add
                                  local.set 8
                                  local.get 4
                                  i32.load offset=16
                                  local.tee 0
                                  br_if 0 (;@14;)
                                end
                                local.get 6
                                i32.const 0
                                i32.store
                                br 10 (;@3;)
                              end
                              block ;; label = @13
                                i32.const 0
                                i32.load offset=1048612
                                local.tee 4
                                local.get 5
                                i32.lt_u
                                br_if 0 (;@13;)
                                i32.const 0
                                i32.load offset=1048624
                                local.set 3
                                block ;; label = @14
                                  block ;; label = @15
                                    local.get 4
                                    local.get 5
                                    i32.sub
                                    local.tee 0
                                    i32.const 16
                                    i32.lt_u
                                    br_if 0 (;@15;)
                                    local.get 3
                                    local.get 5
                                    i32.add
                                    local.tee 8
                                    local.get 0
                                    i32.const 1
                                    i32.or
                                    i32.store offset=4
                                    local.get 3
                                    local.get 4
                                    i32.add
                                    local.get 0
                                    i32.store
                                    local.get 3
                                    local.get 5
                                    i32.const 3
                                    i32.or
                                    i32.store offset=4
                                    br 1 (;@14;)
                                  end
                                  local.get 3
                                  local.get 4
                                  i32.const 3
                                  i32.or
                                  i32.store offset=4
                                  local.get 3
                                  local.get 4
                                  i32.add
                                  local.tee 4
                                  local.get 4
                                  i32.load offset=4
                                  i32.const 1
                                  i32.or
                                  i32.store offset=4
                                  i32.const 0
                                  local.set 8
                                  i32.const 0
                                  local.set 0
                                end
                                i32.const 0
                                local.get 0
                                i32.store offset=1048612
                                i32.const 0
                                local.get 8
                                i32.store offset=1048624
                                local.get 3
                                i32.const 8
                                i32.add
                                local.set 4
                                br 12 (;@1;)
                              end
                              block ;; label = @13
                                i32.const 0
                                i32.load offset=1048616
                                local.tee 0
                                local.get 5
                                i32.le_u
                                br_if 0 (;@13;)
                                local.get 2
                                local.get 5
                                i32.add
                                local.tee 4
                                local.get 0
                                local.get 5
                                i32.sub
                                local.tee 3
                                i32.const 1
                                i32.or
                                i32.store offset=4
                                i32.const 0
                                local.get 4
                                i32.store offset=1048628
                                i32.const 0
                                local.get 3
                                i32.store offset=1048616
                                local.get 2
                                local.get 5
                                i32.const 3
                                i32.or
                                i32.store offset=4
                                local.get 2
                                i32.const 8
                                i32.add
                                local.set 4
                                br 12 (;@1;)
                              end
                              block ;; label = @13
                                block ;; label = @14
                                  i32.const 0
                                  i32.load offset=1049076
                                  i32.eqz
                                  br_if 0 (;@14;)
                                  i32.const 0
                                  i32.load offset=1049084
                                  local.set 3
                                  br 1 (;@13;)
                                end
                                i32.const 0
                                i64.const -1
                                i64.store offset=1049088 align=4
                                i32.const 0
                                i64.const 281474976776192
                                i64.store offset=1049080 align=4
                                i32.const 0
                                local.get 1
                                i32.const 12
                                i32.add
                                i32.const -16
                                i32.and
                                i32.const 1431655768
                                i32.xor
                                i32.store offset=1049076
                                i32.const 0
                                i32.const 0
                                i32.store offset=1049096
                                i32.const 0
                                i32.const 0
                                i32.store offset=1049048
                                i32.const 65536
                                local.set 3
                              end
                              i32.const 0
                              local.set 4
                              block ;; label = @13
                                local.get 3
                                local.get 5
                                i32.const 71
                                i32.add
                                local.tee 11
                                i32.add
                                local.tee 8
                                i32.const 0
                                local.get 3
                                i32.sub
                                local.tee 6
                                i32.and
                                local.tee 9
                                local.get 5
                                i32.gt_u
                                br_if 0 (;@13;)
                                i32.const 0
                                i32.const 48
                                i32.store offset=1049100
                                br 12 (;@1;)
                              end
                              block ;; label = @13
                                i32.const 0
                                i32.load offset=1049044
                                local.tee 4
                                i32.eqz
                                br_if 0 (;@13;)
                                block ;; label = @14
                                  i32.const 0
                                  i32.load offset=1049036
                                  local.tee 3
                                  local.get 9
                                  i32.add
                                  local.tee 7
                                  local.get 3
                                  i32.le_u
                                  br_if 0 (;@14;)
                                  local.get 7
                                  local.get 4
                                  i32.le_u
                                  br_if 1 (;@13;)
                                end
                                i32.const 0
                                local.set 4
                                i32.const 0
                                i32.const 48
                                i32.store offset=1049100
                                br 12 (;@1;)
                              end
                              i32.const 0
                              i32.load8_u offset=1049048
                              i32.const 4
                              i32.and
                              br_if 5 (;@7;)
                              block ;; label = @13
                                block ;; label = @14
                                  block ;; label = @15
                                    local.get 2
                                    i32.eqz
                                    br_if 0 (;@15;)
                                    i32.const 1049052
                                    local.set 4
                                    loop ;; label = @16
                                      block ;; label = @17
                                        local.get 4
                                        i32.load
                                        local.tee 3
                                        local.get 2
                                        i32.gt_u
                                        br_if 0 (;@17;)
                                        local.get 3
                                        local.get 4
                                        i32.load offset=4
                                        i32.add
                                        local.get 2
                                        i32.gt_u
                                        br_if 3 (;@14;)
                                      end
                                      local.get 4
                                      i32.load offset=8
                                      local.tee 4
                                      br_if 0 (;@16;)
                                    end
                                  end
                                  i32.const 0
                                  call $sbrk
                                  local.tee 8
                                  i32.const -1
                                  i32.eq
                                  br_if 6 (;@8;)
                                  local.get 9
                                  local.set 6
                                  block ;; label = @15
                                    i32.const 0
                                    i32.load offset=1049080
                                    local.tee 4
                                    i32.const -1
                                    i32.add
                                    local.tee 3
                                    local.get 8
                                    i32.and
                                    i32.eqz
                                    br_if 0 (;@15;)
                                    local.get 9
                                    local.get 8
                                    i32.sub
                                    local.get 3
                                    local.get 8
                                    i32.add
                                    i32.const 0
                                    local.get 4
                                    i32.sub
                                    i32.and
                                    i32.add
                                    local.set 6
                                  end
                                  local.get 6
                                  local.get 5
                                  i32.le_u
                                  br_if 6 (;@8;)
                                  local.get 6
                                  i32.const 2147483646
                                  i32.gt_u
                                  br_if 6 (;@8;)
                                  block ;; label = @15
                                    i32.const 0
                                    i32.load offset=1049044
                                    local.tee 4
                                    i32.eqz
                                    br_if 0 (;@15;)
                                    i32.const 0
                                    i32.load offset=1049036
                                    local.tee 3
                                    local.get 6
                                    i32.add
                                    local.tee 0
                                    local.get 3
                                    i32.le_u
                                    br_if 7 (;@8;)
                                    local.get 0
                                    local.get 4
                                    i32.gt_u
                                    br_if 7 (;@8;)
                                  end
                                  local.get 6
                                  call $sbrk
                                  local.tee 4
                                  local.get 8
                                  i32.ne
                                  br_if 1 (;@13;)
                                  br 8 (;@6;)
                                end
                                local.get 8
                                local.get 0
                                i32.sub
                                local.get 6
                                i32.and
                                local.tee 6
                                i32.const 2147483646
                                i32.gt_u
                                br_if 5 (;@8;)
                                local.get 6
                                call $sbrk
                                local.tee 8
                                local.get 4
                                i32.load
                                local.get 4
                                i32.load offset=4
                                i32.add
                                i32.eq
                                br_if 4 (;@9;)
                                local.get 8
                                local.set 4
                              end
                              block ;; label = @13
                                local.get 6
                                local.get 5
                                i32.const 72
                                i32.add
                                i32.ge_u
                                br_if 0 (;@13;)
                                local.get 4
                                i32.const -1
                                i32.eq
                                br_if 0 (;@13;)
                                block ;; label = @14
                                  local.get 11
                                  local.get 6
                                  i32.sub
                                  i32.const 0
                                  i32.load offset=1049084
                                  local.tee 3
                                  i32.add
                                  i32.const 0
                                  local.get 3
                                  i32.sub
                                  i32.and
                                  local.tee 3
                                  i32.const 2147483646
                                  i32.le_u
                                  br_if 0 (;@14;)
                                  local.get 4
                                  local.set 8
                                  br 8 (;@6;)
                                end
                                block ;; label = @14
                                  local.get 3
                                  call $sbrk
                                  i32.const -1
                                  i32.eq
                                  br_if 0 (;@14;)
                                  local.get 3
                                  local.get 6
                                  i32.add
                                  local.set 6
                                  local.get 4
                                  local.set 8
                                  br 8 (;@6;)
                                end
                                i32.const 0
                                local.get 6
                                i32.sub
                                call $sbrk
                                drop
                                br 5 (;@8;)
                              end
                              local.get 4
                              local.set 8
                              local.get 4
                              i32.const -1
                              i32.ne
                              br_if 6 (;@6;)
                              br 4 (;@8;)
                            end
                            unreachable
                          end
                          i32.const 0
                          local.set 4
                          br 8 (;@2;)
                        end
                        i32.const 0
                        local.set 4
                        br 6 (;@3;)
                      end
                      local.get 8
                      i32.const -1
                      i32.ne
                      br_if 2 (;@6;)
                    end
                    i32.const 0
                    i32.const 0
                    i32.load offset=1049048
                    i32.const 4
                    i32.or
                    i32.store offset=1049048
                  end
                  local.get 9
                  i32.const 2147483646
                  i32.gt_u
                  br_if 1 (;@5;)
                  local.get 9
                  call $sbrk
                  local.set 8
                  i32.const 0
                  call $sbrk
                  local.set 4
                  local.get 8
                  i32.const -1
                  i32.eq
                  br_if 1 (;@5;)
                  local.get 4
                  i32.const -1
                  i32.eq
                  br_if 1 (;@5;)
                  local.get 8
                  local.get 4
                  i32.ge_u
                  br_if 1 (;@5;)
                  local.get 4
                  local.get 8
                  i32.sub
                  local.tee 6
                  local.get 5
                  i32.const 56
                  i32.add
                  i32.le_u
                  br_if 1 (;@5;)
                end
                i32.const 0
                i32.const 0
                i32.load offset=1049036
                local.get 6
                i32.add
                local.tee 4
                i32.store offset=1049036
                block ;; label = @6
                  local.get 4
                  i32.const 0
                  i32.load offset=1049040
                  i32.le_u
                  br_if 0 (;@6;)
                  i32.const 0
                  local.get 4
                  i32.store offset=1049040
                end
                block ;; label = @6
                  block ;; label = @7
                    block ;; label = @8
                      block ;; label = @9
                        i32.const 0
                        i32.load offset=1048628
                        local.tee 3
                        i32.eqz
                        br_if 0 (;@9;)
                        i32.const 1049052
                        local.set 4
                        loop ;; label = @10
                          local.get 8
                          local.get 4
                          i32.load
                          local.tee 0
                          local.get 4
                          i32.load offset=4
                          local.tee 9
                          i32.add
                          i32.eq
                          br_if 2 (;@8;)
                          local.get 4
                          i32.load offset=8
                          local.tee 4
                          br_if 0 (;@10;)
                          br 3 (;@7;)
                        end
                      end
                      block ;; label = @9
                        block ;; label = @10
                          i32.const 0
                          i32.load offset=1048620
                          local.tee 4
                          i32.eqz
                          br_if 0 (;@10;)
                          local.get 8
                          local.get 4
                          i32.ge_u
                          br_if 1 (;@9;)
                        end
                        i32.const 0
                        local.get 8
                        i32.store offset=1048620
                      end
                      i32.const 0
                      local.set 4
                      i32.const 0
                      local.get 6
                      i32.store offset=1049056
                      i32.const 0
                      local.get 8
                      i32.store offset=1049052
                      i32.const 0
                      i32.const -1
                      i32.store offset=1048636
                      i32.const 0
                      i32.const 0
                      i32.load offset=1049076
                      i32.store offset=1048640
                      i32.const 0
                      i32.const 0
                      i32.store offset=1049064
                      loop ;; label = @9
                        local.get 4
                        i32.const 1048664
                        i32.add
                        local.get 4
                        i32.const 1048652
                        i32.add
                        local.tee 3
                        i32.store
                        local.get 3
                        local.get 4
                        i32.const 1048644
                        i32.add
                        local.tee 0
                        i32.store
                        local.get 4
                        i32.const 1048656
                        i32.add
                        local.get 0
                        i32.store
                        local.get 4
                        i32.const 1048672
                        i32.add
                        local.get 4
                        i32.const 1048660
                        i32.add
                        local.tee 0
                        i32.store
                        local.get 0
                        local.get 3
                        i32.store
                        local.get 4
                        i32.const 1048680
                        i32.add
                        local.get 4
                        i32.const 1048668
                        i32.add
                        local.tee 3
                        i32.store
                        local.get 3
                        local.get 0
                        i32.store
                        local.get 4
                        i32.const 1048676
                        i32.add
                        local.get 3
                        i32.store
                        local.get 4
                        i32.const 32
                        i32.add
                        local.tee 4
                        i32.const 256
                        i32.ne
                        br_if 0 (;@9;)
                      end
                      local.get 8
                      i32.const -8
                      local.get 8
                      i32.sub
                      i32.const 15
                      i32.and
                      local.tee 4
                      i32.add
                      local.tee 3
                      local.get 6
                      i32.const -56
                      i32.add
                      local.tee 0
                      local.get 4
                      i32.sub
                      local.tee 4
                      i32.const 1
                      i32.or
                      i32.store offset=4
                      i32.const 0
                      i32.const 0
                      i32.load offset=1049092
                      i32.store offset=1048632
                      i32.const 0
                      local.get 4
                      i32.store offset=1048616
                      i32.const 0
                      local.get 3
                      i32.store offset=1048628
                      local.get 8
                      local.get 0
                      i32.add
                      i32.const 56
                      i32.store offset=4
                      br 2 (;@6;)
                    end
                    local.get 3
                    local.get 8
                    i32.ge_u
                    br_if 0 (;@7;)
                    local.get 3
                    local.get 0
                    i32.lt_u
                    br_if 0 (;@7;)
                    local.get 4
                    i32.load offset=12
                    i32.const 8
                    i32.and
                    br_if 0 (;@7;)
                    local.get 3
                    i32.const -8
                    local.get 3
                    i32.sub
                    i32.const 15
                    i32.and
                    local.tee 0
                    i32.add
                    local.tee 8
                    i32.const 0
                    i32.load offset=1048616
                    local.get 6
                    i32.add
                    local.tee 11
                    local.get 0
                    i32.sub
                    local.tee 0
                    i32.const 1
                    i32.or
                    i32.store offset=4
                    local.get 4
                    local.get 9
                    local.get 6
                    i32.add
                    i32.store offset=4
                    i32.const 0
                    i32.const 0
                    i32.load offset=1049092
                    i32.store offset=1048632
                    i32.const 0
                    local.get 0
                    i32.store offset=1048616
                    i32.const 0
                    local.get 8
                    i32.store offset=1048628
                    local.get 3
                    local.get 11
                    i32.add
                    i32.const 56
                    i32.store offset=4
                    br 1 (;@6;)
                  end
                  block ;; label = @7
                    local.get 8
                    i32.const 0
                    i32.load offset=1048620
                    i32.ge_u
                    br_if 0 (;@7;)
                    i32.const 0
                    local.get 8
                    i32.store offset=1048620
                  end
                  local.get 8
                  local.get 6
                  i32.add
                  local.set 0
                  i32.const 1049052
                  local.set 4
                  block ;; label = @7
                    block ;; label = @8
                      loop ;; label = @9
                        local.get 4
                        i32.load
                        local.tee 9
                        local.get 0
                        i32.eq
                        br_if 1 (;@8;)
                        local.get 4
                        i32.load offset=8
                        local.tee 4
                        br_if 0 (;@9;)
                        br 2 (;@7;)
                      end
                    end
                    local.get 4
                    i32.load8_u offset=12
                    i32.const 8
                    i32.and
                    i32.eqz
                    br_if 3 (;@4;)
                  end
                  i32.const 1049052
                  local.set 4
                  block ;; label = @7
                    loop ;; label = @8
                      block ;; label = @9
                        local.get 4
                        i32.load
                        local.tee 0
                        local.get 3
                        i32.gt_u
                        br_if 0 (;@9;)
                        local.get 0
                        local.get 4
                        i32.load offset=4
                        i32.add
                        local.tee 0
                        local.get 3
                        i32.gt_u
                        br_if 2 (;@7;)
                      end
                      local.get 4
                      i32.load offset=8
                      local.set 4
                      br 0 (;@8;)
                    end
                  end
                  local.get 8
                  i32.const -8
                  local.get 8
                  i32.sub
                  i32.const 15
                  i32.and
                  local.tee 4
                  i32.add
                  local.tee 11
                  local.get 6
                  i32.const -56
                  i32.add
                  local.tee 9
                  local.get 4
                  i32.sub
                  local.tee 4
                  i32.const 1
                  i32.or
                  i32.store offset=4
                  local.get 8
                  local.get 9
                  i32.add
                  i32.const 56
                  i32.store offset=4
                  local.get 3
                  local.get 0
                  i32.const 55
                  local.get 0
                  i32.sub
                  i32.const 15
                  i32.and
                  i32.add
                  i32.const -63
                  i32.add
                  local.tee 9
                  local.get 9
                  local.get 3
                  i32.const 16
                  i32.add
                  i32.lt_u
                  select
                  local.tee 9
                  i32.const 35
                  i32.store offset=4
                  i32.const 0
                  i32.const 0
                  i32.load offset=1049092
                  i32.store offset=1048632
                  i32.const 0
                  local.get 4
                  i32.store offset=1048616
                  i32.const 0
                  local.get 11
                  i32.store offset=1048628
                  local.get 9
                  i32.const 16
                  i32.add
                  i32.const 0
                  i64.load offset=1049060 align=4
                  i64.store align=4
                  local.get 9
                  i32.const 0
                  i64.load offset=1049052 align=4
                  i64.store offset=8 align=4
                  i32.const 0
                  local.get 9
                  i32.const 8
                  i32.add
                  i32.store offset=1049060
                  i32.const 0
                  local.get 6
                  i32.store offset=1049056
                  i32.const 0
                  local.get 8
                  i32.store offset=1049052
                  i32.const 0
                  i32.const 0
                  i32.store offset=1049064
                  local.get 9
                  i32.const 36
                  i32.add
                  local.set 4
                  loop ;; label = @7
                    local.get 4
                    i32.const 7
                    i32.store
                    local.get 4
                    i32.const 4
                    i32.add
                    local.tee 4
                    local.get 0
                    i32.lt_u
                    br_if 0 (;@7;)
                  end
                  local.get 9
                  local.get 3
                  i32.eq
                  br_if 0 (;@6;)
                  local.get 9
                  local.get 9
                  i32.load offset=4
                  i32.const -2
                  i32.and
                  i32.store offset=4
                  local.get 9
                  local.get 9
                  local.get 3
                  i32.sub
                  local.tee 8
                  i32.store
                  local.get 3
                  local.get 8
                  i32.const 1
                  i32.or
                  i32.store offset=4
                  block ;; label = @7
                    block ;; label = @8
                      local.get 8
                      i32.const 255
                      i32.gt_u
                      br_if 0 (;@8;)
                      local.get 8
                      i32.const -8
                      i32.and
                      i32.const 1048644
                      i32.add
                      local.set 4
                      block ;; label = @9
                        block ;; label = @10
                          i32.const 0
                          i32.load offset=1048604
                          local.tee 0
                          i32.const 1
                          local.get 8
                          i32.const 3
                          i32.shr_u
                          i32.shl
                          local.tee 8
                          i32.and
                          br_if 0 (;@10;)
                          i32.const 0
                          local.get 0
                          local.get 8
                          i32.or
                          i32.store offset=1048604
                          local.get 4
                          local.set 0
                          br 1 (;@9;)
                        end
                        local.get 4
                        i32.load offset=8
                        local.set 0
                      end
                      local.get 0
                      local.get 3
                      i32.store offset=12
                      local.get 4
                      local.get 3
                      i32.store offset=8
                      i32.const 12
                      local.set 8
                      i32.const 8
                      local.set 9
                      br 1 (;@7;)
                    end
                    i32.const 31
                    local.set 4
                    block ;; label = @8
                      local.get 8
                      i32.const 16777215
                      i32.gt_u
                      br_if 0 (;@8;)
                      local.get 8
                      i32.const 38
                      local.get 8
                      i32.const 8
                      i32.shr_u
                      i32.clz
                      local.tee 4
                      i32.sub
                      i32.shr_u
                      i32.const 1
                      i32.and
                      local.get 4
                      i32.const 1
                      i32.shl
                      i32.sub
                      i32.const 62
                      i32.add
                      local.set 4
                    end
                    local.get 3
                    local.get 4
                    i32.store offset=28
                    local.get 3
                    i64.const 0
                    i64.store offset=16 align=4
                    local.get 4
                    i32.const 2
                    i32.shl
                    i32.const 1048908
                    i32.add
                    local.set 0
                    block ;; label = @8
                      block ;; label = @9
                        block ;; label = @10
                          i32.const 0
                          i32.load offset=1048608
                          local.tee 9
                          i32.const 1
                          local.get 4
                          i32.shl
                          local.tee 6
                          i32.and
                          br_if 0 (;@10;)
                          local.get 0
                          local.get 3
                          i32.store
                          i32.const 0
                          local.get 9
                          local.get 6
                          i32.or
                          i32.store offset=1048608
                          local.get 3
                          local.get 0
                          i32.store offset=24
                          br 1 (;@9;)
                        end
                        local.get 8
                        i32.const 0
                        i32.const 25
                        local.get 4
                        i32.const 1
                        i32.shr_u
                        i32.sub
                        local.get 4
                        i32.const 31
                        i32.eq
                        select
                        i32.shl
                        local.set 4
                        local.get 0
                        i32.load
                        local.set 9
                        loop ;; label = @10
                          local.get 9
                          local.tee 0
                          i32.load offset=4
                          i32.const -8
                          i32.and
                          local.get 8
                          i32.eq
                          br_if 2 (;@8;)
                          local.get 4
                          i32.const 29
                          i32.shr_u
                          local.set 9
                          local.get 4
                          i32.const 1
                          i32.shl
                          local.set 4
                          local.get 0
                          local.get 9
                          i32.const 4
                          i32.and
                          i32.add
                          i32.const 16
                          i32.add
                          local.tee 6
                          i32.load
                          local.tee 9
                          br_if 0 (;@10;)
                        end
                        local.get 6
                        local.get 3
                        i32.store
                        local.get 3
                        local.get 0
                        i32.store offset=24
                      end
                      i32.const 8
                      local.set 8
                      i32.const 12
                      local.set 9
                      local.get 3
                      local.set 0
                      local.get 3
                      local.set 4
                      br 1 (;@7;)
                    end
                    local.get 0
                    i32.load offset=8
                    local.set 4
                    local.get 0
                    local.get 3
                    i32.store offset=8
                    local.get 4
                    local.get 3
                    i32.store offset=12
                    local.get 3
                    local.get 4
                    i32.store offset=8
                    i32.const 0
                    local.set 4
                    i32.const 24
                    local.set 8
                    i32.const 12
                    local.set 9
                  end
                  local.get 3
                  local.get 9
                  i32.add
                  local.get 0
                  i32.store
                  local.get 3
                  local.get 8
                  i32.add
                  local.get 4
                  i32.store
                end
                i32.const 0
                i32.load offset=1048616
                local.tee 4
                local.get 5
                i32.le_u
                br_if 0 (;@5;)
                i32.const 0
                i32.load offset=1048628
                local.tee 3
                local.get 5
                i32.add
                local.tee 0
                local.get 4
                local.get 5
                i32.sub
                local.tee 4
                i32.const 1
                i32.or
                i32.store offset=4
                i32.const 0
                local.get 4
                i32.store offset=1048616
                i32.const 0
                local.get 0
                i32.store offset=1048628
                local.get 3
                local.get 5
                i32.const 3
                i32.or
                i32.store offset=4
                local.get 3
                i32.const 8
                i32.add
                local.set 4
                br 4 (;@1;)
              end
              i32.const 0
              local.set 4
              i32.const 0
              i32.const 48
              i32.store offset=1049100
              br 3 (;@1;)
            end
            local.get 4
            local.get 8
            i32.store
            local.get 4
            local.get 4
            i32.load offset=4
            local.get 6
            i32.add
            i32.store offset=4
            local.get 8
            local.get 9
            local.get 5
            call $prepend_alloc
            local.set 4
            br 2 (;@1;)
          end
          block ;; label = @3
            local.get 11
            i32.eqz
            br_if 0 (;@3;)
            block ;; label = @4
              block ;; label = @5
                local.get 9
                local.get 9
                i32.load offset=28
                local.tee 8
                i32.const 2
                i32.shl
                i32.const 1048908
                i32.add
                local.tee 0
                i32.load
                i32.ne
                br_if 0 (;@5;)
                local.get 0
                local.get 4
                i32.store
                local.get 4
                br_if 1 (;@4;)
                i32.const 0
                local.get 10
                i32.const -2
                local.get 8
                i32.rotl
                i32.and
                local.tee 10
                i32.store offset=1048608
                br 2 (;@3;)
              end
              local.get 11
              i32.const 16
              i32.const 20
              local.get 11
              i32.load offset=16
              local.get 9
              i32.eq
              select
              i32.add
              local.get 4
              i32.store
              local.get 4
              i32.eqz
              br_if 1 (;@3;)
            end
            local.get 4
            local.get 11
            i32.store offset=24
            block ;; label = @4
              local.get 9
              i32.load offset=16
              local.tee 0
              i32.eqz
              br_if 0 (;@4;)
              local.get 4
              local.get 0
              i32.store offset=16
              local.get 0
              local.get 4
              i32.store offset=24
            end
            local.get 9
            i32.load offset=20
            local.tee 0
            i32.eqz
            br_if 0 (;@3;)
            local.get 4
            local.get 0
            i32.store offset=20
            local.get 0
            local.get 4
            i32.store offset=24
          end
          block ;; label = @3
            block ;; label = @4
              local.get 3
              i32.const 15
              i32.gt_u
              br_if 0 (;@4;)
              local.get 9
              local.get 3
              local.get 5
              i32.or
              local.tee 4
              i32.const 3
              i32.or
              i32.store offset=4
              local.get 9
              local.get 4
              i32.add
              local.tee 4
              local.get 4
              i32.load offset=4
              i32.const 1
              i32.or
              i32.store offset=4
              br 1 (;@3;)
            end
            local.get 9
            local.get 5
            i32.add
            local.tee 8
            local.get 3
            i32.const 1
            i32.or
            i32.store offset=4
            local.get 9
            local.get 5
            i32.const 3
            i32.or
            i32.store offset=4
            local.get 8
            local.get 3
            i32.add
            local.get 3
            i32.store
            block ;; label = @4
              local.get 3
              i32.const 255
              i32.gt_u
              br_if 0 (;@4;)
              local.get 3
              i32.const -8
              i32.and
              i32.const 1048644
              i32.add
              local.set 4
              block ;; label = @5
                block ;; label = @6
                  i32.const 0
                  i32.load offset=1048604
                  local.tee 5
                  i32.const 1
                  local.get 3
                  i32.const 3
                  i32.shr_u
                  i32.shl
                  local.tee 3
                  i32.and
                  br_if 0 (;@6;)
                  i32.const 0
                  local.get 5
                  local.get 3
                  i32.or
                  i32.store offset=1048604
                  local.get 4
                  local.set 3
                  br 1 (;@5;)
                end
                local.get 4
                i32.load offset=8
                local.set 3
              end
              local.get 3
              local.get 8
              i32.store offset=12
              local.get 4
              local.get 8
              i32.store offset=8
              local.get 8
              local.get 4
              i32.store offset=12
              local.get 8
              local.get 3
              i32.store offset=8
              br 1 (;@3;)
            end
            i32.const 31
            local.set 4
            block ;; label = @4
              local.get 3
              i32.const 16777215
              i32.gt_u
              br_if 0 (;@4;)
              local.get 3
              i32.const 38
              local.get 3
              i32.const 8
              i32.shr_u
              i32.clz
              local.tee 4
              i32.sub
              i32.shr_u
              i32.const 1
              i32.and
              local.get 4
              i32.const 1
              i32.shl
              i32.sub
              i32.const 62
              i32.add
              local.set 4
            end
            local.get 8
            local.get 4
            i32.store offset=28
            local.get 8
            i64.const 0
            i64.store offset=16 align=4
            local.get 4
            i32.const 2
            i32.shl
            i32.const 1048908
            i32.add
            local.set 5
            block ;; label = @4
              local.get 10
              i32.const 1
              local.get 4
              i32.shl
              local.tee 0
              i32.and
              br_if 0 (;@4;)
              local.get 5
              local.get 8
              i32.store
              i32.const 0
              local.get 10
              local.get 0
              i32.or
              i32.store offset=1048608
              local.get 8
              local.get 5
              i32.store offset=24
              local.get 8
              local.get 8
              i32.store offset=8
              local.get 8
              local.get 8
              i32.store offset=12
              br 1 (;@3;)
            end
            local.get 3
            i32.const 0
            i32.const 25
            local.get 4
            i32.const 1
            i32.shr_u
            i32.sub
            local.get 4
            i32.const 31
            i32.eq
            select
            i32.shl
            local.set 4
            local.get 5
            i32.load
            local.set 0
            block ;; label = @4
              loop ;; label = @5
                local.get 0
                local.tee 5
                i32.load offset=4
                i32.const -8
                i32.and
                local.get 3
                i32.eq
                br_if 1 (;@4;)
                local.get 4
                i32.const 29
                i32.shr_u
                local.set 0
                local.get 4
                i32.const 1
                i32.shl
                local.set 4
                local.get 5
                local.get 0
                i32.const 4
                i32.and
                i32.add
                i32.const 16
                i32.add
                local.tee 6
                i32.load
                local.tee 0
                br_if 0 (;@5;)
              end
              local.get 6
              local.get 8
              i32.store
              local.get 8
              local.get 5
              i32.store offset=24
              local.get 8
              local.get 8
              i32.store offset=12
              local.get 8
              local.get 8
              i32.store offset=8
              br 1 (;@3;)
            end
            local.get 5
            i32.load offset=8
            local.tee 4
            local.get 8
            i32.store offset=12
            local.get 5
            local.get 8
            i32.store offset=8
            local.get 8
            i32.const 0
            i32.store offset=24
            local.get 8
            local.get 5
            i32.store offset=12
            local.get 8
            local.get 4
            i32.store offset=8
          end
          local.get 9
          i32.const 8
          i32.add
          local.set 4
          br 1 (;@1;)
        end
        block ;; label = @2
          local.get 2
          i32.eqz
          br_if 0 (;@2;)
          block ;; label = @3
            block ;; label = @4
              local.get 8
              local.get 8
              i32.load offset=28
              local.tee 9
              i32.const 2
              i32.shl
              i32.const 1048908
              i32.add
              local.tee 0
              i32.load
              i32.ne
              br_if 0 (;@4;)
              local.get 0
              local.get 4
              i32.store
              local.get 4
              br_if 1 (;@3;)
              i32.const 0
              local.get 10
              i32.const -2
              local.get 9
              i32.rotl
              i32.and
              i32.store offset=1048608
              br 2 (;@2;)
            end
            local.get 2
            i32.const 16
            i32.const 20
            local.get 2
            i32.load offset=16
            local.get 8
            i32.eq
            select
            i32.add
            local.get 4
            i32.store
            local.get 4
            i32.eqz
            br_if 1 (;@2;)
          end
          local.get 4
          local.get 2
          i32.store offset=24
          block ;; label = @3
            local.get 8
            i32.load offset=16
            local.tee 0
            i32.eqz
            br_if 0 (;@3;)
            local.get 4
            local.get 0
            i32.store offset=16
            local.get 0
            local.get 4
            i32.store offset=24
          end
          local.get 8
          i32.load offset=20
          local.tee 0
          i32.eqz
          br_if 0 (;@2;)
          local.get 4
          local.get 0
          i32.store offset=20
          local.get 0
          local.get 4
          i32.store offset=24
        end
        block ;; label = @2
          block ;; label = @3
            local.get 3
            i32.const 15
            i32.gt_u
            br_if 0 (;@3;)
            local.get 8
            local.get 3
            local.get 5
            i32.or
            local.tee 4
            i32.const 3
            i32.or
            i32.store offset=4
            local.get 8
            local.get 4
            i32.add
            local.tee 4
            local.get 4
            i32.load offset=4
            i32.const 1
            i32.or
            i32.store offset=4
            br 1 (;@2;)
          end
          local.get 8
          local.get 5
          i32.add
          local.tee 0
          local.get 3
          i32.const 1
          i32.or
          i32.store offset=4
          local.get 8
          local.get 5
          i32.const 3
          i32.or
          i32.store offset=4
          local.get 0
          local.get 3
          i32.add
          local.get 3
          i32.store
          block ;; label = @3
            local.get 7
            i32.eqz
            br_if 0 (;@3;)
            local.get 7
            i32.const -8
            i32.and
            i32.const 1048644
            i32.add
            local.set 5
            i32.const 0
            i32.load offset=1048624
            local.set 4
            block ;; label = @4
              block ;; label = @5
                i32.const 1
                local.get 7
                i32.const 3
                i32.shr_u
                i32.shl
                local.tee 9
                local.get 6
                i32.and
                br_if 0 (;@5;)
                i32.const 0
                local.get 9
                local.get 6
                i32.or
                i32.store offset=1048604
                local.get 5
                local.set 9
                br 1 (;@4;)
              end
              local.get 5
              i32.load offset=8
              local.set 9
            end
            local.get 9
            local.get 4
            i32.store offset=12
            local.get 5
            local.get 4
            i32.store offset=8
            local.get 4
            local.get 5
            i32.store offset=12
            local.get 4
            local.get 9
            i32.store offset=8
          end
          i32.const 0
          local.get 0
          i32.store offset=1048624
          i32.const 0
          local.get 3
          i32.store offset=1048612
        end
        local.get 8
        i32.const 8
        i32.add
        local.set 4
      end
      local.get 1
      i32.const 16
      i32.add
      global.set $__stack_pointer
      local.get 4
    )
    (func $prepend_alloc (;8;) (type 5) (param i32 i32 i32) (result i32)
      (local i32 i32 i32 i32 i32 i32 i32)
      local.get 0
      i32.const -8
      local.get 0
      i32.sub
      i32.const 15
      i32.and
      i32.add
      local.tee 3
      local.get 2
      i32.const 3
      i32.or
      i32.store offset=4
      local.get 1
      i32.const -8
      local.get 1
      i32.sub
      i32.const 15
      i32.and
      i32.add
      local.tee 4
      local.get 3
      local.get 2
      i32.add
      local.tee 5
      i32.sub
      local.set 0
      block ;; label = @1
        block ;; label = @2
          local.get 4
          i32.const 0
          i32.load offset=1048628
          i32.ne
          br_if 0 (;@2;)
          i32.const 0
          local.get 5
          i32.store offset=1048628
          i32.const 0
          i32.const 0
          i32.load offset=1048616
          local.get 0
          i32.add
          local.tee 2
          i32.store offset=1048616
          local.get 5
          local.get 2
          i32.const 1
          i32.or
          i32.store offset=4
          br 1 (;@1;)
        end
        block ;; label = @2
          local.get 4
          i32.const 0
          i32.load offset=1048624
          i32.ne
          br_if 0 (;@2;)
          i32.const 0
          local.get 5
          i32.store offset=1048624
          i32.const 0
          i32.const 0
          i32.load offset=1048612
          local.get 0
          i32.add
          local.tee 2
          i32.store offset=1048612
          local.get 5
          local.get 2
          i32.const 1
          i32.or
          i32.store offset=4
          local.get 5
          local.get 2
          i32.add
          local.get 2
          i32.store
          br 1 (;@1;)
        end
        block ;; label = @2
          local.get 4
          i32.load offset=4
          local.tee 1
          i32.const 3
          i32.and
          i32.const 1
          i32.ne
          br_if 0 (;@2;)
          local.get 1
          i32.const -8
          i32.and
          local.set 6
          local.get 4
          i32.load offset=12
          local.set 2
          block ;; label = @3
            block ;; label = @4
              local.get 1
              i32.const 255
              i32.gt_u
              br_if 0 (;@4;)
              block ;; label = @5
                local.get 2
                local.get 4
                i32.load offset=8
                local.tee 7
                i32.ne
                br_if 0 (;@5;)
                i32.const 0
                i32.const 0
                i32.load offset=1048604
                i32.const -2
                local.get 1
                i32.const 3
                i32.shr_u
                i32.rotl
                i32.and
                i32.store offset=1048604
                br 2 (;@3;)
              end
              local.get 2
              local.get 7
              i32.store offset=8
              local.get 7
              local.get 2
              i32.store offset=12
              br 1 (;@3;)
            end
            local.get 4
            i32.load offset=24
            local.set 8
            block ;; label = @4
              block ;; label = @5
                local.get 2
                local.get 4
                i32.eq
                br_if 0 (;@5;)
                local.get 4
                i32.load offset=8
                local.tee 1
                local.get 2
                i32.store offset=12
                local.get 2
                local.get 1
                i32.store offset=8
                br 1 (;@4;)
              end
              block ;; label = @5
                block ;; label = @6
                  block ;; label = @7
                    local.get 4
                    i32.load offset=20
                    local.tee 1
                    i32.eqz
                    br_if 0 (;@7;)
                    local.get 4
                    i32.const 20
                    i32.add
                    local.set 7
                    br 1 (;@6;)
                  end
                  local.get 4
                  i32.load offset=16
                  local.tee 1
                  i32.eqz
                  br_if 1 (;@5;)
                  local.get 4
                  i32.const 16
                  i32.add
                  local.set 7
                end
                loop ;; label = @6
                  local.get 7
                  local.set 9
                  local.get 1
                  local.tee 2
                  i32.const 20
                  i32.add
                  local.set 7
                  local.get 2
                  i32.load offset=20
                  local.tee 1
                  br_if 0 (;@6;)
                  local.get 2
                  i32.const 16
                  i32.add
                  local.set 7
                  local.get 2
                  i32.load offset=16
                  local.tee 1
                  br_if 0 (;@6;)
                end
                local.get 9
                i32.const 0
                i32.store
                br 1 (;@4;)
              end
              i32.const 0
              local.set 2
            end
            local.get 8
            i32.eqz
            br_if 0 (;@3;)
            block ;; label = @4
              block ;; label = @5
                local.get 4
                local.get 4
                i32.load offset=28
                local.tee 7
                i32.const 2
                i32.shl
                i32.const 1048908
                i32.add
                local.tee 1
                i32.load
                i32.ne
                br_if 0 (;@5;)
                local.get 1
                local.get 2
                i32.store
                local.get 2
                br_if 1 (;@4;)
                i32.const 0
                i32.const 0
                i32.load offset=1048608
                i32.const -2
                local.get 7
                i32.rotl
                i32.and
                i32.store offset=1048608
                br 2 (;@3;)
              end
              local.get 8
              i32.const 16
              i32.const 20
              local.get 8
              i32.load offset=16
              local.get 4
              i32.eq
              select
              i32.add
              local.get 2
              i32.store
              local.get 2
              i32.eqz
              br_if 1 (;@3;)
            end
            local.get 2
            local.get 8
            i32.store offset=24
            block ;; label = @4
              local.get 4
              i32.load offset=16
              local.tee 1
              i32.eqz
              br_if 0 (;@4;)
              local.get 2
              local.get 1
              i32.store offset=16
              local.get 1
              local.get 2
              i32.store offset=24
            end
            local.get 4
            i32.load offset=20
            local.tee 1
            i32.eqz
            br_if 0 (;@3;)
            local.get 2
            local.get 1
            i32.store offset=20
            local.get 1
            local.get 2
            i32.store offset=24
          end
          local.get 6
          local.get 0
          i32.add
          local.set 0
          local.get 4
          local.get 6
          i32.add
          local.tee 4
          i32.load offset=4
          local.set 1
        end
        local.get 4
        local.get 1
        i32.const -2
        i32.and
        i32.store offset=4
        local.get 5
        local.get 0
        i32.add
        local.get 0
        i32.store
        local.get 5
        local.get 0
        i32.const 1
        i32.or
        i32.store offset=4
        block ;; label = @2
          local.get 0
          i32.const 255
          i32.gt_u
          br_if 0 (;@2;)
          local.get 0
          i32.const -8
          i32.and
          i32.const 1048644
          i32.add
          local.set 2
          block ;; label = @3
            block ;; label = @4
              i32.const 0
              i32.load offset=1048604
              local.tee 1
              i32.const 1
              local.get 0
              i32.const 3
              i32.shr_u
              i32.shl
              local.tee 0
              i32.and
              br_if 0 (;@4;)
              i32.const 0
              local.get 1
              local.get 0
              i32.or
              i32.store offset=1048604
              local.get 2
              local.set 0
              br 1 (;@3;)
            end
            local.get 2
            i32.load offset=8
            local.set 0
          end
          local.get 0
          local.get 5
          i32.store offset=12
          local.get 2
          local.get 5
          i32.store offset=8
          local.get 5
          local.get 2
          i32.store offset=12
          local.get 5
          local.get 0
          i32.store offset=8
          br 1 (;@1;)
        end
        i32.const 31
        local.set 2
        block ;; label = @2
          local.get 0
          i32.const 16777215
          i32.gt_u
          br_if 0 (;@2;)
          local.get 0
          i32.const 38
          local.get 0
          i32.const 8
          i32.shr_u
          i32.clz
          local.tee 2
          i32.sub
          i32.shr_u
          i32.const 1
          i32.and
          local.get 2
          i32.const 1
          i32.shl
          i32.sub
          i32.const 62
          i32.add
          local.set 2
        end
        local.get 5
        local.get 2
        i32.store offset=28
        local.get 5
        i64.const 0
        i64.store offset=16 align=4
        local.get 2
        i32.const 2
        i32.shl
        i32.const 1048908
        i32.add
        local.set 1
        block ;; label = @2
          i32.const 0
          i32.load offset=1048608
          local.tee 7
          i32.const 1
          local.get 2
          i32.shl
          local.tee 4
          i32.and
          br_if 0 (;@2;)
          local.get 1
          local.get 5
          i32.store
          i32.const 0
          local.get 7
          local.get 4
          i32.or
          i32.store offset=1048608
          local.get 5
          local.get 1
          i32.store offset=24
          local.get 5
          local.get 5
          i32.store offset=8
          local.get 5
          local.get 5
          i32.store offset=12
          br 1 (;@1;)
        end
        local.get 0
        i32.const 0
        i32.const 25
        local.get 2
        i32.const 1
        i32.shr_u
        i32.sub
        local.get 2
        i32.const 31
        i32.eq
        select
        i32.shl
        local.set 2
        local.get 1
        i32.load
        local.set 7
        block ;; label = @2
          loop ;; label = @3
            local.get 7
            local.tee 1
            i32.load offset=4
            i32.const -8
            i32.and
            local.get 0
            i32.eq
            br_if 1 (;@2;)
            local.get 2
            i32.const 29
            i32.shr_u
            local.set 7
            local.get 2
            i32.const 1
            i32.shl
            local.set 2
            local.get 1
            local.get 7
            i32.const 4
            i32.and
            i32.add
            i32.const 16
            i32.add
            local.tee 4
            i32.load
            local.tee 7
            br_if 0 (;@3;)
          end
          local.get 4
          local.get 5
          i32.store
          local.get 5
          local.get 1
          i32.store offset=24
          local.get 5
          local.get 5
          i32.store offset=12
          local.get 5
          local.get 5
          i32.store offset=8
          br 1 (;@1;)
        end
        local.get 1
        i32.load offset=8
        local.tee 2
        local.get 5
        i32.store offset=12
        local.get 1
        local.get 5
        i32.store offset=8
        local.get 5
        i32.const 0
        i32.store offset=24
        local.get 5
        local.get 1
        i32.store offset=12
        local.get 5
        local.get 2
        i32.store offset=8
      end
      local.get 3
      i32.const 8
      i32.add
    )
    (func $free (;9;) (type 6) (param i32)
      local.get 0
      call $dlfree
    )
    (func $dlfree (;10;) (type 6) (param i32)
      (local i32 i32 i32 i32 i32 i32 i32)
      block ;; label = @1
        local.get 0
        i32.eqz
        br_if 0 (;@1;)
        local.get 0
        i32.const -8
        i32.add
        local.tee 1
        local.get 0
        i32.const -4
        i32.add
        i32.load
        local.tee 2
        i32.const -8
        i32.and
        local.tee 0
        i32.add
        local.set 3
        block ;; label = @2
          local.get 2
          i32.const 1
          i32.and
          br_if 0 (;@2;)
          local.get 2
          i32.const 2
          i32.and
          i32.eqz
          br_if 1 (;@1;)
          local.get 1
          local.get 1
          i32.load
          local.tee 4
          i32.sub
          local.tee 1
          i32.const 0
          i32.load offset=1048620
          i32.lt_u
          br_if 1 (;@1;)
          local.get 4
          local.get 0
          i32.add
          local.set 0
          block ;; label = @3
            block ;; label = @4
              block ;; label = @5
                block ;; label = @6
                  local.get 1
                  i32.const 0
                  i32.load offset=1048624
                  i32.eq
                  br_if 0 (;@6;)
                  local.get 1
                  i32.load offset=12
                  local.set 2
                  block ;; label = @7
                    local.get 4
                    i32.const 255
                    i32.gt_u
                    br_if 0 (;@7;)
                    local.get 2
                    local.get 1
                    i32.load offset=8
                    local.tee 5
                    i32.ne
                    br_if 2 (;@5;)
                    i32.const 0
                    i32.const 0
                    i32.load offset=1048604
                    i32.const -2
                    local.get 4
                    i32.const 3
                    i32.shr_u
                    i32.rotl
                    i32.and
                    i32.store offset=1048604
                    br 5 (;@2;)
                  end
                  local.get 1
                  i32.load offset=24
                  local.set 6
                  block ;; label = @7
                    local.get 2
                    local.get 1
                    i32.eq
                    br_if 0 (;@7;)
                    local.get 1
                    i32.load offset=8
                    local.tee 4
                    local.get 2
                    i32.store offset=12
                    local.get 2
                    local.get 4
                    i32.store offset=8
                    br 4 (;@3;)
                  end
                  block ;; label = @7
                    block ;; label = @8
                      local.get 1
                      i32.load offset=20
                      local.tee 4
                      i32.eqz
                      br_if 0 (;@8;)
                      local.get 1
                      i32.const 20
                      i32.add
                      local.set 5
                      br 1 (;@7;)
                    end
                    local.get 1
                    i32.load offset=16
                    local.tee 4
                    i32.eqz
                    br_if 3 (;@4;)
                    local.get 1
                    i32.const 16
                    i32.add
                    local.set 5
                  end
                  loop ;; label = @7
                    local.get 5
                    local.set 7
                    local.get 4
                    local.tee 2
                    i32.const 20
                    i32.add
                    local.set 5
                    local.get 2
                    i32.load offset=20
                    local.tee 4
                    br_if 0 (;@7;)
                    local.get 2
                    i32.const 16
                    i32.add
                    local.set 5
                    local.get 2
                    i32.load offset=16
                    local.tee 4
                    br_if 0 (;@7;)
                  end
                  local.get 7
                  i32.const 0
                  i32.store
                  br 3 (;@3;)
                end
                local.get 3
                i32.load offset=4
                local.tee 2
                i32.const 3
                i32.and
                i32.const 3
                i32.ne
                br_if 3 (;@2;)
                local.get 3
                local.get 2
                i32.const -2
                i32.and
                i32.store offset=4
                i32.const 0
                local.get 0
                i32.store offset=1048612
                local.get 3
                local.get 0
                i32.store
                local.get 1
                local.get 0
                i32.const 1
                i32.or
                i32.store offset=4
                return
              end
              local.get 2
              local.get 5
              i32.store offset=8
              local.get 5
              local.get 2
              i32.store offset=12
              br 2 (;@2;)
            end
            i32.const 0
            local.set 2
          end
          local.get 6
          i32.eqz
          br_if 0 (;@2;)
          block ;; label = @3
            block ;; label = @4
              local.get 1
              local.get 1
              i32.load offset=28
              local.tee 5
              i32.const 2
              i32.shl
              i32.const 1048908
              i32.add
              local.tee 4
              i32.load
              i32.ne
              br_if 0 (;@4;)
              local.get 4
              local.get 2
              i32.store
              local.get 2
              br_if 1 (;@3;)
              i32.const 0
              i32.const 0
              i32.load offset=1048608
              i32.const -2
              local.get 5
              i32.rotl
              i32.and
              i32.store offset=1048608
              br 2 (;@2;)
            end
            local.get 6
            i32.const 16
            i32.const 20
            local.get 6
            i32.load offset=16
            local.get 1
            i32.eq
            select
            i32.add
            local.get 2
            i32.store
            local.get 2
            i32.eqz
            br_if 1 (;@2;)
          end
          local.get 2
          local.get 6
          i32.store offset=24
          block ;; label = @3
            local.get 1
            i32.load offset=16
            local.tee 4
            i32.eqz
            br_if 0 (;@3;)
            local.get 2
            local.get 4
            i32.store offset=16
            local.get 4
            local.get 2
            i32.store offset=24
          end
          local.get 1
          i32.load offset=20
          local.tee 4
          i32.eqz
          br_if 0 (;@2;)
          local.get 2
          local.get 4
          i32.store offset=20
          local.get 4
          local.get 2
          i32.store offset=24
        end
        local.get 1
        local.get 3
        i32.ge_u
        br_if 0 (;@1;)
        local.get 3
        i32.load offset=4
        local.tee 4
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@1;)
        block ;; label = @2
          block ;; label = @3
            block ;; label = @4
              block ;; label = @5
                block ;; label = @6
                  local.get 4
                  i32.const 2
                  i32.and
                  br_if 0 (;@6;)
                  block ;; label = @7
                    local.get 3
                    i32.const 0
                    i32.load offset=1048628
                    i32.ne
                    br_if 0 (;@7;)
                    i32.const 0
                    local.get 1
                    i32.store offset=1048628
                    i32.const 0
                    i32.const 0
                    i32.load offset=1048616
                    local.get 0
                    i32.add
                    local.tee 0
                    i32.store offset=1048616
                    local.get 1
                    local.get 0
                    i32.const 1
                    i32.or
                    i32.store offset=4
                    local.get 1
                    i32.const 0
                    i32.load offset=1048624
                    i32.ne
                    br_if 6 (;@1;)
                    i32.const 0
                    i32.const 0
                    i32.store offset=1048612
                    i32.const 0
                    i32.const 0
                    i32.store offset=1048624
                    return
                  end
                  block ;; label = @7
                    local.get 3
                    i32.const 0
                    i32.load offset=1048624
                    i32.ne
                    br_if 0 (;@7;)
                    i32.const 0
                    local.get 1
                    i32.store offset=1048624
                    i32.const 0
                    i32.const 0
                    i32.load offset=1048612
                    local.get 0
                    i32.add
                    local.tee 0
                    i32.store offset=1048612
                    local.get 1
                    local.get 0
                    i32.const 1
                    i32.or
                    i32.store offset=4
                    local.get 1
                    local.get 0
                    i32.add
                    local.get 0
                    i32.store
                    return
                  end
                  local.get 4
                  i32.const -8
                  i32.and
                  local.get 0
                  i32.add
                  local.set 0
                  local.get 3
                  i32.load offset=12
                  local.set 2
                  block ;; label = @7
                    local.get 4
                    i32.const 255
                    i32.gt_u
                    br_if 0 (;@7;)
                    block ;; label = @8
                      local.get 2
                      local.get 3
                      i32.load offset=8
                      local.tee 5
                      i32.ne
                      br_if 0 (;@8;)
                      i32.const 0
                      i32.const 0
                      i32.load offset=1048604
                      i32.const -2
                      local.get 4
                      i32.const 3
                      i32.shr_u
                      i32.rotl
                      i32.and
                      i32.store offset=1048604
                      br 5 (;@3;)
                    end
                    local.get 2
                    local.get 5
                    i32.store offset=8
                    local.get 5
                    local.get 2
                    i32.store offset=12
                    br 4 (;@3;)
                  end
                  local.get 3
                  i32.load offset=24
                  local.set 6
                  block ;; label = @7
                    local.get 2
                    local.get 3
                    i32.eq
                    br_if 0 (;@7;)
                    local.get 3
                    i32.load offset=8
                    local.tee 4
                    local.get 2
                    i32.store offset=12
                    local.get 2
                    local.get 4
                    i32.store offset=8
                    br 3 (;@4;)
                  end
                  block ;; label = @7
                    block ;; label = @8
                      local.get 3
                      i32.load offset=20
                      local.tee 4
                      i32.eqz
                      br_if 0 (;@8;)
                      local.get 3
                      i32.const 20
                      i32.add
                      local.set 5
                      br 1 (;@7;)
                    end
                    local.get 3
                    i32.load offset=16
                    local.tee 4
                    i32.eqz
                    br_if 2 (;@5;)
                    local.get 3
                    i32.const 16
                    i32.add
                    local.set 5
                  end
                  loop ;; label = @7
                    local.get 5
                    local.set 7
                    local.get 4
                    local.tee 2
                    i32.const 20
                    i32.add
                    local.set 5
                    local.get 2
                    i32.load offset=20
                    local.tee 4
                    br_if 0 (;@7;)
                    local.get 2
                    i32.const 16
                    i32.add
                    local.set 5
                    local.get 2
                    i32.load offset=16
                    local.tee 4
                    br_if 0 (;@7;)
                  end
                  local.get 7
                  i32.const 0
                  i32.store
                  br 2 (;@4;)
                end
                local.get 3
                local.get 4
                i32.const -2
                i32.and
                i32.store offset=4
                local.get 1
                local.get 0
                i32.add
                local.get 0
                i32.store
                local.get 1
                local.get 0
                i32.const 1
                i32.or
                i32.store offset=4
                br 3 (;@2;)
              end
              i32.const 0
              local.set 2
            end
            local.get 6
            i32.eqz
            br_if 0 (;@3;)
            block ;; label = @4
              block ;; label = @5
                local.get 3
                local.get 3
                i32.load offset=28
                local.tee 5
                i32.const 2
                i32.shl
                i32.const 1048908
                i32.add
                local.tee 4
                i32.load
                i32.ne
                br_if 0 (;@5;)
                local.get 4
                local.get 2
                i32.store
                local.get 2
                br_if 1 (;@4;)
                i32.const 0
                i32.const 0
                i32.load offset=1048608
                i32.const -2
                local.get 5
                i32.rotl
                i32.and
                i32.store offset=1048608
                br 2 (;@3;)
              end
              local.get 6
              i32.const 16
              i32.const 20
              local.get 6
              i32.load offset=16
              local.get 3
              i32.eq
              select
              i32.add
              local.get 2
              i32.store
              local.get 2
              i32.eqz
              br_if 1 (;@3;)
            end
            local.get 2
            local.get 6
            i32.store offset=24
            block ;; label = @4
              local.get 3
              i32.load offset=16
              local.tee 4
              i32.eqz
              br_if 0 (;@4;)
              local.get 2
              local.get 4
              i32.store offset=16
              local.get 4
              local.get 2
              i32.store offset=24
            end
            local.get 3
            i32.load offset=20
            local.tee 4
            i32.eqz
            br_if 0 (;@3;)
            local.get 2
            local.get 4
            i32.store offset=20
            local.get 4
            local.get 2
            i32.store offset=24
          end
          local.get 1
          local.get 0
          i32.add
          local.get 0
          i32.store
          local.get 1
          local.get 0
          i32.const 1
          i32.or
          i32.store offset=4
          local.get 1
          i32.const 0
          i32.load offset=1048624
          i32.ne
          br_if 0 (;@2;)
          i32.const 0
          local.get 0
          i32.store offset=1048612
          return
        end
        block ;; label = @2
          local.get 0
          i32.const 255
          i32.gt_u
          br_if 0 (;@2;)
          local.get 0
          i32.const -8
          i32.and
          i32.const 1048644
          i32.add
          local.set 2
          block ;; label = @3
            block ;; label = @4
              i32.const 0
              i32.load offset=1048604
              local.tee 4
              i32.const 1
              local.get 0
              i32.const 3
              i32.shr_u
              i32.shl
              local.tee 0
              i32.and
              br_if 0 (;@4;)
              i32.const 0
              local.get 4
              local.get 0
              i32.or
              i32.store offset=1048604
              local.get 2
              local.set 0
              br 1 (;@3;)
            end
            local.get 2
            i32.load offset=8
            local.set 0
          end
          local.get 0
          local.get 1
          i32.store offset=12
          local.get 2
          local.get 1
          i32.store offset=8
          local.get 1
          local.get 2
          i32.store offset=12
          local.get 1
          local.get 0
          i32.store offset=8
          return
        end
        i32.const 31
        local.set 2
        block ;; label = @2
          local.get 0
          i32.const 16777215
          i32.gt_u
          br_if 0 (;@2;)
          local.get 0
          i32.const 38
          local.get 0
          i32.const 8
          i32.shr_u
          i32.clz
          local.tee 2
          i32.sub
          i32.shr_u
          i32.const 1
          i32.and
          local.get 2
          i32.const 1
          i32.shl
          i32.sub
          i32.const 62
          i32.add
          local.set 2
        end
        local.get 1
        local.get 2
        i32.store offset=28
        local.get 1
        i64.const 0
        i64.store offset=16 align=4
        local.get 2
        i32.const 2
        i32.shl
        i32.const 1048908
        i32.add
        local.set 3
        block ;; label = @2
          block ;; label = @3
            block ;; label = @4
              block ;; label = @5
                i32.const 0
                i32.load offset=1048608
                local.tee 4
                i32.const 1
                local.get 2
                i32.shl
                local.tee 5
                i32.and
                br_if 0 (;@5;)
                i32.const 0
                local.get 4
                local.get 5
                i32.or
                i32.store offset=1048608
                i32.const 8
                local.set 0
                i32.const 24
                local.set 2
                local.get 3
                local.set 5
                br 1 (;@4;)
              end
              local.get 0
              i32.const 0
              i32.const 25
              local.get 2
              i32.const 1
              i32.shr_u
              i32.sub
              local.get 2
              i32.const 31
              i32.eq
              select
              i32.shl
              local.set 2
              local.get 3
              i32.load
              local.set 5
              loop ;; label = @5
                local.get 5
                local.tee 4
                i32.load offset=4
                i32.const -8
                i32.and
                local.get 0
                i32.eq
                br_if 2 (;@3;)
                local.get 2
                i32.const 29
                i32.shr_u
                local.set 5
                local.get 2
                i32.const 1
                i32.shl
                local.set 2
                local.get 4
                local.get 5
                i32.const 4
                i32.and
                i32.add
                i32.const 16
                i32.add
                local.tee 3
                i32.load
                local.tee 5
                br_if 0 (;@5;)
              end
              i32.const 8
              local.set 0
              i32.const 24
              local.set 2
              local.get 4
              local.set 5
            end
            local.get 1
            local.set 4
            local.get 1
            local.set 7
            br 1 (;@2;)
          end
          local.get 4
          i32.load offset=8
          local.tee 5
          local.get 1
          i32.store offset=12
          i32.const 8
          local.set 2
          local.get 4
          i32.const 8
          i32.add
          local.set 3
          i32.const 0
          local.set 7
          i32.const 24
          local.set 0
        end
        local.get 3
        local.get 1
        i32.store
        local.get 1
        local.get 2
        i32.add
        local.get 5
        i32.store
        local.get 1
        local.get 4
        i32.store offset=12
        local.get 1
        local.get 0
        i32.add
        local.get 7
        i32.store
        i32.const 0
        i32.const 0
        i32.load offset=1048636
        i32.const -1
        i32.add
        local.tee 1
        i32.const -1
        local.get 1
        select
        i32.store offset=1048636
      end
    )
    (func $realloc (;11;) (type 7) (param i32 i32) (result i32)
      (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
      block ;; label = @1
        local.get 0
        br_if 0 (;@1;)
        local.get 1
        call $dlmalloc
        return
      end
      block ;; label = @1
        local.get 1
        i32.const -64
        i32.lt_u
        br_if 0 (;@1;)
        i32.const 0
        i32.const 48
        i32.store offset=1049100
        i32.const 0
        return
      end
      i32.const 16
      local.get 1
      i32.const 19
      i32.add
      i32.const -16
      i32.and
      local.get 1
      i32.const 11
      i32.lt_u
      select
      local.set 2
      local.get 0
      i32.const -4
      i32.add
      local.tee 3
      i32.load
      local.tee 4
      i32.const -8
      i32.and
      local.set 5
      block ;; label = @1
        block ;; label = @2
          block ;; label = @3
            local.get 4
            i32.const 3
            i32.and
            br_if 0 (;@3;)
            local.get 2
            i32.const 256
            i32.lt_u
            br_if 1 (;@2;)
            local.get 5
            local.get 2
            i32.const 4
            i32.or
            i32.lt_u
            br_if 1 (;@2;)
            local.get 5
            local.get 2
            i32.sub
            i32.const 0
            i32.load offset=1049084
            i32.const 1
            i32.shl
            i32.le_u
            br_if 2 (;@1;)
            br 1 (;@2;)
          end
          local.get 0
          i32.const -8
          i32.add
          local.tee 6
          local.get 5
          i32.add
          local.set 7
          block ;; label = @3
            local.get 5
            local.get 2
            i32.lt_u
            br_if 0 (;@3;)
            local.get 5
            local.get 2
            i32.sub
            local.tee 1
            i32.const 16
            i32.lt_u
            br_if 2 (;@1;)
            local.get 3
            local.get 2
            local.get 4
            i32.const 1
            i32.and
            i32.or
            i32.const 2
            i32.or
            i32.store
            local.get 6
            local.get 2
            i32.add
            local.tee 2
            local.get 1
            i32.const 3
            i32.or
            i32.store offset=4
            local.get 7
            local.get 7
            i32.load offset=4
            i32.const 1
            i32.or
            i32.store offset=4
            local.get 2
            local.get 1
            call $dispose_chunk
            local.get 0
            return
          end
          block ;; label = @3
            local.get 7
            i32.const 0
            i32.load offset=1048628
            i32.ne
            br_if 0 (;@3;)
            i32.const 0
            i32.load offset=1048616
            local.get 5
            i32.add
            local.tee 5
            local.get 2
            i32.le_u
            br_if 1 (;@2;)
            local.get 3
            local.get 2
            local.get 4
            i32.const 1
            i32.and
            i32.or
            i32.const 2
            i32.or
            i32.store
            i32.const 0
            local.get 6
            local.get 2
            i32.add
            local.tee 1
            i32.store offset=1048628
            i32.const 0
            local.get 5
            local.get 2
            i32.sub
            local.tee 2
            i32.store offset=1048616
            local.get 1
            local.get 2
            i32.const 1
            i32.or
            i32.store offset=4
            local.get 0
            return
          end
          block ;; label = @3
            local.get 7
            i32.const 0
            i32.load offset=1048624
            i32.ne
            br_if 0 (;@3;)
            i32.const 0
            i32.load offset=1048612
            local.get 5
            i32.add
            local.tee 5
            local.get 2
            i32.lt_u
            br_if 1 (;@2;)
            block ;; label = @4
              block ;; label = @5
                local.get 5
                local.get 2
                i32.sub
                local.tee 1
                i32.const 16
                i32.lt_u
                br_if 0 (;@5;)
                local.get 3
                local.get 2
                local.get 4
                i32.const 1
                i32.and
                i32.or
                i32.const 2
                i32.or
                i32.store
                local.get 6
                local.get 2
                i32.add
                local.tee 2
                local.get 1
                i32.const 1
                i32.or
                i32.store offset=4
                local.get 6
                local.get 5
                i32.add
                local.tee 5
                local.get 1
                i32.store
                local.get 5
                local.get 5
                i32.load offset=4
                i32.const -2
                i32.and
                i32.store offset=4
                br 1 (;@4;)
              end
              local.get 3
              local.get 4
              i32.const 1
              i32.and
              local.get 5
              i32.or
              i32.const 2
              i32.or
              i32.store
              local.get 6
              local.get 5
              i32.add
              local.tee 1
              local.get 1
              i32.load offset=4
              i32.const 1
              i32.or
              i32.store offset=4
              i32.const 0
              local.set 1
              i32.const 0
              local.set 2
            end
            i32.const 0
            local.get 2
            i32.store offset=1048624
            i32.const 0
            local.get 1
            i32.store offset=1048612
            local.get 0
            return
          end
          local.get 7
          i32.load offset=4
          local.tee 8
          i32.const 2
          i32.and
          br_if 0 (;@2;)
          local.get 8
          i32.const -8
          i32.and
          local.get 5
          i32.add
          local.tee 9
          local.get 2
          i32.lt_u
          br_if 0 (;@2;)
          local.get 9
          local.get 2
          i32.sub
          local.set 10
          local.get 7
          i32.load offset=12
          local.set 1
          block ;; label = @3
            block ;; label = @4
              local.get 8
              i32.const 255
              i32.gt_u
              br_if 0 (;@4;)
              block ;; label = @5
                local.get 1
                local.get 7
                i32.load offset=8
                local.tee 5
                i32.ne
                br_if 0 (;@5;)
                i32.const 0
                i32.const 0
                i32.load offset=1048604
                i32.const -2
                local.get 8
                i32.const 3
                i32.shr_u
                i32.rotl
                i32.and
                i32.store offset=1048604
                br 2 (;@3;)
              end
              local.get 1
              local.get 5
              i32.store offset=8
              local.get 5
              local.get 1
              i32.store offset=12
              br 1 (;@3;)
            end
            local.get 7
            i32.load offset=24
            local.set 11
            block ;; label = @4
              block ;; label = @5
                local.get 1
                local.get 7
                i32.eq
                br_if 0 (;@5;)
                local.get 7
                i32.load offset=8
                local.tee 5
                local.get 1
                i32.store offset=12
                local.get 1
                local.get 5
                i32.store offset=8
                br 1 (;@4;)
              end
              block ;; label = @5
                block ;; label = @6
                  block ;; label = @7
                    local.get 7
                    i32.load offset=20
                    local.tee 5
                    i32.eqz
                    br_if 0 (;@7;)
                    local.get 7
                    i32.const 20
                    i32.add
                    local.set 8
                    br 1 (;@6;)
                  end
                  local.get 7
                  i32.load offset=16
                  local.tee 5
                  i32.eqz
                  br_if 1 (;@5;)
                  local.get 7
                  i32.const 16
                  i32.add
                  local.set 8
                end
                loop ;; label = @6
                  local.get 8
                  local.set 12
                  local.get 5
                  local.tee 1
                  i32.const 20
                  i32.add
                  local.set 8
                  local.get 1
                  i32.load offset=20
                  local.tee 5
                  br_if 0 (;@6;)
                  local.get 1
                  i32.const 16
                  i32.add
                  local.set 8
                  local.get 1
                  i32.load offset=16
                  local.tee 5
                  br_if 0 (;@6;)
                end
                local.get 12
                i32.const 0
                i32.store
                br 1 (;@4;)
              end
              i32.const 0
              local.set 1
            end
            local.get 11
            i32.eqz
            br_if 0 (;@3;)
            block ;; label = @4
              block ;; label = @5
                local.get 7
                local.get 7
                i32.load offset=28
                local.tee 8
                i32.const 2
                i32.shl
                i32.const 1048908
                i32.add
                local.tee 5
                i32.load
                i32.ne
                br_if 0 (;@5;)
                local.get 5
                local.get 1
                i32.store
                local.get 1
                br_if 1 (;@4;)
                i32.const 0
                i32.const 0
                i32.load offset=1048608
                i32.const -2
                local.get 8
                i32.rotl
                i32.and
                i32.store offset=1048608
                br 2 (;@3;)
              end
              local.get 11
              i32.const 16
              i32.const 20
              local.get 11
              i32.load offset=16
              local.get 7
              i32.eq
              select
              i32.add
              local.get 1
              i32.store
              local.get 1
              i32.eqz
              br_if 1 (;@3;)
            end
            local.get 1
            local.get 11
            i32.store offset=24
            block ;; label = @4
              local.get 7
              i32.load offset=16
              local.tee 5
              i32.eqz
              br_if 0 (;@4;)
              local.get 1
              local.get 5
              i32.store offset=16
              local.get 5
              local.get 1
              i32.store offset=24
            end
            local.get 7
            i32.load offset=20
            local.tee 5
            i32.eqz
            br_if 0 (;@3;)
            local.get 1
            local.get 5
            i32.store offset=20
            local.get 5
            local.get 1
            i32.store offset=24
          end
          block ;; label = @3
            local.get 10
            i32.const 15
            i32.gt_u
            br_if 0 (;@3;)
            local.get 3
            local.get 4
            i32.const 1
            i32.and
            local.get 9
            i32.or
            i32.const 2
            i32.or
            i32.store
            local.get 6
            local.get 9
            i32.add
            local.tee 1
            local.get 1
            i32.load offset=4
            i32.const 1
            i32.or
            i32.store offset=4
            local.get 0
            return
          end
          local.get 3
          local.get 2
          local.get 4
          i32.const 1
          i32.and
          i32.or
          i32.const 2
          i32.or
          i32.store
          local.get 6
          local.get 2
          i32.add
          local.tee 1
          local.get 10
          i32.const 3
          i32.or
          i32.store offset=4
          local.get 6
          local.get 9
          i32.add
          local.tee 2
          local.get 2
          i32.load offset=4
          i32.const 1
          i32.or
          i32.store offset=4
          local.get 1
          local.get 10
          call $dispose_chunk
          local.get 0
          return
        end
        block ;; label = @2
          local.get 1
          call $dlmalloc
          local.tee 2
          br_if 0 (;@2;)
          i32.const 0
          return
        end
        local.get 2
        local.get 0
        i32.const -4
        i32.const -8
        local.get 3
        i32.load
        local.tee 5
        i32.const 3
        i32.and
        select
        local.get 5
        i32.const -8
        i32.and
        i32.add
        local.tee 5
        local.get 1
        local.get 5
        local.get 1
        i32.lt_u
        select
        call $memcpy
        local.set 1
        local.get 0
        call $dlfree
        local.get 1
        local.set 0
      end
      local.get 0
    )
    (func $dispose_chunk (;12;) (type 8) (param i32 i32)
      (local i32 i32 i32 i32 i32 i32)
      local.get 0
      local.get 1
      i32.add
      local.set 2
      block ;; label = @1
        block ;; label = @2
          local.get 0
          i32.load offset=4
          local.tee 3
          i32.const 1
          i32.and
          br_if 0 (;@2;)
          local.get 3
          i32.const 2
          i32.and
          i32.eqz
          br_if 1 (;@1;)
          local.get 0
          i32.load
          local.tee 4
          local.get 1
          i32.add
          local.set 1
          block ;; label = @3
            block ;; label = @4
              block ;; label = @5
                block ;; label = @6
                  local.get 0
                  local.get 4
                  i32.sub
                  local.tee 0
                  i32.const 0
                  i32.load offset=1048624
                  i32.eq
                  br_if 0 (;@6;)
                  local.get 0
                  i32.load offset=12
                  local.set 3
                  block ;; label = @7
                    local.get 4
                    i32.const 255
                    i32.gt_u
                    br_if 0 (;@7;)
                    local.get 3
                    local.get 0
                    i32.load offset=8
                    local.tee 5
                    i32.ne
                    br_if 2 (;@5;)
                    i32.const 0
                    i32.const 0
                    i32.load offset=1048604
                    i32.const -2
                    local.get 4
                    i32.const 3
                    i32.shr_u
                    i32.rotl
                    i32.and
                    i32.store offset=1048604
                    br 5 (;@2;)
                  end
                  local.get 0
                  i32.load offset=24
                  local.set 6
                  block ;; label = @7
                    local.get 3
                    local.get 0
                    i32.eq
                    br_if 0 (;@7;)
                    local.get 0
                    i32.load offset=8
                    local.tee 4
                    local.get 3
                    i32.store offset=12
                    local.get 3
                    local.get 4
                    i32.store offset=8
                    br 4 (;@3;)
                  end
                  block ;; label = @7
                    block ;; label = @8
                      local.get 0
                      i32.load offset=20
                      local.tee 4
                      i32.eqz
                      br_if 0 (;@8;)
                      local.get 0
                      i32.const 20
                      i32.add
                      local.set 5
                      br 1 (;@7;)
                    end
                    local.get 0
                    i32.load offset=16
                    local.tee 4
                    i32.eqz
                    br_if 3 (;@4;)
                    local.get 0
                    i32.const 16
                    i32.add
                    local.set 5
                  end
                  loop ;; label = @7
                    local.get 5
                    local.set 7
                    local.get 4
                    local.tee 3
                    i32.const 20
                    i32.add
                    local.set 5
                    local.get 3
                    i32.load offset=20
                    local.tee 4
                    br_if 0 (;@7;)
                    local.get 3
                    i32.const 16
                    i32.add
                    local.set 5
                    local.get 3
                    i32.load offset=16
                    local.tee 4
                    br_if 0 (;@7;)
                  end
                  local.get 7
                  i32.const 0
                  i32.store
                  br 3 (;@3;)
                end
                local.get 2
                i32.load offset=4
                local.tee 3
                i32.const 3
                i32.and
                i32.const 3
                i32.ne
                br_if 3 (;@2;)
                local.get 2
                local.get 3
                i32.const -2
                i32.and
                i32.store offset=4
                i32.const 0
                local.get 1
                i32.store offset=1048612
                local.get 2
                local.get 1
                i32.store
                local.get 0
                local.get 1
                i32.const 1
                i32.or
                i32.store offset=4
                return
              end
              local.get 3
              local.get 5
              i32.store offset=8
              local.get 5
              local.get 3
              i32.store offset=12
              br 2 (;@2;)
            end
            i32.const 0
            local.set 3
          end
          local.get 6
          i32.eqz
          br_if 0 (;@2;)
          block ;; label = @3
            block ;; label = @4
              local.get 0
              local.get 0
              i32.load offset=28
              local.tee 5
              i32.const 2
              i32.shl
              i32.const 1048908
              i32.add
              local.tee 4
              i32.load
              i32.ne
              br_if 0 (;@4;)
              local.get 4
              local.get 3
              i32.store
              local.get 3
              br_if 1 (;@3;)
              i32.const 0
              i32.const 0
              i32.load offset=1048608
              i32.const -2
              local.get 5
              i32.rotl
              i32.and
              i32.store offset=1048608
              br 2 (;@2;)
            end
            local.get 6
            i32.const 16
            i32.const 20
            local.get 6
            i32.load offset=16
            local.get 0
            i32.eq
            select
            i32.add
            local.get 3
            i32.store
            local.get 3
            i32.eqz
            br_if 1 (;@2;)
          end
          local.get 3
          local.get 6
          i32.store offset=24
          block ;; label = @3
            local.get 0
            i32.load offset=16
            local.tee 4
            i32.eqz
            br_if 0 (;@3;)
            local.get 3
            local.get 4
            i32.store offset=16
            local.get 4
            local.get 3
            i32.store offset=24
          end
          local.get 0
          i32.load offset=20
          local.tee 4
          i32.eqz
          br_if 0 (;@2;)
          local.get 3
          local.get 4
          i32.store offset=20
          local.get 4
          local.get 3
          i32.store offset=24
        end
        block ;; label = @2
          block ;; label = @3
            block ;; label = @4
              block ;; label = @5
                block ;; label = @6
                  local.get 2
                  i32.load offset=4
                  local.tee 4
                  i32.const 2
                  i32.and
                  br_if 0 (;@6;)
                  block ;; label = @7
                    local.get 2
                    i32.const 0
                    i32.load offset=1048628
                    i32.ne
                    br_if 0 (;@7;)
                    i32.const 0
                    local.get 0
                    i32.store offset=1048628
                    i32.const 0
                    i32.const 0
                    i32.load offset=1048616
                    local.get 1
                    i32.add
                    local.tee 1
                    i32.store offset=1048616
                    local.get 0
                    local.get 1
                    i32.const 1
                    i32.or
                    i32.store offset=4
                    local.get 0
                    i32.const 0
                    i32.load offset=1048624
                    i32.ne
                    br_if 6 (;@1;)
                    i32.const 0
                    i32.const 0
                    i32.store offset=1048612
                    i32.const 0
                    i32.const 0
                    i32.store offset=1048624
                    return
                  end
                  block ;; label = @7
                    local.get 2
                    i32.const 0
                    i32.load offset=1048624
                    i32.ne
                    br_if 0 (;@7;)
                    i32.const 0
                    local.get 0
                    i32.store offset=1048624
                    i32.const 0
                    i32.const 0
                    i32.load offset=1048612
                    local.get 1
                    i32.add
                    local.tee 1
                    i32.store offset=1048612
                    local.get 0
                    local.get 1
                    i32.const 1
                    i32.or
                    i32.store offset=4
                    local.get 0
                    local.get 1
                    i32.add
                    local.get 1
                    i32.store
                    return
                  end
                  local.get 4
                  i32.const -8
                  i32.and
                  local.get 1
                  i32.add
                  local.set 1
                  local.get 2
                  i32.load offset=12
                  local.set 3
                  block ;; label = @7
                    local.get 4
                    i32.const 255
                    i32.gt_u
                    br_if 0 (;@7;)
                    block ;; label = @8
                      local.get 3
                      local.get 2
                      i32.load offset=8
                      local.tee 5
                      i32.ne
                      br_if 0 (;@8;)
                      i32.const 0
                      i32.const 0
                      i32.load offset=1048604
                      i32.const -2
                      local.get 4
                      i32.const 3
                      i32.shr_u
                      i32.rotl
                      i32.and
                      i32.store offset=1048604
                      br 5 (;@3;)
                    end
                    local.get 3
                    local.get 5
                    i32.store offset=8
                    local.get 5
                    local.get 3
                    i32.store offset=12
                    br 4 (;@3;)
                  end
                  local.get 2
                  i32.load offset=24
                  local.set 6
                  block ;; label = @7
                    local.get 3
                    local.get 2
                    i32.eq
                    br_if 0 (;@7;)
                    local.get 2
                    i32.load offset=8
                    local.tee 4
                    local.get 3
                    i32.store offset=12
                    local.get 3
                    local.get 4
                    i32.store offset=8
                    br 3 (;@4;)
                  end
                  block ;; label = @7
                    block ;; label = @8
                      local.get 2
                      i32.load offset=20
                      local.tee 4
                      i32.eqz
                      br_if 0 (;@8;)
                      local.get 2
                      i32.const 20
                      i32.add
                      local.set 5
                      br 1 (;@7;)
                    end
                    local.get 2
                    i32.load offset=16
                    local.tee 4
                    i32.eqz
                    br_if 2 (;@5;)
                    local.get 2
                    i32.const 16
                    i32.add
                    local.set 5
                  end
                  loop ;; label = @7
                    local.get 5
                    local.set 7
                    local.get 4
                    local.tee 3
                    i32.const 20
                    i32.add
                    local.set 5
                    local.get 3
                    i32.load offset=20
                    local.tee 4
                    br_if 0 (;@7;)
                    local.get 3
                    i32.const 16
                    i32.add
                    local.set 5
                    local.get 3
                    i32.load offset=16
                    local.tee 4
                    br_if 0 (;@7;)
                  end
                  local.get 7
                  i32.const 0
                  i32.store
                  br 2 (;@4;)
                end
                local.get 2
                local.get 4
                i32.const -2
                i32.and
                i32.store offset=4
                local.get 0
                local.get 1
                i32.add
                local.get 1
                i32.store
                local.get 0
                local.get 1
                i32.const 1
                i32.or
                i32.store offset=4
                br 3 (;@2;)
              end
              i32.const 0
              local.set 3
            end
            local.get 6
            i32.eqz
            br_if 0 (;@3;)
            block ;; label = @4
              block ;; label = @5
                local.get 2
                local.get 2
                i32.load offset=28
                local.tee 5
                i32.const 2
                i32.shl
                i32.const 1048908
                i32.add
                local.tee 4
                i32.load
                i32.ne
                br_if 0 (;@5;)
                local.get 4
                local.get 3
                i32.store
                local.get 3
                br_if 1 (;@4;)
                i32.const 0
                i32.const 0
                i32.load offset=1048608
                i32.const -2
                local.get 5
                i32.rotl
                i32.and
                i32.store offset=1048608
                br 2 (;@3;)
              end
              local.get 6
              i32.const 16
              i32.const 20
              local.get 6
              i32.load offset=16
              local.get 2
              i32.eq
              select
              i32.add
              local.get 3
              i32.store
              local.get 3
              i32.eqz
              br_if 1 (;@3;)
            end
            local.get 3
            local.get 6
            i32.store offset=24
            block ;; label = @4
              local.get 2
              i32.load offset=16
              local.tee 4
              i32.eqz
              br_if 0 (;@4;)
              local.get 3
              local.get 4
              i32.store offset=16
              local.get 4
              local.get 3
              i32.store offset=24
            end
            local.get 2
            i32.load offset=20
            local.tee 4
            i32.eqz
            br_if 0 (;@3;)
            local.get 3
            local.get 4
            i32.store offset=20
            local.get 4
            local.get 3
            i32.store offset=24
          end
          local.get 0
          local.get 1
          i32.add
          local.get 1
          i32.store
          local.get 0
          local.get 1
          i32.const 1
          i32.or
          i32.store offset=4
          local.get 0
          i32.const 0
          i32.load offset=1048624
          i32.ne
          br_if 0 (;@2;)
          i32.const 0
          local.get 1
          i32.store offset=1048612
          return
        end
        block ;; label = @2
          local.get 1
          i32.const 255
          i32.gt_u
          br_if 0 (;@2;)
          local.get 1
          i32.const -8
          i32.and
          i32.const 1048644
          i32.add
          local.set 3
          block ;; label = @3
            block ;; label = @4
              i32.const 0
              i32.load offset=1048604
              local.tee 4
              i32.const 1
              local.get 1
              i32.const 3
              i32.shr_u
              i32.shl
              local.tee 1
              i32.and
              br_if 0 (;@4;)
              i32.const 0
              local.get 4
              local.get 1
              i32.or
              i32.store offset=1048604
              local.get 3
              local.set 1
              br 1 (;@3;)
            end
            local.get 3
            i32.load offset=8
            local.set 1
          end
          local.get 1
          local.get 0
          i32.store offset=12
          local.get 3
          local.get 0
          i32.store offset=8
          local.get 0
          local.get 3
          i32.store offset=12
          local.get 0
          local.get 1
          i32.store offset=8
          return
        end
        i32.const 31
        local.set 3
        block ;; label = @2
          local.get 1
          i32.const 16777215
          i32.gt_u
          br_if 0 (;@2;)
          local.get 1
          i32.const 38
          local.get 1
          i32.const 8
          i32.shr_u
          i32.clz
          local.tee 3
          i32.sub
          i32.shr_u
          i32.const 1
          i32.and
          local.get 3
          i32.const 1
          i32.shl
          i32.sub
          i32.const 62
          i32.add
          local.set 3
        end
        local.get 0
        local.get 3
        i32.store offset=28
        local.get 0
        i64.const 0
        i64.store offset=16 align=4
        local.get 3
        i32.const 2
        i32.shl
        i32.const 1048908
        i32.add
        local.set 4
        block ;; label = @2
          i32.const 0
          i32.load offset=1048608
          local.tee 5
          i32.const 1
          local.get 3
          i32.shl
          local.tee 2
          i32.and
          br_if 0 (;@2;)
          local.get 4
          local.get 0
          i32.store
          i32.const 0
          local.get 5
          local.get 2
          i32.or
          i32.store offset=1048608
          local.get 0
          local.get 4
          i32.store offset=24
          local.get 0
          local.get 0
          i32.store offset=8
          local.get 0
          local.get 0
          i32.store offset=12
          return
        end
        local.get 1
        i32.const 0
        i32.const 25
        local.get 3
        i32.const 1
        i32.shr_u
        i32.sub
        local.get 3
        i32.const 31
        i32.eq
        select
        i32.shl
        local.set 3
        local.get 4
        i32.load
        local.set 5
        block ;; label = @2
          loop ;; label = @3
            local.get 5
            local.tee 4
            i32.load offset=4
            i32.const -8
            i32.and
            local.get 1
            i32.eq
            br_if 1 (;@2;)
            local.get 3
            i32.const 29
            i32.shr_u
            local.set 5
            local.get 3
            i32.const 1
            i32.shl
            local.set 3
            local.get 4
            local.get 5
            i32.const 4
            i32.and
            i32.add
            i32.const 16
            i32.add
            local.tee 2
            i32.load
            local.tee 5
            br_if 0 (;@3;)
          end
          local.get 2
          local.get 0
          i32.store
          local.get 0
          local.get 4
          i32.store offset=24
          local.get 0
          local.get 0
          i32.store offset=12
          local.get 0
          local.get 0
          i32.store offset=8
          return
        end
        local.get 4
        i32.load offset=8
        local.tee 1
        local.get 0
        i32.store offset=12
        local.get 4
        local.get 0
        i32.store offset=8
        local.get 0
        i32.const 0
        i32.store offset=24
        local.get 0
        local.get 4
        i32.store offset=12
        local.get 0
        local.get 1
        i32.store offset=8
      end
    )
    (func $posix_memalign (;13;) (type 5) (param i32 i32 i32) (result i32)
      (local i32 i32)
      block ;; label = @1
        block ;; label = @2
          block ;; label = @3
            local.get 1
            i32.const 16
            i32.ne
            br_if 0 (;@3;)
            local.get 2
            call $dlmalloc
            local.set 1
            br 1 (;@2;)
          end
          i32.const 28
          local.set 3
          local.get 1
          i32.const 4
          i32.lt_u
          br_if 1 (;@1;)
          local.get 1
          i32.const 3
          i32.and
          br_if 1 (;@1;)
          local.get 1
          i32.const 2
          i32.shr_u
          local.tee 4
          local.get 4
          i32.const -1
          i32.add
          i32.and
          br_if 1 (;@1;)
          block ;; label = @3
            i32.const -64
            local.get 1
            i32.sub
            local.get 2
            i32.ge_u
            br_if 0 (;@3;)
            i32.const 48
            return
          end
          local.get 1
          i32.const 16
          local.get 1
          i32.const 16
          i32.gt_u
          select
          local.get 2
          call $internal_memalign
          local.set 1
        end
        block ;; label = @2
          local.get 1
          br_if 0 (;@2;)
          i32.const 48
          return
        end
        local.get 0
        local.get 1
        i32.store
        i32.const 0
        local.set 3
      end
      local.get 3
    )
    (func $internal_memalign (;14;) (type 7) (param i32 i32) (result i32)
      (local i32 i32 i32 i32 i32)
      block ;; label = @1
        block ;; label = @2
          local.get 0
          i32.const 16
          local.get 0
          i32.const 16
          i32.gt_u
          select
          local.tee 2
          local.get 2
          i32.const -1
          i32.add
          i32.and
          br_if 0 (;@2;)
          local.get 2
          local.set 0
          br 1 (;@1;)
        end
        i32.const 32
        local.set 3
        loop ;; label = @2
          local.get 3
          local.tee 0
          i32.const 1
          i32.shl
          local.set 3
          local.get 0
          local.get 2
          i32.lt_u
          br_if 0 (;@2;)
        end
      end
      block ;; label = @1
        i32.const -64
        local.get 0
        i32.sub
        local.get 1
        i32.gt_u
        br_if 0 (;@1;)
        i32.const 0
        i32.const 48
        i32.store offset=1049100
        i32.const 0
        return
      end
      block ;; label = @1
        local.get 0
        i32.const 16
        local.get 1
        i32.const 19
        i32.add
        i32.const -16
        i32.and
        local.get 1
        i32.const 11
        i32.lt_u
        select
        local.tee 1
        i32.add
        i32.const 12
        i32.add
        call $dlmalloc
        local.tee 3
        br_if 0 (;@1;)
        i32.const 0
        return
      end
      local.get 3
      i32.const -8
      i32.add
      local.set 2
      block ;; label = @1
        block ;; label = @2
          local.get 0
          i32.const -1
          i32.add
          local.get 3
          i32.and
          br_if 0 (;@2;)
          local.get 2
          local.set 0
          br 1 (;@1;)
        end
        local.get 3
        i32.const -4
        i32.add
        local.tee 4
        i32.load
        local.tee 5
        i32.const -8
        i32.and
        local.get 3
        local.get 0
        i32.add
        i32.const -1
        i32.add
        i32.const 0
        local.get 0
        i32.sub
        i32.and
        i32.const -8
        i32.add
        local.tee 3
        i32.const 0
        local.get 0
        local.get 3
        local.get 2
        i32.sub
        i32.const 15
        i32.gt_u
        select
        i32.add
        local.tee 0
        local.get 2
        i32.sub
        local.tee 3
        i32.sub
        local.set 6
        block ;; label = @2
          local.get 5
          i32.const 3
          i32.and
          br_if 0 (;@2;)
          local.get 0
          local.get 6
          i32.store offset=4
          local.get 0
          local.get 2
          i32.load
          local.get 3
          i32.add
          i32.store
          br 1 (;@1;)
        end
        local.get 0
        local.get 6
        local.get 0
        i32.load offset=4
        i32.const 1
        i32.and
        i32.or
        i32.const 2
        i32.or
        i32.store offset=4
        local.get 0
        local.get 6
        i32.add
        local.tee 6
        local.get 6
        i32.load offset=4
        i32.const 1
        i32.or
        i32.store offset=4
        local.get 4
        local.get 3
        local.get 4
        i32.load
        i32.const 1
        i32.and
        i32.or
        i32.const 2
        i32.or
        i32.store
        local.get 2
        local.get 3
        i32.add
        local.tee 6
        local.get 6
        i32.load offset=4
        i32.const 1
        i32.or
        i32.store offset=4
        local.get 2
        local.get 3
        call $dispose_chunk
      end
      block ;; label = @1
        local.get 0
        i32.load offset=4
        local.tee 3
        i32.const 3
        i32.and
        i32.eqz
        br_if 0 (;@1;)
        local.get 3
        i32.const -8
        i32.and
        local.tee 2
        local.get 1
        i32.const 16
        i32.add
        i32.le_u
        br_if 0 (;@1;)
        local.get 0
        local.get 1
        local.get 3
        i32.const 1
        i32.and
        i32.or
        i32.const 2
        i32.or
        i32.store offset=4
        local.get 0
        local.get 1
        i32.add
        local.tee 3
        local.get 2
        local.get 1
        i32.sub
        local.tee 1
        i32.const 3
        i32.or
        i32.store offset=4
        local.get 0
        local.get 2
        i32.add
        local.tee 2
        local.get 2
        i32.load offset=4
        i32.const 1
        i32.or
        i32.store offset=4
        local.get 3
        local.get 1
        call $dispose_chunk
      end
      local.get 0
      i32.const 8
      i32.add
    )
    (func $abort (;15;) (type 0)
      unreachable
    )
    (func $sbrk (;16;) (type 4) (param i32) (result i32)
      block ;; label = @1
        local.get 0
        br_if 0 (;@1;)
        memory.size
        i32.const 16
        i32.shl
        return
      end
      block ;; label = @1
        local.get 0
        i32.const 65535
        i32.and
        br_if 0 (;@1;)
        local.get 0
        i32.const -1
        i32.le_s
        br_if 0 (;@1;)
        block ;; label = @2
          local.get 0
          i32.const 16
          i32.shr_u
          memory.grow
          local.tee 0
          i32.const -1
          i32.ne
          br_if 0 (;@2;)
          i32.const 0
          i32.const 48
          i32.store offset=1049100
          i32.const -1
          return
        end
        local.get 0
        i32.const 16
        i32.shl
        return
      end
      call $abort
      unreachable
    )
    (func $memcpy (;17;) (type 5) (param i32 i32 i32) (result i32)
      (local i32 i32 i32 i32)
      block ;; label = @1
        block ;; label = @2
          block ;; label = @3
            local.get 2
            i32.const 32
            i32.gt_u
            br_if 0 (;@3;)
            local.get 1
            i32.const 3
            i32.and
            i32.eqz
            br_if 1 (;@2;)
            local.get 2
            i32.eqz
            br_if 1 (;@2;)
            local.get 0
            local.get 1
            i32.load8_u
            i32.store8
            local.get 2
            i32.const -1
            i32.add
            local.set 3
            local.get 0
            i32.const 1
            i32.add
            local.set 4
            local.get 1
            i32.const 1
            i32.add
            local.tee 5
            i32.const 3
            i32.and
            i32.eqz
            br_if 2 (;@1;)
            local.get 3
            i32.eqz
            br_if 2 (;@1;)
            local.get 0
            local.get 1
            i32.load8_u offset=1
            i32.store8 offset=1
            local.get 2
            i32.const -2
            i32.add
            local.set 3
            local.get 0
            i32.const 2
            i32.add
            local.set 4
            local.get 1
            i32.const 2
            i32.add
            local.tee 5
            i32.const 3
            i32.and
            i32.eqz
            br_if 2 (;@1;)
            local.get 3
            i32.eqz
            br_if 2 (;@1;)
            local.get 0
            local.get 1
            i32.load8_u offset=2
            i32.store8 offset=2
            local.get 2
            i32.const -3
            i32.add
            local.set 3
            local.get 0
            i32.const 3
            i32.add
            local.set 4
            local.get 1
            i32.const 3
            i32.add
            local.tee 5
            i32.const 3
            i32.and
            i32.eqz
            br_if 2 (;@1;)
            local.get 3
            i32.eqz
            br_if 2 (;@1;)
            local.get 0
            local.get 1
            i32.load8_u offset=3
            i32.store8 offset=3
            local.get 2
            i32.const -4
            i32.add
            local.set 3
            local.get 0
            i32.const 4
            i32.add
            local.set 4
            local.get 1
            i32.const 4
            i32.add
            local.set 5
            br 2 (;@1;)
          end
          local.get 0
          local.get 1
          local.get 2
          memory.copy
          local.get 0
          return
        end
        local.get 2
        local.set 3
        local.get 0
        local.set 4
        local.get 1
        local.set 5
      end
      block ;; label = @1
        block ;; label = @2
          local.get 4
          i32.const 3
          i32.and
          local.tee 2
          br_if 0 (;@2;)
          block ;; label = @3
            block ;; label = @4
              local.get 3
              i32.const 16
              i32.ge_u
              br_if 0 (;@4;)
              local.get 3
              local.set 2
              br 1 (;@3;)
            end
            block ;; label = @4
              local.get 3
              i32.const -16
              i32.add
              local.tee 2
              i32.const 16
              i32.and
              br_if 0 (;@4;)
              local.get 4
              local.get 5
              i64.load align=4
              i64.store align=4
              local.get 4
              local.get 5
              i64.load offset=8 align=4
              i64.store offset=8 align=4
              local.get 4
              i32.const 16
              i32.add
              local.set 4
              local.get 5
              i32.const 16
              i32.add
              local.set 5
              local.get 2
              local.set 3
            end
            local.get 2
            i32.const 16
            i32.lt_u
            br_if 0 (;@3;)
            local.get 3
            local.set 2
            loop ;; label = @4
              local.get 4
              local.get 5
              i64.load align=4
              i64.store align=4
              local.get 4
              local.get 5
              i64.load offset=8 align=4
              i64.store offset=8 align=4
              local.get 4
              local.get 5
              i64.load offset=16 align=4
              i64.store offset=16 align=4
              local.get 4
              local.get 5
              i64.load offset=24 align=4
              i64.store offset=24 align=4
              local.get 4
              i32.const 32
              i32.add
              local.set 4
              local.get 5
              i32.const 32
              i32.add
              local.set 5
              local.get 2
              i32.const -32
              i32.add
              local.tee 2
              i32.const 15
              i32.gt_u
              br_if 0 (;@4;)
            end
          end
          block ;; label = @3
            local.get 2
            i32.const 8
            i32.lt_u
            br_if 0 (;@3;)
            local.get 4
            local.get 5
            i64.load align=4
            i64.store align=4
            local.get 5
            i32.const 8
            i32.add
            local.set 5
            local.get 4
            i32.const 8
            i32.add
            local.set 4
          end
          block ;; label = @3
            local.get 2
            i32.const 4
            i32.and
            i32.eqz
            br_if 0 (;@3;)
            local.get 4
            local.get 5
            i32.load
            i32.store
            local.get 5
            i32.const 4
            i32.add
            local.set 5
            local.get 4
            i32.const 4
            i32.add
            local.set 4
          end
          block ;; label = @3
            local.get 2
            i32.const 2
            i32.and
            i32.eqz
            br_if 0 (;@3;)
            local.get 4
            local.get 5
            i32.load16_u align=1
            i32.store16 align=1
            local.get 4
            i32.const 2
            i32.add
            local.set 4
            local.get 5
            i32.const 2
            i32.add
            local.set 5
          end
          local.get 2
          i32.const 1
          i32.and
          i32.eqz
          br_if 1 (;@1;)
          local.get 4
          local.get 5
          i32.load8_u
          i32.store8
          local.get 0
          return
        end
        block ;; label = @2
          block ;; label = @3
            block ;; label = @4
              block ;; label = @5
                block ;; label = @6
                  local.get 3
                  i32.const 32
                  i32.lt_u
                  br_if 0 (;@6;)
                  local.get 4
                  local.get 5
                  i32.load
                  local.tee 3
                  i32.store8
                  block ;; label = @7
                    block ;; label = @8
                      local.get 2
                      i32.const -1
                      i32.add
                      br_table 3 (;@5;) 0 (;@8;) 1 (;@7;) 3 (;@5;)
                    end
                    local.get 4
                    local.get 3
                    i32.const 8
                    i32.shr_u
                    i32.store8 offset=1
                    local.get 4
                    local.get 5
                    i32.const 6
                    i32.add
                    i64.load align=2
                    i64.store offset=6 align=4
                    local.get 4
                    local.get 5
                    i32.load offset=4
                    i32.const 16
                    i32.shl
                    local.get 3
                    i32.const 16
                    i32.shr_u
                    i32.or
                    i32.store offset=2
                    local.get 4
                    i32.const 18
                    i32.add
                    local.set 2
                    local.get 5
                    i32.const 18
                    i32.add
                    local.set 1
                    i32.const 14
                    local.set 6
                    local.get 5
                    i32.const 14
                    i32.add
                    i32.load align=2
                    local.set 5
                    i32.const 14
                    local.set 3
                    br 3 (;@4;)
                  end
                  local.get 4
                  local.get 5
                  i32.const 5
                  i32.add
                  i64.load align=1
                  i64.store offset=5 align=4
                  local.get 4
                  local.get 5
                  i32.load offset=4
                  i32.const 24
                  i32.shl
                  local.get 3
                  i32.const 8
                  i32.shr_u
                  i32.or
                  i32.store offset=1
                  local.get 4
                  i32.const 17
                  i32.add
                  local.set 2
                  local.get 5
                  i32.const 17
                  i32.add
                  local.set 1
                  i32.const 13
                  local.set 6
                  local.get 5
                  i32.const 13
                  i32.add
                  i32.load align=1
                  local.set 5
                  i32.const 15
                  local.set 3
                  br 2 (;@4;)
                end
                block ;; label = @6
                  block ;; label = @7
                    local.get 3
                    i32.const 16
                    i32.ge_u
                    br_if 0 (;@7;)
                    local.get 4
                    local.set 2
                    local.get 5
                    local.set 1
                    br 1 (;@6;)
                  end
                  local.get 4
                  local.get 5
                  i32.load8_u
                  i32.store8
                  local.get 4
                  local.get 5
                  i32.load offset=1 align=1
                  i32.store offset=1 align=1
                  local.get 4
                  local.get 5
                  i64.load offset=5 align=1
                  i64.store offset=5 align=1
                  local.get 4
                  local.get 5
                  i32.load16_u offset=13 align=1
                  i32.store16 offset=13 align=1
                  local.get 4
                  local.get 5
                  i32.load8_u offset=15
                  i32.store8 offset=15
                  local.get 4
                  i32.const 16
                  i32.add
                  local.set 2
                  local.get 5
                  i32.const 16
                  i32.add
                  local.set 1
                end
                local.get 3
                i32.const 8
                i32.and
                br_if 2 (;@3;)
                br 3 (;@2;)
              end
              local.get 4
              local.get 3
              i32.const 16
              i32.shr_u
              i32.store8 offset=2
              local.get 4
              local.get 3
              i32.const 8
              i32.shr_u
              i32.store8 offset=1
              local.get 4
              local.get 5
              i32.const 7
              i32.add
              i64.load align=1
              i64.store offset=7 align=4
              local.get 4
              local.get 5
              i32.load offset=4
              i32.const 8
              i32.shl
              local.get 3
              i32.const 24
              i32.shr_u
              i32.or
              i32.store offset=3
              local.get 4
              i32.const 19
              i32.add
              local.set 2
              local.get 5
              i32.const 19
              i32.add
              local.set 1
              i32.const 15
              local.set 6
              local.get 5
              i32.const 15
              i32.add
              i32.load align=1
              local.set 5
              i32.const 13
              local.set 3
            end
            local.get 4
            local.get 6
            i32.add
            local.get 5
            i32.store
          end
          local.get 2
          local.get 1
          i64.load align=1
          i64.store align=1
          local.get 2
          i32.const 8
          i32.add
          local.set 2
          local.get 1
          i32.const 8
          i32.add
          local.set 1
        end
        block ;; label = @2
          local.get 3
          i32.const 4
          i32.and
          i32.eqz
          br_if 0 (;@2;)
          local.get 2
          local.get 1
          i32.load align=1
          i32.store align=1
          local.get 2
          i32.const 4
          i32.add
          local.set 2
          local.get 1
          i32.const 4
          i32.add
          local.set 1
        end
        block ;; label = @2
          local.get 3
          i32.const 2
          i32.and
          i32.eqz
          br_if 0 (;@2;)
          local.get 2
          local.get 1
          i32.load16_u align=1
          i32.store16 align=1
          local.get 2
          i32.const 2
          i32.add
          local.set 2
          local.get 1
          i32.const 2
          i32.add
          local.set 1
        end
        local.get 3
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@1;)
        local.get 2
        local.get 1
        i32.load8_u
        i32.store8
      end
      local.get 0
    )
    (data $.data (;0;) (i32.const 1048576) "\01\00\00\00\01\00\00\00\01\00\00\00\01\00\00\00\01\00\00\00\02\00\00\00")
    (@producers
      (language "Rust" "")
      (language "C11" "")
      (processed-by "rustc" "1.89.0 (29483883e 2025-08-04)")
      (processed-by "clang" "19.1.5-wasi-sdk (https://github.com/llvm/llvm-project ab4b5a2db582958af1ee308a790cfdb42bd24720)")
      (processed-by "wit-component" "0.239.0")
      (processed-by "wit-bindgen-rust" "0.46.0")
    )
    (@custom "target_features" (after data) "\08+\0bbulk-memory+\0fbulk-memory-opt+\16call-indirect-overlong+\0amultivalue+\0fmutable-globals+\13nontrapping-fptoint+\0freference-types+\08sign-ext")
  )
  (alias export 0 "cron-event-tag" (type (;1;)))
  (alias export 0 "cron-tagged" (type (;2;)))
  (core instance (;0;) (instantiate 0))
  (alias core export 0 "memory" (core memory (;0;)))
  (type (;3;) (func (result bool)))
  (alias core export 0 "hermes:init/event#init" (core func (;0;)))
  (alias core export 0 "cabi_realloc" (core func (;1;)))
  (func (;0;) (type 3) (canon lift (core func 0)))
  (component (;0;)
    (type (;0;) (func (result bool)))
    (import "import-func-init" (func (;0;) (type 0)))
    (type (;1;) (func (result bool)))
    (export (;1;) "init" (func 0) (func (type 1)))
  )
  (instance (;1;) (instantiate 0
      (with "import-func-init" (func 0))
    )
  )
  (export (;2;) "hermes:init/event" (instance 1))
  (type (;4;) (func (param "event" 2) (param "last" bool) (result bool)))
  (alias core export 0 "hermes:cron/event#on-cron" (core func (;2;)))
  (func (;1;) (type 4) (canon lift (core func 2) (memory 0) (realloc 1) string-encoding=utf8))
  (alias export 0 "cron-event-tag" (type (;5;)))
  (alias export 0 "cron-sched" (type (;6;)))
  (alias export 0 "cron-tagged" (type (;7;)))
  (component (;1;)
    (type (;0;) string)
    (import "import-type-cron-event-tag" (type (;1;) (eq 0)))
    (type (;2;) string)
    (import "import-type-cron-sched" (type (;3;) (eq 2)))
    (type (;4;) (record (field "when" 3) (field "tag" 1)))
    (import "import-type-cron-tagged" (type (;5;) (eq 4)))
    (import "import-type-cron-tagged0" (type (;6;) (eq 5)))
    (type (;7;) (func (param "event" 6) (param "last" bool) (result bool)))
    (import "import-func-on-cron" (func (;0;) (type 7)))
    (export (;8;) "cron-event-tag" (type 1))
    (export (;9;) "cron-tagged" (type 5))
    (type (;10;) (func (param "event" 9) (param "last" bool) (result bool)))
    (export (;1;) "on-cron" (func 0) (func (type 10)))
  )
  (instance (;3;) (instantiate 1
      (with "import-func-on-cron" (func 1))
      (with "import-type-cron-event-tag" (type 5))
      (with "import-type-cron-sched" (type 6))
      (with "import-type-cron-tagged" (type 7))
      (with "import-type-cron-tagged0" (type 2))
    )
  )
  (export (;4;) "hermes:cron/event" (instance 3))
  (@producers
    (processed-by "wit-component" "0.229.0")
  )
)
        "#;

        let patcher = Patcher::from_str(HERMES_LIKE_WAT).unwrap();

        let WasmInternals {
            mut core_module,
            mut component_part,
            mut pre_core_component_part,
        } = patcher.core_and_component().unwrap();

        std::fs::write("core.wat", core_module).unwrap();
        std::fs::write("component.wat", component_part).unwrap();
        std::fs::write("pre_core_component_part.wat", pre_core_component_part).unwrap();

        let patched_wat = patcher.patch().unwrap();
        std::fs::write("patched.wat", &patched_wat).unwrap();

        let encoded = wat::parse_str(&patched_wat);
        match encoded {
            Ok(_) => println!("Success!"),
            Err(err) => println!("Err: {err}"),
        }
    }
}
