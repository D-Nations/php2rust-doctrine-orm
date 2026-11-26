//! LexerTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! Names and assertions are PHPUnit's. The lexer's values are strings, so
//! `assertEquals()` with a number compares numerically, as PHPUnit's
//! numeric comparators do (`assert_equals_number()`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Query/LexerTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

use php_value::numeric::numeric_value;
use php_value::{Number, PhpString};
use php2rust_doctrine_lexer::{AbstractLexer, Token};
use php2rust_doctrine_orm::query::{Lexer, TokenType};

/// PHPUnit's `assertEquals($number, $numericString)`: equal within
/// `PHP_FLOAT_EPSILON`.
fn equals_number(expected: f64, actual: &PhpString) -> bool {
    let actual = match numeric_value(actual.as_bytes()) {
        Some(Number::Int(i)) => Number::Int(i).as_f64(),
        Some(Number::Float(f)) => f,
        None => return false,
    };
    (actual - expected).abs() <= f64::EPSILON
}

fn assert_equals_number(expected: f64, actual: &PhpString) {
    assert!(equals_number(expected, actual), "{expected} != {actual:?}");
}

fn lookahead(lexer: &Lexer) -> Token<TokenType, PhpString> {
    lexer.lookahead().unwrap().clone()
}

#[test]
fn testScannerRecognizesTokens() {
    for (type_, value) in provide_tokens() {
        let mut lexer = Lexer::new(value).unwrap();

        lexer.move_next();
        let token = lookahead(&lexer);

        assert_eq!(Some(type_), token.type_);
        assert_eq!(PhpString::from(value), token.value);
    }
}

#[test]
fn testScannerRecognizesTerminalString() {
    /*
     * "all" looks like an identifier, but in fact it's a reserved word
     * (a terminal string). It's up to the parser to accept it as an identifier
     * (with its literal value) when appropriate.
     */

    let mut lexer = Lexer::new("all").unwrap();

    lexer.move_next();
    let token = lookahead(&lexer);

    assert_eq!(Some(TokenType::T_ALL), token.type_);
}

#[test]
fn testScannerRecognizesDecimalInteger() {
    let mut lexer = Lexer::new("1234").unwrap();
    lexer.move_next();
    let token = lookahead(&lexer);
    assert_eq!(Some(TokenType::T_INTEGER), token.type_);
    assert_equals_number(1234.0, &token.value);
}

#[test]
fn testScannerRecognizesFloat() {
    let mut lexer = Lexer::new("1.234").unwrap();
    lexer.move_next();
    let token = lookahead(&lexer);
    assert_eq!(Some(TokenType::T_FLOAT), token.type_);
    assert_equals_number(1.234, &token.value);
}

#[test]
fn testScannerRecognizesFloatWithExponent() {
    let mut lexer = Lexer::new("1.2e3").unwrap();
    lexer.move_next();
    let token = lookahead(&lexer);
    assert_eq!(Some(TokenType::T_FLOAT), token.type_);
    assert_equals_number(1.2e3, &token.value);
}

#[test]
fn testScannerRecognizesFloatWithExponent2() {
    let mut lexer = Lexer::new("0.2e3").unwrap();
    lexer.move_next();
    let token = lookahead(&lexer);
    assert_eq!(Some(TokenType::T_FLOAT), token.type_);
    assert_equals_number(0.2e3, &token.value);
}

#[test]
fn testScannerRecognizesFloatWithNegativeExponent() {
    let mut lexer = Lexer::new("7E-10").unwrap();
    lexer.move_next();
    let token = lookahead(&lexer);
    assert_eq!(Some(TokenType::T_FLOAT), token.type_);
    assert_equals_number(7E-10, &token.value);
}

#[test]
fn testScannerRecognizesFloatBig() {
    let mut lexer = Lexer::new("123456789.01").unwrap();
    lexer.move_next();
    let token = lookahead(&lexer);
    assert_eq!(Some(TokenType::T_FLOAT), token.type_);
    assert_equals_number(1.234_567_890_1e8, &token.value);
}

