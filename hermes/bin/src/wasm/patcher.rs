use std::path::Path;

use regex::Regex;

const MAGIC: &str = r#"VmUcqq2137emxpaTzkMUYy1SzCPx23lp_hermes_"#;

#[derive(Debug)]
struct WasmInternals {
    core_module: String,
    component_part: String,
}

struct WatMatch {
    pos: usize,
    len: usize,
}

enum WatElementMatcher {
    Exact(&'static str),
    Regex(Regex),
}

impl From<&'static str> for WatElementMatcher {
    fn from(s: &'static str) -> Self {
        WatElementMatcher::Exact(s)
    }
}

impl WatElementMatcher {
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
            WatElementMatcher::Regex(re) => todo!(), //re.find(s.as_ref()).map(|m| m.start()),
        }
    }
}

pub(crate) struct Patcher {
    wat: String,
}

impl Patcher {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, anyhow::Error> {
        let wat = wasmprinter::print_file(path)?;
        Ok(Self { wat })
    }

    pub fn from_str<P: AsRef<str>>(wat: P) -> Result<Self, anyhow::Error> {
        let _syntax_check = wat::parse_str(wat.as_ref())?;
        Ok(Self {
            wat: wat.as_ref().to_string(),
        })
    }

    pub fn patch(&self) -> Result<(), anyhow::Error> {
        let WasmInternals {
            mut core_module,
            component_part,
        } = self.core_and_component()?;

        let next_type_index = Self::get_next_core_type_index(&core_module);

        Ok(())
    }

    fn get_core_item_count<I, S>(
        item: I,
        core_module: S,
    ) -> u32
    where
        I: Into<WatElementMatcher>,
        S: AsRef<str>,
    {
        let mut start = 0;
        let mut count = 0;

        let matcher: WatElementMatcher = item.into();
        loop {
            match matcher.first_match(&core_module.as_ref()[start..]) {
                Some(WatMatch { pos, len }) => {
                    count += 1;
                    start += pos + len;
                },
                None => break,
            };
        }

        count
    }

    fn get_next_core_type_index<S: AsRef<str>>(core_module: S) -> u32 {
        Self::get_core_item_count("type (;", core_module.as_ref())
    }

    fn core_and_component(&self) -> Result<WasmInternals, anyhow::Error> {
        let module_start = self
            .wat
            .find("(core module")
            .ok_or_else(|| anyhow::anyhow!("no core module"))?;
        let mut module_end = module_start;

        let mut count = 1;
        for ch in self.wat[(module_start + 1)..].chars() {
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

        let core_module = &self.wat[module_start..=module_end];
        let component_part = &self.wat[(module_end + 1)..];
        let last_parenthesis = component_part
            .rfind(')')
            .ok_or_else(|| anyhow::anyhow!("no closing parenthesis in component part"))?;

        Ok(WasmInternals {
            core_module: core_module.to_string(),
            component_part: component_part[..last_parenthesis].to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::wasm::patcher::{Patcher, WasmInternals};

    const TEST_MODULE: &str = "../../wasm/test_wasm_modules/patcher_test.wasm";

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
        assert!(Patcher::from_file(TEST_MODULE).is_ok());
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
        let index = Patcher::get_next_core_type_index(&CORE_1);
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
        let index = Patcher::get_next_core_type_index(&CORE_2);
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
        let index = Patcher::get_next_core_type_index(&CORE_3);
        assert_eq!(index, 7);
    }

    #[test]
    fn foo() {
        let patcher = Patcher::from_str(MAKESHIFT_CORRECT_WAT).expect("should create patcher");
        let result = patcher.patch().expect("should patch");
    }
}
