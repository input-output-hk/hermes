pub(crate) mod rfc_8610;

use std::fmt::Debug;

pub use pest::Parser;
use pest_derive::Parser;

extern crate derive_more;

// Parser with DEBUG rules.  These rules are only used in tests.
#[derive(Parser)]
#[grammar = "grammar/cddl.pest"]
#[grammar = "grammar/cddl_test.pest"] // Ideally this would only be used in tests.
pub struct CDDLParser;

/// Represents different parser extensions for handling CDDL specifications.
pub enum Extension {
  /// RFC8610 ONLY limited parser.
  RFC8610Parser,
  /// RFC8610 and RFC9615 limited parser.
  RFC9615Parser,
  /// RFC8610, RFC9615, and CDDL modules.
  CDDLParser,
  /// Same as CDDLParser but includes the `cddl_test.pest` file for integration test usage, mainly for development testing.
  CDDLTestParser
}

pub const POSTLUDE: &str = include_str!("../grammar/postlude.cddl");