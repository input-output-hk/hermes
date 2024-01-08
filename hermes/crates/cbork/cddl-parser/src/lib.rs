// cspell: words apskhem

#![allow(missing_docs)] // TODO(apskhem): Temporary, to bo removed in a subsequent PR

//! A parser for CDDL, utilized for parsing in accordance with RFC 8610.

pub mod error;
pub mod parser;

pub use pest::Parser;

extern crate derive_more;

pub type Result = std::result::Result<(), Box<error::CDDLError>>;

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
/// use cddl_parser::parse_cddl;
/// use std:fs;
///
/// let input = fs::read_to_string("path/to/your/file.cddl").unwrap();
/// let result = parse_cddl(&input);
/// assert!(result.is_ok());
/// ```
pub fn parse_cddl(input: &str, extension: parser::Extension) -> Result {
    let result = match extension {
        parser::Extension::RFC8610Parser => parser::rfc_8610::parse(input),
        parser::Extension::RFC9615Parser => unimplemented!(),
        parser::Extension::CDDLParser => unimplemented!(),
        parser::Extension::CDDLTestParser => unimplemented!(),
    };

    if let Err(err) = &result {
        println!("{err:?}");
        println!("{err}");
    }

    result
}