#[test]
fn testScannerRecognizesFloatContainingWhitespace() {
    let mut lexer = Lexer::new("-   1.234e2").unwrap();
    lexer.move_next();
    let token = lookahead(&lexer);
    assert_eq!(Some(TokenType::T_MINUS), token.type_);
    assert_eq!(PhpString::from("-"), token.value);

    lexer.move_next();
    let token = lookahead(&lexer);
    assert_eq!(Some(TokenType::T_FLOAT), token.type_);
    assert!(!equals_number(-1.234e2, &token.value));
    assert_equals_number(1.234e2, &token.value);
}

#[test]
fn testScannerRecognizesStringContainingWhitespace() {
    let mut lexer = Lexer::new("'This is a string.'").unwrap();
    lexer.move_next();
    let token = lookahead(&lexer);
    assert_eq!(Some(TokenType::T_STRING), token.type_);
    assert_eq!(PhpString::from("This is a string."), token.value);
}

#[test]
fn testScannerRecognizesStringContainingSingleQuotes() {
    let mut lexer = Lexer::new("'abc''defg'''").unwrap();
    lexer.move_next();
    let token = lookahead(&lexer);
    assert_eq!(Some(TokenType::T_STRING), token.type_);
    assert_eq!(PhpString::from("abc'defg'"), token.value);
}

#[test]
fn testScannerRecognizesInputParameter() {
    let mut lexer = Lexer::new("?1").unwrap();
    lexer.move_next();
    let token = lookahead(&lexer);
    assert_eq!(Some(TokenType::T_INPUT_PARAMETER), token.type_);
    assert_eq!(PhpString::from("?1"), token.value);
}

#[test]
fn testScannerRecognizesNamedInputParameter() {
    let mut lexer = Lexer::new(":name").unwrap();
    lexer.move_next();
    let token = lookahead(&lexer);
    assert_eq!(Some(TokenType::T_INPUT_PARAMETER), token.type_);
    assert_eq!(PhpString::from(":name"), token.value);
}

#[test]
fn testScannerRecognizesNamedInputParameterStartingWithUnderscore() {
    let mut lexer = Lexer::new(":_name").unwrap();
    lexer.move_next();
    let token = lookahead(&lexer);
    assert_eq!(Some(TokenType::T_INPUT_PARAMETER), token.type_);
    assert_eq!(PhpString::from(":_name"), token.value);
}

#[test]
fn testScannerTokenizesASimpleQueryCorrectly() {
    let dql = "SELECT u FROM My\\Namespace\\User u WHERE u.name = 'Jack O''Neil'";
    let mut lexer = Lexer::new(dql).unwrap();

    let tokens = [
        ("SELECT", TokenType::T_SELECT, 0),
        ("u", TokenType::T_IDENTIFIER, 7),
        ("FROM", TokenType::T_FROM, 9),
        ("My\\Namespace\\User", TokenType::T_FULLY_QUALIFIED_NAME, 14),
        ("u", TokenType::T_IDENTIFIER, 32),
        ("WHERE", TokenType::T_WHERE, 34),
        ("u", TokenType::T_IDENTIFIER, 40),
        (".", TokenType::T_DOT, 41),
        ("name", TokenType::T_IDENTIFIER, 42),
        ("=", TokenType::T_EQUALS, 47),
        ("Jack O'Neil", TokenType::T_STRING, 49),
    ];

    for (value, type_, position) in tokens {
        lexer.move_next();
        let actual = lookahead(&lexer);
        assert_eq!(PhpString::from(value), actual.value);
        assert_eq!(Some(type_), actual.type_);
        assert_eq!(position, actual.position);
    }

    assert!(!lexer.move_next());
}

/// `provideTokens()`.
fn provide_tokens() -> Vec<(TokenType, &'static str)> {
    vec![
        (TokenType::T_IDENTIFIER, "u"), // one char
        (TokenType::T_IDENTIFIER, "someIdentifier"),
        (TokenType::T_IDENTIFIER, "s0m31d3nt1f13r"), // including digits
        (TokenType::T_IDENTIFIER, "some_identifier"), // including underscore
        (TokenType::T_IDENTIFIER, "_some_identifier"), // starts with underscore
        (TokenType::T_IDENTIFIER, "comma"), // name of a token class with value < 100 (whitebox test)
        (TokenType::T_FULLY_QUALIFIED_NAME, "Some\\Class"), // DQL class reference
    ]
}
