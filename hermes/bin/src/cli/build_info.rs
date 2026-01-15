//! Hermes binary build info

#[allow(clippy::doc_markdown)]
/// Hermes binary build info
mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

/// Macro to unwrap an option or return a default value in the const context
macro_rules! const_unwrap_or {
    ($opt:expr, $default:expr) => {
        match $opt {
            Some(v) => v,
            None => $default,
        }
    };
}

/// Formatted hermes binary build info
pub(crate) const BUILD_INFO: &str = const_format::formatcp!(
    "
        version: {}
        git info: {}
        compiler: {}
        build time: {}
    ",
    built_info::PKG_VERSION,
    const_unwrap_or!(built_info::GIT_COMMIT_HASH_SHORT, "unknown"),
    built_info::RUSTC_VERSION,
    built_info::BUILT_TIME_UTC
);

#[cfg(all(test, debug_assertions))]
mod tests {
    use super::*;

    #[test]
    fn build_info_test() {
        println!("{BUILD_INFO}");
    }
}
