use std::path::Path;

struct Patcher {
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
}

#[cfg(test)]
mod tests {
    use crate::wasm::patcher::Patcher;

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
            )
    "#;

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
}
