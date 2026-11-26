//! AbstractLexerTest.php at doctrine/lexer
//! 71d305eaa6480d3d3460c7d053d1da7749fa9cca. Names and assertions are
//! PHPUnit's. `assertEquals()` on tokens is `===` here, which the values
//! the lexers produce also meet in PHP (testPeek's `assertSame()`s).
//!
//! Upstream: doctrine/lexer tests/AbstractLexerTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

mod support;

use php_value::numeric::{is_numeric, numeric_value};
use php_value::{Number, PhpString, Value};
use php2rust_doctrine_lexer::{AbstractLexer, LexerState, Token};
use support::{ConcreteLexer, EnumLexer, MutableLexer, TokenType};

type ConcreteToken = Token<&'static str>;

fn token(value: impl Into<Value>, type_: &'static str, position: usize) -> ConcreteToken {
    Token::new(value, Some(type_), position)
}

/// `dataProvider()`.
fn data_provider() -> Vec<(&'static str, Vec<ConcreteToken>)> {
    vec![(
        "price=10",
        vec![
            token("price", "string", 0),
            token("=", "operator", 5),
            token(10, "int", 6),
        ],
    )]
}

#[test]
fn testResetPeek() {
    let expected_tokens = [
        token("price", "string", 0),
        token("=", "operator", 5),
        token(10, "int", 6),
    ];

    let mut concrete_lexer = ConcreteLexer::new();
    concrete_lexer.set_input("price=10").unwrap();

    assert_eq!(Some(expected_tokens[0].clone()), concrete_lexer.peek());
    assert_eq!(Some(expected_tokens[1].clone()), concrete_lexer.peek());
    concrete_lexer.reset_peek();
    assert_eq!(Some(expected_tokens[0].clone()), concrete_lexer.peek());
}

#[test]
fn testResetPosition() {
    let expected_tokens = [
        token("price", "string", 0),
        token("=", "operator", 5),
        token(10, "int", 6),
    ];

    let mut concrete_lexer = ConcreteLexer::new();
    concrete_lexer.set_input("price=10").unwrap();
    assert!(concrete_lexer.lookahead().is_none());

    assert!(concrete_lexer.move_next());
    assert_eq!(Some(&expected_tokens[0]), concrete_lexer.lookahead());

    assert!(concrete_lexer.move_next());
    assert_eq!(Some(&expected_tokens[1]), concrete_lexer.lookahead());

    concrete_lexer.reset_position(0);

    assert!(concrete_lexer.move_next());
    assert_eq!(Some(&expected_tokens[0]), concrete_lexer.lookahead());
}

/// @see <https://github.com/doctrine/lexer/issues/53>
///
/// `resetPosition()` takes a character offset, not a token index. The input
/// below has multi-character tokens, so the second token's offset (6)
/// differs from its index (1).
#[test]
fn testResetPositionByTokenPosition() {
    let mut concrete_lexer = ConcreteLexer::new();
    concrete_lexer.set_input("price=10").unwrap();

    assert!(concrete_lexer.move_next());
    assert!(concrete_lexer.move_next());

    let target = concrete_lexer.lookahead().unwrap().clone();
    assert_eq!(Value::from("="), target.value);
    assert_eq!(5, target.position);

    // Move the cursor past the index that used to coincide with the target.
    let mut remaining_tokens = 0;
    while concrete_lexer.move_next() {
        remaining_tokens += 1;
    }

    assert_eq!(1, remaining_tokens);
    assert!(concrete_lexer.lookahead().is_none());

    concrete_lexer.reset_position(i64::try_from(target.position).unwrap());

    assert!(concrete_lexer.move_next());
    assert_eq!(Some(&target), concrete_lexer.lookahead());
}

#[test]
fn testResetPositionPastTheLastToken() {
    let mut concrete_lexer = ConcreteLexer::new();
    concrete_lexer.set_input("price=10").unwrap();

    let mut token_count = 0;
    while concrete_lexer.move_next() {
        token_count += 1;
    }

    assert_eq!(3, token_count);

    // No token starts at or after this offset, so the lexer lands at the end.
    concrete_lexer.reset_position(99);

    assert!(!concrete_lexer.move_next());
    assert!(concrete_lexer.lookahead().is_none());
}

#[test]
fn testMoveNext() {
    for (input, expected_tokens) in data_provider() {
        let mut concrete_lexer = ConcreteLexer::new();
        concrete_lexer.set_input(input).unwrap();
        assert!(concrete_lexer.lookahead().is_none());

        for expected_token in &expected_tokens {
            assert!(concrete_lexer.move_next());
            assert_eq!(Some(expected_token), concrete_lexer.lookahead());
        }

        assert!(!concrete_lexer.move_next());
        assert!(concrete_lexer.lookahead().is_none());
    }
}

