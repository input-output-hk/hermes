pub(crate) const S_PASSES: &[&str] = &[" ", "  ", " \t \t", " \t  \r \n \r\n   "];
pub(crate) const S_FAILS: &[&str] = &[" a ", "zz", " \t d \t", " \t  \r \n \t \r\n  x"];
pub(crate) const TEXT_PASSES: &[&str] = &[r#""""#, r#""abc""#, "\"abc\\n\""];
pub(crate) const TEXT_FAILS: &[&str] = &["", "''", "\"abc\n\""];
