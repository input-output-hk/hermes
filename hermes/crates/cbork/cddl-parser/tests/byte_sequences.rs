use cddl_parser::{self, CDDLParser, Parser, Rule};

#[test]
/// Test if the `HEX_PAIR` rule passes properly.
fn check_hexpair() {
    let hex_pairs = vec!["00", "ab", "de", "0f", "f0"];

    let not_hex_pairs = vec!["0", " 0", "0 ", "az", "0p"];

    for hp in hex_pairs {
        let parse = CDDLParser::parse(Rule::HEX_PAIR, hp);
        assert!(parse.is_ok());
    }

    for hp in not_hex_pairs {
        let parse = CDDLParser::parse(Rule::HEX_PAIR, hp);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `URL_BASE64` rule passes properly.
fn check_url_base64() {
    let tests = vec![
        "abcdefghijklmnopq   rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ~",
        "abcdefghijklmnopqrstuvwyz0123456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ",
    ];

    let fails = vec![
        "abcdefghijklmnopq #  rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ~ ",
        "abcdefghijklmnopq $  rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ~\t",
        "abcdefghijklmnopq %  rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ~\n",
        "abcdefghijklmnopq ^  rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ~\r",
        "abcdefghijklmnopq &  rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ~\r\n",
    ];

    for test in tests {
        let parse = CDDLParser::parse(Rule::URL_BASE64_TEST, test);
        assert!(parse.is_ok());
    }

    for test in fails {
        let parse = CDDLParser::parse(Rule::URL_BASE64_TEST, test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `bytes` rule passes properly.
fn check_bytes() {
    let test = vec![
        "h''",
        "b64''",
        "''",
        "h'00'",
        "h'00112233445566778899aabbccddeeff0123456789abcdef'",
        "h'0 1 2 3 4 5 6 7 8 9 a b c d e f'",
        "h' 0 1 2 3 4 5\r 6 7 \n 8 9 a\r\n\t b c d e f'",
        "b64'abcdefghijklmnopq   rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ~'",
        "b64'abcdefghijklmnopq   rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ'",
        "''",
        "'text\n that gets converted \\\' into a byte string...'",
    ];

    let fail = vec![
        "h64",
        "b64",
        "\"\"",
        "h ''",
        "b64 ''",
        "h'001'",
        "b64'abcdefghijklmnopq #  rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ~'",
        "b64'abcdefghijklmnopq   & rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ'",
        "'\u{7}'",
    ];

    for test in test {
        let parse = CDDLParser::parse(Rule::bytes_TEST, test);
        assert!(parse.is_ok());
    }

    for test in fail {
        let parse = CDDLParser::parse(Rule::bytes_TEST, test);
        assert!(parse.is_err());
    }
}
