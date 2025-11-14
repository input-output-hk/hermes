//! Patcher for WASM files.
//! It uses the text representation of component model .wasm files (.wat) to inject
//! functions necessary for the linear memory snapshotting.
//!
//! The patching process is split into the following stages:
//! 1. Load the WASM file from binary .wasm of directly from a .wat string.
//! 2. Split the WASM into 3 distinct parts
//!    1. The component part that is defined before core modules (pre-component)
//!    2. The core modules
//!    3. The main component part
//! 3. Inject the core level functions into the first core module
//! 4. Inject the component level functions, types and aliases into the component part
//! 5. Stich the parts together and return the resulting WASM in .wat format.
//!
//! # Step 1: Load the WASM file from binary .wasm of directly from a .wat string.
//!
//! The WASM can be loaded from a binary file. In such case it is converted into
//! the .wat format using the `wasmprinter` crate and sent for further processing.
//! In case the WASM is provided as a .wat string, it is used directly after
//! being checked for syntax errors.
//!
//! # Step 2: Split the WASM into 3 distinct parts
//!
//! ## The extraction of the core module:
//! 1. Use the `(core module (;` string marker to discover an embedded core module
//! 2. Once tha marker is found, parse the string until the corresponding closing
//!    parenthesis ")" is found
//! 3. Store the entire core module section and keep looking for more core module markers
//! 4. Collect the list of all encountered core modules
//!
//! ## The extraction of the component part:
//! The entire section after the last encountered core module is considered to be the
//! component part.
//!
//! ## The extraction of the pre-component part:
//! The entire section before the first encountered core module is considered to be the
//! pre-component part.
//!
//! # Step 3: Inject the core level functions into the first core module
//!
//! This process involves:
//! 1. Injecting the core level functions into the *first* core module along with the
//!    necessary types and exports.
//! 2. Injecting code necessary to expose these functions on the component level.
//!
//! We inject a total of 3 functions:
//! 1. `get-memory-size() -> i32` - returns the number of memory pages of the linear
//!    memory
//! 2. `get-memory-raw-bytes(offset: i32) -> i64` - returns 8-bytes from the linear memory
//!    at a given offset
//! 3. `set-memory-raw-bytes(offset: i32, bytes: i64)` - writes 8-bytes to the linear
//!    memory at a given offset
//!
//! Each of this function is prefixed with a magic string to avoid name collisions with
//! existing functions.
//!
//! # Step 4: Inject the component level functions, types and aliases into the component part
//!
//! ## Discovery of the index of the next available type index in the core module
//! 1. Count the number of type markers `type (;` in the core module, assign to `X`
//! 2. Use `X+1` for the type of the `get-memory-size()` function
//! 3. Use `X+2` for the type of the `get-memory-raw-bytes()` function
//! 4. Use `X+3` for the type of the `set-memory-raw-bytes()` function
//!
//! ## Discovery of the index of the next available type index in the component part
//! 1. The component section can contain an internal subsections with separate types.
//!    Since the types from the inner subsections use a different index space, we must
//!    skip them. Every such section is detected and removed from the further processing
//! 2. The same process is repeated for the pre-component section, however this time all
//!    the internal sections referring to instances are removed from further processing,
//!    as these can also contain types from a different index space
//! 3. Count the number of type markers `type (;` in the component section, assign to `X`
//! 4. Count the number of type markers `type (;` in the pre-component section, assign to
//!    `Y`
//! 5. Use `X+Y+1` for the type of the `get-memory-size()` function re-export
//! 6. Use `X+Y+2` for the type of the `get-memory-raw-bytes()` function re-export
//! 7. Use `X+Y+3` for the type of the `set-memory-raw-bytes()` function re-export
//!
//! ## Discovery of the index of the alias of the core function in the component space
//! 1. Count the number of existing function in the core module by using the following
//!    regex
//! ```text
//! \(alias\s+core\s+export\s+0\s+"[^"]+"\s+\(core\s+func
//! ```
//! and assign to `X`
//! 2. Use `X+1` as the next available alias index
//!
//! ## Discovery of the index of the next available component level export
//! 1. The component section can contain an internal subsections with separate exports.
//!    Since the exports from the inner subsections use a different index space, we must
//!    skip them. Every such section is detected and removed from the further processing
//! 2. Count all export markers using the regex
//! ```text
//! \(export \(.*\(func
//! ```
//! and assign to `X`
//!
//! 3. Count all functions already present in the component sections using the regex
//! ```text
//! \(func\s+\(;?\d+;?\)\s+\(type\s+\d+\) \(canon
//! ```
//! and assign to `Y`
//!
//! 4. Use `X+Y+1` as the next component level export index for `get-memory-size()`
//! 5. Use `X+Y+3` as the next component level export index for `get-memory-raw-bytes()`
//!    (please note that the index is incremented by 2 because both exports and functions
//!    on the component level share the same index space)
//! 6. Use `X+Y+5` as the next component level export index for `set-memory-raw-bytes()`
//!
//! ## The actual injection of the functions, types and aliases
//! Once all the necessary indices are calculated the injection is prepared.
//!
//! ### Core module
//! This section shows the full code that is injected into the core module. The
//! `{PLACEHOLDERS}` are replaced with correctly calculated values.
//!
//! #### Types
//! ```text
//!    (type (func (result i32)))
//!    (type (func (param i32) (result i64)))
//!    (type (func (param i32 i64)))
//! ```
//! ### Exports
//! ```text
//!    (export "{MAGIC}get-memory-size" (func ${MAGIC}get-memory-size))
//!    (export "{MAGIC}get-memory-raw-bytes" (func ${MAGIC}get-memory-raw-bytes))
//!    (export "{MAGIC}set-memory-raw-bytes" (func ${MAGIC}set-memory-raw-bytes))
//! ```
//! ### Functions
//! ```text
//!    (func ${MAGIC}get-memory-size (type {TYPE_ID}) (result i32)
//!        memory.size
//!    )
//!    (func ${MAGIC}get-memory-raw-bytes (type {TYPE_ID}) (param i32) (result i64)
//!        local.get 0
//!        i64.load
//!    )
//!    (func ${MAGIC}set-memory-raw-bytes (type {TYPE_ID}) (param i32 i64)
//!      local.get 0
//!      local.get 1
//!      i64.store
//!    )
//! ```
//!
//! ### Component section
//!
//! #### Types
//! ```text
//!    (type (func (result u32)))
//!    (type (func (param "val" u32) (result s64)))
//!    (type (func (param "val" u32) (param "val2" s64)))
//! ```
//! #### Aliases to core functions
//! ```text
//!    (alias core export 0 "{MAGIC}get-memory-size" (core func))
//!    (alias core export 0 "{MAGIC}get-memory-raw-bytes" (core func))
//!    (alias core export 0 "{MAGIC}set-memory-raw-bytes" (core func))
//! ```
//! #### Canonical ABI lifted function
//! "To lift" means to wrap w low-level core function into high-level component function.
//! Therefore we do not need to add any new code on the component level, but instead we
//! use the ABI lift to add the necessary marshalling logic interface.
//! ```text
//!    (func (type {TYPE_ID}) (canon lift (core func {FUNC_ID})))
//!    (func (type {TYPE_ID}) (canon lift (core func {FUNC_ID})))
//!    (func (type {TYPE_ID}) (canon lift (core func {FUNC_ID})))
//! ```
//! #### Exports
//! ```text
//!    (export "{MAGIC}get-memory-size" (func {FUNC_ID}))
//!    (export "{MAGIC}get-memory-raw-bytes" (func {FUNC_ID}))
//!    (export "{MAGIC}set-memory-raw-bytes" (func {FUNC_ID}))
//! ```
//! All parts with the added injections are stitched together and returned as a new .wat
//! file. The parts are merged in the following order:
//! 1. Pre-component section
//! 2. All core modules
//! 3. Component section
//!
//! # Notes on the parser
//! The core functionality of the patcher is a parenthesis-based parser used to navigate
//! different parts of the .wat file. Given the starting point in the .wat file it is able
//! to find the end of given section by analyzing the pairs of open-close parenthesis. It
//! is context aware, ie. it'll properly recognize and ignore parenthesis in strings.
//! However, there are still some edge cases that can potentially break the current parser
//! implementation. If such cases are encountered it's advised to switch to using the
//! `wasmparser` tool.

