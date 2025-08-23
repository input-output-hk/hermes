//! Hermes binary build info

use build_info as build_info_crate;

/// Formatted hermes binary build info
pub(crate) const BUILD_INFO: &str = build_info_crate::format!("
version: {},
git info: {{{}}}
compiler: {}
build time: {}
",
    $.crate_info.version,
    $.version_control,
    $.compiler,
    $.timestamp
);

#[cfg(all(test, debug_assertions))]
mod tests {
    use super::*;

    #[test]
    fn build_info_test() {
        println!("{BUILD_INFO}");
    }
}