#[test]
fn testSkipUntil() {
    let mut concrete_lexer = ConcreteLexer::new();
    concrete_lexer.set_input("price=10").unwrap();

    assert!(concrete_lexer.move_next());
    concrete_lexer.skip_until(&"operator");

    assert_eq!(Some(&token("=", "operator", 5)), concrete_lexer.lookahead(),);
}

#[test]
fn testUtf8Mismatch() {
    let mut concrete_lexer = ConcreteLexer::new();
    concrete_lexer.set_input(b"\xE9=10".as_slice()).unwrap();

    assert!(concrete_lexer.move_next());

    assert_eq!(
        Some(&token(PhpString::from(b"\xE9=10".as_slice()), "string", 0)),
        concrete_lexer.lookahead(),
    );
}

#[test]
fn testPeek() {
    for (input, expected_tokens) in data_provider() {
        let mut concrete_lexer = ConcreteLexer::new();
        concrete_lexer.set_input(input).unwrap();
        for expected_token in &expected_tokens {
            let actual_token = concrete_lexer.peek().unwrap();
            assert_eq!(expected_token, &actual_token);
            assert!(expected_token.value.identical(&actual_token.value));
            assert_eq!(expected_token.type_, actual_token.type_);
            assert_eq!(expected_token.position, actual_token.position);
        }

        assert!(concrete_lexer.peek().is_none());
    }
}

#[test]
fn testGlimpse() {
    for (input, expected_tokens) in data_provider() {
        let mut concrete_lexer = ConcreteLexer::new();
        concrete_lexer.set_input(input).unwrap();

        for expected_token in &expected_tokens {
            let actual_token = concrete_lexer.glimpse().unwrap();
            assert_eq!(expected_token, &actual_token);
            assert_eq!(Some(expected_token.clone()), concrete_lexer.glimpse());
            assert!(expected_token.value.identical(&actual_token.value));
            assert_eq!(expected_token.type_, actual_token.type_);
            assert_eq!(expected_token.position, actual_token.position);
            concrete_lexer.move_next();
        }

        assert!(concrete_lexer.peek().is_none());
    }
}

/// `inputUntilPositionDataProvider()`.
fn input_until_position_data_provider() -> Vec<(&'static str, i64, &'static str)> {
    vec![("price=10", 5, "price")]
}

#[test]
fn testGetInputUntilPosition() {
    for (input, position, expected_input) in input_until_position_data_provider() {
        let mut concrete_lexer = ConcreteLexer::new();
        concrete_lexer.set_input(input).unwrap();

        assert_eq!(
            PhpString::from(expected_input),
            concrete_lexer.get_input_until_position(position)
        );
    }
}

#[test]
fn testIsNextToken() {
    for (input, expected_tokens) in data_provider() {
        let mut concrete_lexer = ConcreteLexer::new();
        concrete_lexer.set_input(input).unwrap();

        concrete_lexer.move_next();
        for expected_token in &expected_tokens {
            let type_ = expected_token.type_.unwrap();
            assert!(concrete_lexer.is_next_token(&type_));
            concrete_lexer.move_next();
        }
    }
}

#[test]
fn testIsNextTokenAny() {
    for (input, expected_tokens) in data_provider() {
        let all_token_types: Vec<&'static str> = expected_tokens
            .iter()
            .map(|token| token.type_.unwrap())
            .collect();

        let mut concrete_lexer = ConcreteLexer::new();
        concrete_lexer.set_input(input).unwrap();

        concrete_lexer.move_next();
        for expected_token in &expected_tokens {
            let type_ = expected_token.type_.unwrap();
            assert!(concrete_lexer.is_next_token_any(&[type_]));
            assert!(concrete_lexer.is_next_token_any(&all_token_types));
            concrete_lexer.move_next();
        }
    }
}

#[test]
fn testGetLiteral() {
    let concrete_lexer = ConcreteLexer::new();
    assert_eq!(
        Value::from("Doctrine\\Tests\\Common\\Lexer\\ConcreteLexer::INT"),
        concrete_lexer.get_literal(&"int")
    );
    assert_eq!(
        Value::from("fake_token"),
        concrete_lexer.get_literal(&"fake_token")
    );
}

#[test]
fn testGetLiteralWithEnumLexer() {
    let enum_lexer = EnumLexer::new();
    assert_eq!(
        Value::from("Doctrine\\Tests\\Common\\Lexer\\TokenType::OPERATOR"),
        enum_lexer.get_literal(&TokenType::Operator),
    );
}

#[test]
fn testIsA() {
    let concrete_lexer = ConcreteLexer::new();
    assert!(concrete_lexer.is_a(b"11", &"int"));
    assert!(concrete_lexer.is_a(b"1.1", &"int"));
    assert!(concrete_lexer.is_a(b"=", &"operator"));
    assert!(concrete_lexer.is_a(b">", &"operator"));
    assert!(concrete_lexer.is_a(b"<", &"operator"));
    assert!(concrete_lexer.is_a(b"fake_text", &"string"));
}

