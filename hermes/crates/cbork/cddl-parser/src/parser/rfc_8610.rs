// cspell: words apskhem

#![allow(missing_docs)] // TODO(apskhem): Temporary, to bo removed in a subsequent PR

//! A parser for CDDL, utilized for parsing in accordance with RFC 8610.

use std::fmt::Debug;

pub use pest::Parser;
use pest_derive::Parser;
use crate::{Result, error::CDDLError};

extern crate derive_more;

// Parser with DEBUG rules.  These rules are only used in tests.
#[derive(Parser)]
#[grammar = "grammar/cddl.pest"]
#[grammar = "grammar/cddl_test.pest"] // Ideally this would only be used in tests.
pub struct CDDLParser;

pub fn parse(input: &str) -> Result {
  let result = CDDLParser::parse(Rule::cddl, input);

  match result {
      Ok(c) => println!("{c:?}"),
      Err(e) => {
          println!("{e:?}");
          println!("{e}");
          return Err(Box::new(CDDLError::ParsingRFC8610(e)));
      },
  }

  Ok(())
}