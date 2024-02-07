// cspell: words apskhem ABNF abnf Naur

#![allow(missing_docs)] // TODO(apskhem): Temporary, to bo removed in a subsequent PR

//! A parser for ABNF, utilized for parsing in accordance with RFC 5234.

use std::fmt::Debug;

pub use pest::Parser;

extern crate derive_more;
use derive_more::{Display, From};
use pest::{error::Error, iterators::Pairs};

pub mod abnf {
    pub use pest::Parser;

    #[derive(pest_derive::Parser)]
    #[grammar = "grammar/rfc_5234.pest"]
    pub struct ABNFParser;
}

pub mod abnf_test {
    pub use pest::Parser;

    #[derive(pest_derive::Parser)]
    #[grammar = "grammar/rfc_5234.pest"]
    #[grammar = "grammar/abnf_test.pest"]
    pub struct ABNFTestParser;
}

#[derive(Debug)]
#[allow(dead_code)]
/// Abstract Syntax Tree (AST) representing parsed ABNF syntax.
pub struct AST<'a>(Pairs<'a, abnf::Rule>);

/// Represents an error that may occur during ABNF parsing.
#[derive(Display, Debug, From)]
/// Error type for ABNF parsing.
pub struct ABNFError(Error<abnf::Rule>);

/// Parses the input string containing ABNF (Augmented Backus-Naur Form) syntax and
/// returns the Abstract Syntax Tree (AST).
///
/// # Arguments
///
/// * `input` - A reference to a string slice containing the ABNF syntax to parse.
///
/// # Returns
///
/// Returns a `Result` where the successful variant contains the Abstract Syntax Tree
/// (AST) representing the parsed ABNF, and the error variant contains a boxed
/// `ABNFError`.
///
/// # Errors
///
/// This function may return an error in the following cases:
///
/// - If there is an issue with parsing the ABNF input.
///
/// # Examples
///
/// ```rs
/// use abnf_parser::parse_abnf;
/// use std:fs;
///
/// let input = fs::read_to_string("path/to/your/file.abnf").unwrap();
/// let result = parse_abnf(&input);
/// ```
pub fn parse_abnf(input: &str) -> Result<AST<'_>, Box<ABNFError>> {
    let result: Result<AST<'_>, _> = abnf::ABNFParser::parse(abnf::Rule::abnf, input)
        .map(AST)
        .map_err(ABNFError);

    result.map_err(Box::new)
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn it_works() {
        let input = String::new();
        let result = parse_abnf(&input);

        match result {
            Ok(c) => println!("{c:?}"),
            Err(e) => {
                println!("{e:?}");
                println!("{e}");
            },
        }
    }
}
