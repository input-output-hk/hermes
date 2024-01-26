pub(crate) const COMMENT_PASSES: &[&str] = &["; A Comment \n", "; And another\r", ";more\r\n"];

pub(crate) const COMMENT_FAILS: &[&str] = &["not a comment\n"];

pub(crate) const WHITESPACE_COMMENT_PASSES: &[&str] = &[
    " ",
    "  ",
    " \t \t",
    " \t  \r \n \r\n   ",
    "; A Comment\r",
    " \t ; A Comment    \n",
    "; One Comment\n; Two Comments\n",
    "; One Comment  \n; Two Comments\r; Another Comment\r\n",
    "\t; One Comment \n\t; Two Comments\r; Another Comment\r\n",
    "\t; A Comment \n    ; Another Comment \t \r\n    \t  ; A Final Comment   \r\n",
];

pub(crate) const WHITESPACE_COMMENT_FAILS: &[&str] = &["not a comment"];