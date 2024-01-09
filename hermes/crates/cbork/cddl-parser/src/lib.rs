// cspell: words apskhem

#![allow(missing_docs)] // TODO(apskhem): Temporary, to bo removed in a subsequent PR

//! A parser for CDDL, utilized for parsing in accordance with RFC 8610.

use std::fmt::Debug;

pub use pest::Parser;

extern crate derive_more;
use derive_more::{Display, From};
use pest::error::Error;

pub mod rfc_8610 {
    pub use pest::Parser;

    #[derive(pest_derive::Parser)]
    // #[grammar = "grammar/rfc_8610.pest"]
    // TODO: we will implement to support those specifications later
    #[grammar = "grammar/cddl.pest"]
    pub struct RFC8610Parser;
}

pub mod rfc_9615 {
    pub use pest::Parser;

    #[derive(pest_derive::Parser)]
    // #[grammar = "grammar/rfc_8610.pest"]
    // #[grammar = "grammar/rfc_9615.pest"]
    // TODO: we will implement to support those specifications later
    #[grammar = "grammar/cddl.pest"]
    pub struct RFC8610Parser;
}

pub mod cddl {
    pub use pest::Parser;

    #[derive(pest_derive::Parser)]
    // #[grammar = "grammar/rfc_8610.pest"]
    // #[grammar = "grammar/rfc_9615.pest"]
    // TODO: we will implement to support those specifications later
    #[grammar = "grammar/cddl.pest"]
    pub struct RFC8610Parser;
}

pub mod cddl_test {
    pub use pest::Parser;

    // Parser with DEBUG rules. These rules are only used in tests.
    #[derive(pest_derive::Parser)]
    // #[grammar = "grammar/rfc_8610.pest"]
    // #[grammar = "grammar/rfc_9615.pest"]
    // TODO: we will implement to support those specifications later
    #[grammar = "grammar/cddl.pest"]
    #[grammar = "grammar/cddl_test.pest"] // Ideally this would only be used in tests.
    pub struct CDDLTestParser;
}

/// Represents different parser extensions for handling CDDL specifications.
pub enum Extension {
    /// RFC8610 ONLY limited parser.
    RFC8610Parser,
    /// RFC8610 and RFC9615 limited parser.
    RFC9615Parser,
    /// RFC8610, RFC9615, and CDDL modules.
    CDDLParser,
}

#[derive(Display, Debug)]
pub enum CDDLErrorType {
    RFC8610(Error<rfc_8610::Rule>),
    RFC9615(Error<rfc_9615::Rule>),
    CDDL(Error<cddl::Rule>),
    CDDLTest(Error<cddl_test::Rule>),
}

/// Represents an error that may occur during CDDL parsing.
#[derive(Display, Debug, From)]
pub struct CDDLError(CDDLErrorType);

// CDDL Standard Postlude - read from an external file
pub const POSTLUDE: &str = include_str!("grammar/postlude.cddl");

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
/// let input = fs::read_to_string("path/to/your/file.cddl").unwrap();
/// let result = parse_cddl(&input, &Extension::CDDLParser);
/// assert!(result.is_ok());
/// ```
pub fn parse_cddl(input: &str, extension: &Extension) -> Result<(), Box<CDDLError>> {
    let result = match extension {
        Extension::RFC8610Parser => {
            rfc_8610::RFC8610Parser::parse(rfc_8610::Rule::cddl, input)
                .map(|_| ())
                .map_err(|e| Box::new(CDDLError::from(CDDLErrorType::RFC8610(e))))
        },
        Extension::RFC9615Parser => {
            rfc_9615::RFC8610Parser::parse(rfc_9615::Rule::cddl, input)
                .map(|_| ())
                .map_err(|e| Box::new(CDDLError::from(CDDLErrorType::RFC9615(e))))
        },
        Extension::CDDLParser => {
            cddl::RFC8610Parser::parse(cddl::Rule::cddl, input)
                .map(|_| ())
                .map_err(|e| Box::new(CDDLError::from(CDDLErrorType::CDDL(e))))
        },
    };

    result.map_err(|e| {
        println!("{e:?}");
        println!("{e}");
        e
    })
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn it_works() {
        let result = cddl_test::CDDLTestParser::parse(cddl_test::Rule::cddl, POSTLUDE);

        match result {
            Ok(c) => println!("{c:?}"),
            Err(e) => {
                println!("{e:?}");
                println!("{e:?}");
            },
        }
    }
}
