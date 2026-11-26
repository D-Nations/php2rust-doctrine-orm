//! TokenTest.php at doctrine/lexer 71d305eaa6480d3d3460c7d053d1da7749fa9cca.
//!
//! Upstream: doctrine/lexer tests/TokenTest.php

#![allow(non_snake_case)]

use php_value::Value;
use php2rust_doctrine_lexer::Token;

#[test]
fn testIsA() {
    let token: Token<&'static str, Value> = Token::new("foo", Some("string"), 1);

    assert!(token.is_a(&["string"]));
    assert!(token.is_a(&["int", "string"]));
    assert!(!token.is_a(&["int"]));
}
