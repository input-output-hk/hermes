// cspell: words apskhem

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
pub struct AST<'a>(Pairs<'a, abnf::Rule>);

/// Represents an error that may occur during ABNF parsing.
#[derive(Display, Debug, From)]
pub struct ABNFError(Error<abnf::Rule>);

pub fn parse_abnf<'a>(input: &'a str) -> Result<AST<'a>, Box<ABNFError>> {
    let result: Result<AST<'_>, _> = abnf::ABNFParser::parse(abnf::Rule::cddl, input)
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
