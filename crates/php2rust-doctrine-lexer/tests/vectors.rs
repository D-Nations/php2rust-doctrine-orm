//! The tokens PHP 8.4.27 makes of 431 inputs with fourteen lexers
//! (tests/data/lexer.json, oracle/recorders/lexer.php): `scan()`'s
//! `preg_split()` with the upstream tests' patterns, the ORM's DQL
//! patterns, without `/u`, under `/x`, with empty and lazy matches, groups
//! in the non-catchable patterns and a backreference, and invalid patterns
//! (one token, PHP's warning not reproduced); `ConcreteLexer::getType()`.
//!
//! Upstream: none

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

mod support;

use php_value::{PhpString, Value};
use php2rust_doctrine_lexer::{AbstractLexer, LexerState, Token};
use serde_json::Value as Json;
use support::ConcreteLexer;

/// The recorder's `RawLexer`: the given patterns, no type, values left alone.
struct RawLexer {
    state: LexerState<&'static str>,
    catchable: Vec<PhpString>,
    non_catchable: Vec<PhpString>,
    modifiers: PhpString,
}

impl AbstractLexer for RawLexer {
    type Type = &'static str;
    type Value = Value;

    const CLASS: &'static str = "RawLexer";

    fn lexer(&self) -> &LexerState<&'static str> {
        &self.state
    }

    fn lexer_mut(&mut self) -> &mut LexerState<&'static str> {
        &mut self.state
    }

    fn get_catchable_patterns(&self) -> Vec<PhpString> {
        self.catchable.clone()
    }

    fn get_non_catchable_patterns(&self) -> Vec<PhpString> {
        self.non_catchable.clone()
    }

    fn get_modifiers(&self) -> PhpString {
        self.modifiers.clone()
    }

    fn get_type(&self, _value: &mut Value) -> Option<&'static str> {
        None
    }
}

fn hex(json: &Json) -> Vec<u8> {
    let text = json.as_str().unwrap().as_bytes();
    text.chunks(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn patterns(json: &Json) -> Vec<PhpString> {
    json.as_array()
        .unwrap()
        .iter()
        .map(|p| PhpString::from(hex(p)))
        .collect()
}

fn expected(json: &Json) -> Vec<Token<&'static str>> {
    json.as_array()
        .unwrap()
        .iter()
        .map(|token| {
            let value = if let Some(s) = token[0].get("s") {
                Value::from(PhpString::from(hex(s)))
            } else {
                Value::Int(token[0]["i"].as_i64().unwrap())
            };
            let type_ = token[1].as_str().map(|t| match t {
                "int" => "int",
                "operator" => "operator",
                "string" => "string",
                other => panic!("unknown type {other}"),
            });
            Token {
                value,
                type_,
                position: usize::try_from(token[2].as_u64().unwrap()).unwrap(),
            }
        })
        .collect()
}

fn all_tokens(
    lexer: &mut impl AbstractLexer<Type = &'static str, Value = Value>,
) -> Vec<Token<&'static str>> {
    let mut tokens = Vec::new();
    while let Some(token) = lexer.peek() {
        tokens.push(token);
    }
    tokens
}

#[test]
fn lexers_split_as_php() {
    let data: Json = serde_json::from_str(include_str!("data/lexer.json")).unwrap();
    let mut checked = 0;
    for lexer_data in data["lexers"].as_array().unwrap() {
        let name = lexer_data["name"].as_str().unwrap();
        let mut concrete = ConcreteLexer::new();
        let mut raw = RawLexer {
            state: LexerState::new(),
            catchable: patterns(&lexer_data["catchable"]),
            non_catchable: patterns(&lexer_data["non_catchable"]),
            modifiers: PhpString::from(lexer_data["modifiers"].as_str().unwrap()),
        };
        for case in lexer_data["cases"].as_array().unwrap() {
            let input = hex(&case["input"]);
            let tokens = if name == "concrete" {
                concrete.set_input(input.as_slice()).unwrap();
                all_tokens(&mut concrete)
            } else {
                raw.set_input(input.as_slice()).unwrap();
                all_tokens(&mut raw)
            };
            assert_eq!(
                expected(&case["tokens"]),
                tokens,
                "lexer {name}, input {:?}",
                String::from_utf8_lossy(&input)
            );
            checked += 1;
        }
    }
    assert_eq!(14 * 431, checked);
}

/// After an empty match PCRE retries for a non-empty one at the same place;
/// with a backreference in the patterns this port cannot, and says so.
#[test]
fn empty_match_retry_with_backreference_is_refused() {
    let mut lexer = RawLexer {
        state: LexerState::new(),
        catchable: vec!["x*".into()],
        non_catchable: vec![r"(y)\1".into()],
        modifiers: "iu".into(),
    };
    assert!(matches!(
        lexer.set_input("a"),
        Err(php2rust_doctrine_lexer::PcreError::Unsupported(_))
    ));
}