// TODO[RC]: Patcher is not yet wired into the Hermes.
#![allow(unused)]

use std::path::Path;

use regex::Regex;

/// Magic string to avoid name collisions with existing functions.
// cspell:disable-next-line
const MAGIC: &str = r"vmucqq2137emxpatzkmuyy1szcpx23lp-hermes-";

/// Regex to detect the function definitions in the core module.
const CORE_FUNC_REGEX: &str = r"\(func\s+\$[^\s()]+[^)]*\(;";

/// A string that marks the beginning of a core module.
const CORE_MODULE_MARKER: &str = "(core module (;";

/// Regex to detect the aliases of core functions in the component part.
// TODO[RC]: The core number here (0) should not be hardcoded, but aligned with the
// component structure.
const COMPONENT_CORE_FUNC_REGEX: &str = r#"\(alias\s+core\s+export\s+0\s+"[^"]+"\s+\(core\s+func"#;

/// Regex to detect the function export definitions in the component part.
const COMPONENT_EXPORT_FUNC_REGEX: &str = r"\(export \(.*\(func";

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
// TODO[RC]: The core number here (0) should not be hardcoded, but aligned with the
// component structure.
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
    core_modules: Vec<String>,
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
        Ok(Self { wat })
    }

    /// Creates a new patcher from a WAT string.
    pub fn from_str<S: AsRef<str>>(wat: S) -> Result<Self, anyhow::Error> {
        let _syntax_check = wat::parse_str(wat.as_ref())?;
        Ok(Self {
            wat: wat.as_ref().to_string(),
        })
    }

    /// Patches the WAT by injecting functions to get memory size and read raw memory
    /// bytes.
    #[allow(clippy::arithmetic_side_effects)]
    pub fn patch(&self) -> Result<String, anyhow::Error> {
        let WasmInternals {
            mut component_part,
            mut pre_core_component_part,
            core_modules,
        } = self.split_into_parts()?;

        let mut core_modules_iter = core_modules.into_iter();
        let module_0 = core_modules_iter
            .next()
            .ok_or_else(|| anyhow::anyhow!("should have at least one module"))?;

        let module_0_last_parenthesis = module_0
            .rfind(')')
            .ok_or_else(|| anyhow::anyhow!("no closing parenthesis in core part"))?;
        let mut module_0 = module_0
            .get(..module_0_last_parenthesis)
            .ok_or_else(|| anyhow::anyhow!("malformed module 0 part"))?
            .to_string();

        let next_core_type_index = Self::get_next_core_type_index(&module_0)?;

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

        let next_component_func_index =
            Self::get_next_component_export_func_index(&component_part)?;
        let component_func_1_index = next_component_func_index.to_string();
        // For each injected function we also add an 'export' which shares the same index space,
        // hence we need to bump the index by 2.
        let component_func_2_index = (next_component_func_index + 2).to_string();
        let component_func_3_index = (next_component_func_index + 4).to_string();

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

        module_0.push_str(&core_type_injection);
        module_0.push_str(&core_func_injection);
        module_0.push_str(&core_export_injection);
        component_part.push_str(&component_injections);

        #[allow(clippy::format_collect)]
        let other_modules = core_modules_iter
            .map(|m| format!("    {m}\n"))
            .collect::<String>();

        let patched_wat = format!(
            "
            (component 
                {pre_core_component_part}

                {module_0}
            )

                {other_modules}
            
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
    #[allow(clippy::arithmetic_side_effects)]
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

        let mut processed_pre_component = pre_core_component.as_ref().to_string();
        while let Some(inner_instance_start) = processed_pre_component.find("(instance") {
            let inner_instance_end =
                Self::parse_until_section_end(inner_instance_start, &processed_pre_component)? + 1;
            processed_pre_component.replace_range(inner_instance_start..inner_instance_end, "---");
        }

        let component_type_count = Self::get_item_count("type (;", processed_component)?;
        let pre_component_type_count = Self::get_item_count("type (;", processed_pre_component)?;

        Ok(component_type_count + pre_component_type_count)
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
    #[allow(clippy::expect_used, clippy::arithmetic_side_effects)] // regex is hardcoded and should be valid
    fn get_next_component_export_func_index<S: AsRef<str>>(
        component: S
    ) -> Result<u32, anyhow::Error> {
        let mut processed_component = component.as_ref().to_string();
        while let Some(inner_component_start) = processed_component.find("(component") {
            let inner_component_end =
                Self::parse_until_section_end(inner_component_start, &processed_component)? + 1;
            processed_component.replace_range(inner_component_start..inner_component_end, "---");
        }

        let export_count = Self::get_item_count(
            Regex::new(COMPONENT_EXPORT_FUNC_REGEX).expect("this should be a proper regex"),
            &processed_component,
        )?;
        let func_count = Self::get_item_count(
            Regex::new(COMPONENT_FUNC_REGEX).expect("this should be a proper regex"),
            &processed_component,
        )?;
        Ok(export_count + func_count)
    }

    /// Gets the next available core function index.
    #[allow(clippy::expect_used)] // regex is hardcoded and should be valid
    fn get_next_core_func_index<S: AsRef<str>>(core_module: S) -> Result<u32, anyhow::Error> {
        Self::get_item_count(
            Regex::new(CORE_FUNC_REGEX).expect("this should be a proper regex"),
            core_module.as_ref(),
        )
    }

    #[allow(clippy::arithmetic_side_effects)]
    /// Looks for the end of the WAT section that starts at `start`.
    fn parse_until_section_end<S: AsRef<str>>(
        start: usize,
        wat: S,
    ) -> Result<usize, anyhow::Error> {
        let mut end = start;
        let mut count = 1;
        let mut in_string = false;
        for ch in wat
            .as_ref()
            .get((start + 1)..)
            .ok_or_else(|| anyhow::anyhow!("malformed wat"))?
            .chars()
        {
            end += 1;
            // TODO[RC]: We need to be more cautious, since WAT supports both \" escapes in strings
            // and ; arbitrary comments. See: https://webassembly.github.io/spec/core/text/values.html#strings
            // Ultimately, this needs to be fixed by using a proper parser.
            if ch == '"' {
                in_string = !in_string;
            }
            if !in_string {
                if ch == '(' {
                    count += 1;
                } else if ch == ')' {
                    count -= 1;
                    if count == 0 {
                        break;
                    }
                }
            }
        }
        Ok(end)
    }

    /// Extracts the core module and component part from the WAT.
    #[allow(clippy::arithmetic_side_effects)]
    fn split_into_parts(&self) -> Result<WasmInternals, anyhow::Error> {
        const COMPONENT_ITEM: &str = "(component";

        let mut processed_component = self.wat.clone();
        let last_core_start = processed_component
            .rfind(CORE_MODULE_MARKER)
            .ok_or_else(|| anyhow::anyhow!("no core module"))?;
        let last_core_end =
            Self::parse_until_section_end(last_core_start, &processed_component)? + 1;

        let mut core_modules = Vec::new();
        let mut processed_component = self.wat.clone();
        while let Some(core_module_start) = processed_component.find(CORE_MODULE_MARKER) {
            let core_module_end =
                Self::parse_until_section_end(core_module_start, &processed_component)? + 1;
            core_modules.push(
                processed_component
                    .get(core_module_start..core_module_end)
                    .ok_or_else(|| anyhow::anyhow!("should have core module"))?
                    .to_string(),
            );

            processed_component.replace_range(core_module_start..core_module_end, "---");
        }

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
            .get((last_core_end + 1)..)
            .ok_or_else(|| anyhow::anyhow!("malformed wat"))?;
        let component_last_parenthesis = component_part
            .rfind(')')
            .ok_or_else(|| anyhow::anyhow!("no closing parenthesis in component part"))?;

        Ok(WasmInternals {
            core_modules,
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
        AsContextMut, Engine, Store,
        component::{Instance, Linker, bindgen},
    };
    use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiView, p2::add_to_linker_sync};

    use crate::wasm::patcher::{MAGIC, Patcher, WasmInternals};

    const LINEAR_MEMORY_PAGE_SIZE_BYTES: u32 = 65536;

    const COMPONENT_SINGLE_CORE_MODULE: &str =
        "tests/test_wasm_files/component_single_core_module.wasm";
    const COMPONENT_MULTIPLE_CORE_MODULES: &str =
        "tests/test_wasm_files/component_multiple_core_modules.wasm";
    const HERMES_REAL_LIFE_MODULE: &str = "tests/test_wasm_files/hermes_real_life_module.wasm";

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
    fn extracts_wasm_internals_no_pre_core() {
        const EXPECTED_CORE: &str = r"
            (core module (;0;)
                (type (;0;) (func))
                (type (;1;) (func (result i32)))
                (type (;2;) (func (param i32 i32) (result i32)))
                (func $two (;1;) (type 1) (result i32)
                    i32.const 2
                )
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
            core_modules,
            component_part,
            pre_core_component_part,
        } = patcher.split_into_parts().expect("should extract parts");

        let module_0 = core_modules.first().expect("should have first module");

        assert_eq!(
            strip_whitespaces(module_0),
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
    fn types_from_pre_core_are_included_when_patching() {
        let patcher = Patcher::from_str(MAKESHIFT_CORRECT_WAT_WITH_PRE_CORE_COMPONENT)
            .expect("should create patcher");
        let WasmInternals {
            core_modules,
            component_part,
            pre_core_component_part,
        } = patcher.split_into_parts().expect("should extract parts");

        let next_index =
            Patcher::get_next_component_type_index(&component_part, &pre_core_component_part)
                .expect("should get next index");

        // There is 1 type in the component part and another one in the pre_component part that is
        // included before the actual core module
        assert_eq!(next_index, 2);
    }

    #[test]
    fn extracts_wasm_internals_with_pre_core() {
        const EXPECTED_CORE: &str = r"
            (core module (;0;)
                (type (;0;) (func))
                (type (;1;) (func (result i32)))
                (type (;2;) (func (param i32 i32) (result i32)))
                (func $two (;1;) (type 1) (result i32)
                    i32.const 2
                )
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
            core_modules,
            component_part,
            pre_core_component_part,
        } = patcher.split_into_parts().expect("should extract parts");

        let module_0 = core_modules.first().expect("should have first module");

        assert_eq!(
            strip_whitespaces(module_0),
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

        let patcher = Patcher::from_file(HERMES_REAL_LIFE_MODULE).expect("should create patcher");
        let result = patcher.patch().expect("should patch");
        let encoded = wat::parse_str(&result);
        assert!(encoded.is_ok());
    }

    #[test]
    fn injected_get_memory_size_works() {
        let files = [COMPONENT_SINGLE_CORE_MODULE, HERMES_REAL_LIFE_MODULE];

        for file in files {
            // Step 1: Patch the WASM file
            let patcher = Patcher::from_file(file).expect("should create patcher");
            let result = patcher.patch().expect("should patch");
            let encoded = wat::parse_str(&result).expect("should encode");

            // Step 2: Instantiate the patched WASM
            let engine = Engine::default();
            let component = wasmtime::component::Component::new(&engine, encoded)
                .expect("should create component");
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
            let source_wat = wasmprinter::print_file(file).expect("should read");
            let expected_memory_entry = format!("(memory (;0;) {memory_size_in_pages})");

            assert!(source_wat.contains(&expected_memory_entry));
        }
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
        assert!(
            linear_memory
                .windows(1024)
                .any(|window| window == expected_pattern)
        );
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
        let files = [COMPONENT_SINGLE_CORE_MODULE, HERMES_REAL_LIFE_MODULE];

        for file in files {
            // Step 1: Patch the WASM file
            let patcher = Patcher::from_file(file).expect("should create patcher");
            let result = patcher.patch().expect("should patch");
            let encoded = wat::parse_str(&result).expect("should encode");

            // Step 2: Instantiate the patched WASM
            let engine = Engine::default();
            let component = wasmtime::component::Component::new(&engine, encoded)
                .expect("should create component");
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
    }

    #[test]
    #[allow(clippy::too_many_lines)]
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
    fn patching_real_life_hermes_module_works() {
        let patcher = Patcher::from_file(HERMES_REAL_LIFE_MODULE).expect("should create patcher");

        let WasmInternals {
            mut component_part,
            mut pre_core_component_part,
            ..
        } = patcher.split_into_parts().expect("should split into parts");

        let patched_wat = patcher.patch().expect("should patch");

        let encoded = wat::parse_str(&patched_wat);
        assert!(encoded.is_ok());
    }
}
