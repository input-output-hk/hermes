#![allow(missing_docs)] // TODO(apskhem): Temporary, to bo removed in a subsequent PR

//! A parser for CDDL, utilized for parsing in accordance with RFC 8610.

use std::fmt::Debug;

pub use pest::Parser;
use pest_derive::Parser;

extern crate derive_more;
use derive_more::{Display, From};

// Parser with DEBUG rules.  These rules are only used in tests.
#[derive(Parser)]
#[grammar = "grammar/cddl.pest"]
#[grammar = "grammar/cddl_test.pest"] // Ideally this would only be used in tests.
pub struct CDDLParser;

/// Represents an error that may occur during CDDL parsing.
#[derive(Display, Debug, From)]
pub struct CDDLError(pest::error::Error<Rule>);

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
/// use cddl_parser::parse_cddl;
/// use std:fs;
///
/// let input = fs::read_to_string("path/to/your/file.cddl").unwrap();
/// let result = parse_cddl(&input);
/// assert!(result.is_ok());
/// ```
pub fn parse_cddl(input: &str) -> Result<(), Box<CDDLError>> {
    let result = CDDLParser::parse(Rule::cddl, input);

    match result {
        Ok(c) => println!("{c:?}"),
        Err(e) => {
            println!("{e:?}");
            println!("{e}");
            return Err(Box::new(CDDLError::from(e)));
        },
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{parse_cddl, POSTLUDE};

    #[test]
    fn it_works() {
        let result = parse_cddl(POSTLUDE);

        match result {
            Ok(c) => println!("{c:?}"),
            Err(e) => {
                println!("{e:?}");
                println!("{e}");
            },
        }
    }
}
