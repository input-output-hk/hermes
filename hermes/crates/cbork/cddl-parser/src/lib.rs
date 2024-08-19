//! A parser for CDDL, utilized for parsing in accordance with RFC 8610.

#![allow(missing_docs)] // TODO(apskhem): Temporary, to bo removed in a subsequent PR

use std::fmt::Debug;

use derive_more::{Display, From};
pub use pest::Parser;
use pest::{error::Error, iterators::Pairs};

pub mod rfc_8610 {
    pub use pest::Parser;

    #[derive(pest_derive::Parser)]
    #[grammar = "grammar/rfc_8610.pest"]
    pub struct RFC8610Parser;
}

pub mod rfc_9165 {
    pub use pest::Parser;

    #[derive(pest_derive::Parser)]
    #[grammar = "grammar/rfc_8610.pest"]
    #[grammar = "grammar/rfc_9165.pest"]
    pub struct RFC8610Parser;
}

pub mod cddl {
    pub use pest::Parser;

    #[derive(pest_derive::Parser)]
    #[grammar = "grammar/rfc_8610.pest"]
    #[grammar = "grammar/rfc_9165.pest"]
    #[grammar = "grammar/cddl_modules.pest"]
    pub struct RFC8610Parser;
}

pub mod cddl_test {
    pub use pest::Parser;

    // Parser with DEBUG rules. These rules are only used in tests.
    #[derive(pest_derive::Parser)]
    #[grammar = "grammar/rfc_8610.pest"]
    #[grammar = "grammar/rfc_9165.pest"]
    #[grammar = "grammar/cddl_modules.pest"]
    #[grammar = "grammar/cddl_test.pest"] // Ideally this would only be used in tests.
    pub struct CDDLTestParser;
}

/// Represents different parser extensions for handling CDDL specifications.
pub enum Extension {
    /// RFC8610 ONLY limited parser.
    RFC8610Parser,
    /// RFC8610 and RFC9165 limited parser.
    RFC9165Parser,
    /// RFC8610, RFC9165, and CDDL modules.
    CDDLParser,
}

// CDDL Standard Postlude - read from an external file
pub const POSTLUDE: &str = include_str!("grammar/postlude.cddl");

/// Abstract Syntax Tree (AST) representing parsed CDDL syntax.
// TODO: this is temporary. need to add more pragmatic nodes
#[derive(Debug)]
pub enum AST<'a> {
    /// Represents the AST for RFC 8610 CDDL rules.
    RFC8610(Pairs<'a, rfc_8610::Rule>),
    /// Represents the AST for RFC 9165 CDDL rules.
    RFC9165(Pairs<'a, rfc_9165::Rule>),
    /// Represents the AST for CDDL Modules rules.
    CDDL(Pairs<'a, cddl::Rule>),
}

/// Represents different types of errors related to different types of extension.
#[derive(Display, Debug)]
enum CDDLErrorType {
    /// An error related to RFC 8610 extension.
    RFC8610(Error<rfc_8610::Rule>),
    /// An error related to RFC 9165 extension.
    RFC9165(Error<rfc_9165::Rule>),
    /// An error related to CDDL modules extension.
    Cddl(Error<cddl::Rule>),
}

/// Represents an error that may occur during CDDL parsing.
#[derive(thiserror::Error, Debug, From)]
#[error("{0}")]
pub struct CDDLError(CDDLErrorType);

/// Parses and checks semantically a CDDL input string.
///
/// # Arguments
///
/// * `input` - A string containing the CDDL input to be parsed.
///
/// # Returns
///
/// Returns `Ok(())` if parsing is successful, otherwise returns an `Err` containing
/// a boxed `CDDLError` indicating the parsing error.
///
/// # Errors
///
/// This function may return an error in the following cases:
///
/// - If there is an issue with parsing the CDDL input.
///
/// # Examples
///
/// ```rs
/// use cddl_parser::{parse_cddl, Extension};
/// use std:fs;
///
/// let mut input = fs::read_to_string("path/to/your/file.cddl").unwrap();
/// let result = parse_cddl(&mut input, &Extension::CDDLParser);
/// assert!(result.is_ok());
/// ```
pub fn parse_cddl<'a>(
    input: &'a mut String, extension: &Extension,
) -> Result<AST<'a>, Box<CDDLError>> {
    input.push_str("\n\n");
    input.push_str(POSTLUDE);

    let result = match extension {
        Extension::RFC8610Parser => {
            rfc_8610::RFC8610Parser::parse(rfc_8610::Rule::cddl, input)
                .map(AST::RFC8610)
                .map_err(CDDLErrorType::RFC8610)
        },
        Extension::RFC9165Parser => {
            rfc_9165::RFC8610Parser::parse(rfc_9165::Rule::cddl, input)
                .map(AST::RFC9165)
                .map_err(CDDLErrorType::RFC9165)
        },
        Extension::CDDLParser => {
            cddl::RFC8610Parser::parse(cddl::Rule::cddl, input)
                .map(AST::CDDL)
                .map_err(CDDLErrorType::Cddl)
        },
    };

    result.map_err(CDDLError).map_err(Box::new)
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn it_works() {
        let mut input = String::new();
        let result = parse_cddl(&mut input, &Extension::CDDLParser);

        match result {
            Ok(c) => println!("{c:?}"),
            Err(e) => {
                println!("{e:?}");
                println!("{e}");
            },
        }
    }
}
