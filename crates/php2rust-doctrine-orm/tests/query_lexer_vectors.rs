//! The tokens doctrine/orm's DQL lexer makes of 1,201 inputs in PHP 8.4.27
//! (`tests/data/query_lexer.json`, `oracle/recorders/orm_query_lexer.php`):
//! doctrine/lexer's recorded inputs, every keyword and token name in
//! three cases, quoting, parameter and number edge cases, and 400 strings
//! drawn from DQL words and symbols.
//!
//! Upstream: none

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

use php_value::PhpString;
use php2rust_doctrine_lexer::{AbstractLexer, Token};
use php2rust_doctrine_orm::query::{Lexer, TokenType};
use serde_json::Value as Json;

fn hex(json: &Json) -> Vec<u8> {
    let text = json.as_str().unwrap().as_bytes();
    text.chunks(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn dql_tokens_as_php() {
    let data: Json = serde_json::from_str(include_str!("data/query_lexer.json")).unwrap();
    let cases = data["cases"].as_array().unwrap();
    for case in cases {
        let input = hex(&case["input"]);
        let expected: Vec<Token<TokenType, PhpString>> = case["tokens"]
            .as_array()
            .unwrap()
            .iter()
            .map(|token| Token {
                value: PhpString::from(hex(&token[0])),
                type_: Some(TokenType::from_name(token[1].as_str().unwrap().as_bytes()).unwrap()),
                position: usize::try_from(token[2].as_u64().unwrap()).unwrap(),
            })
            .collect();
        let mut lexer = Lexer::new(input.as_slice()).unwrap();
        let mut actual = Vec::new();
        while let Some(token) = lexer.peek() {
            actual.push(token);
        }
        assert_eq!(
            expected,
            actual,
            "input {:?}",
            String::from_utf8_lossy(&input)
        );
    }
    assert_eq!(1201, cases.len());
}