#[test]
fn testAddCatchablePatternsToMutableLexer() {
    let mut mutable_lexer = MutableLexer::new();
    mutable_lexer.add_catchable_pattern("[a-z]");
    mutable_lexer.set_input("one").unwrap();
    let token = mutable_lexer.glimpse();

    let token = token.unwrap();
    assert_eq!(PhpString::from("o"), token.value);

    let mut mutable_lexer = MutableLexer::new();
    mutable_lexer.add_catchable_pattern("[a-z]+");
    mutable_lexer.set_input("one").unwrap();
    let token = mutable_lexer.glimpse();

    let token = token.unwrap();
    assert_eq!(PhpString::from("one"), token.value);
}

/// Upstream first calls `setlocale(LC_ALL, 'tr_TR.utf8', 'tr_TR')`: this
/// port has no locale, its patterns behave the same in every one.
#[test]
fn testMarkerAnnotationLocaleTr() {
    let mut mutable_lexer = MutableLexer::new();
    mutable_lexer.add_catchable_pattern(r"[a-z_\\][a-z0-9_\:\\]*[a-z_][a-z0-9_]*");
    mutable_lexer.add_catchable_pattern(r"(?:[+-]?[0-9]+(?:[\.][0-9]+)*)(?:[eE][+-]?[0-9]+)?");
    mutable_lexer.add_catchable_pattern(r#""(?:""|[^"])*+""#);
    mutable_lexer.set_input("@ODM\\Id").unwrap();

    assert!(mutable_lexer.token().is_none());
    assert!(mutable_lexer.lookahead().is_none());
    assert!(mutable_lexer.move_next());
    assert!(mutable_lexer.token().is_none());
    assert!(mutable_lexer.lookahead().is_some());
    assert_eq!(
        PhpString::from("@"),
        mutable_lexer.lookahead().unwrap().value
    );
    assert!(mutable_lexer.move_next());
    assert!(mutable_lexer.token().is_some());
    assert_eq!(PhpString::from("@"), mutable_lexer.token().unwrap().value);
    assert_eq!(
        PhpString::from("ODM\\Id"),
        mutable_lexer.lookahead().unwrap().value
    );
}

/// The anonymous lexer of `testCanTokenizeFloatValue()`.
#[derive(Default)]
struct FloatLexer {
    state: LexerState<i64>,
}

impl FloatLexer {
    const T_NONE: i64 = 1;
    const T_INTEGER: i64 = 2;
    const T_FLOAT: i64 = 4;
    const T_BOOL: i64 = 8;
}

impl AbstractLexer for FloatLexer {
    type Type = i64;
    type Value = Value;

    const CLASS: &'static str = "AbstractLexer@anonymous";

    fn lexer(&self) -> &LexerState<i64> {
        &self.state
    }

    fn lexer_mut(&mut self) -> &mut LexerState<i64> {
        &mut self.state
    }

    fn get_type(&self, value: &mut Value) -> Option<i64> {
        if value.identical(&Value::from("y")) {
            *value = Value::Bool(true);

            return Some(Self::T_BOOL);
        }

        let text = value
            .as_string()
            .map(PhpString::as_bytes)
            .unwrap_or_default();
        if is_numeric(text) {
            *value = match numeric_value(text) {
                Some(Number::Int(i)) => Value::Int(i),
                Some(Number::Float(f)) => Value::Float(f),
                None => Value::Int(0),
            };

            if matches!(value, Value::Int(_)) {
                return Some(Self::T_INTEGER);
            }

            return Some(Self::T_FLOAT);
        }

        Some(Self::T_NONE)
    }

    fn get_catchable_patterns(&self) -> Vec<PhpString> {
        vec![
            r"(?:[0-9]+)(?:[\.][0-9]+)?(?:e[+-]?[0-9]+)?".into(),
            "y".into(),
        ]
    }

    fn get_non_catchable_patterns(&self) -> Vec<PhpString> {
        vec![r"\s+".into()]
    }

    fn constants(&self) -> Vec<(&'static str, i64)> {
        vec![
            ("T_NONE", Self::T_NONE),
            ("T_INTEGER", Self::T_INTEGER),
            ("T_FLOAT", Self::T_FLOAT),
            ("T_BOOL", Self::T_BOOL),
        ]
    }
}

#[test]
fn testCanTokenizeFloatValue() {
    let mut lexer = FloatLexer::default();

    lexer.set_input("123.456").unwrap();
    let token = lexer.peek().unwrap();
    assert!(
        token.value.identical(&Value::Float(123.456)),
        "expect a real float, not a numerical string"
    );

    lexer.set_input("y").unwrap();
    let token = lexer.peek().unwrap();
    assert!(
        token.value.identical(&Value::Bool(true)),
        "expect a real bool, not a numerical string"
    );
}
