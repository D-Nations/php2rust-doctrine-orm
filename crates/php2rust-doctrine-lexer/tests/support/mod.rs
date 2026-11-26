//! The lexers of doctrine/lexer's tests.
//!
//! Upstream: doctrine/lexer tests/ConcreteLexer.php tests/EnumLexer.php tests/MutableLexer.php tests/TokenType.php

#![allow(dead_code, reason = "each test file uses some of the lexers")]

use php_value::compare::loose_equals;
use php_value::numeric::{is_numeric, string_to_int};
use php_value::{PhpString, Value};
use php2rust_doctrine_lexer::{AbstractLexer, LexerState, Literal, TokenType as TokenTypeTrait};

/// `in_array($value, ['=', '<', '>'])`.
fn is_operator(value: &Value) -> bool {
    ["=", "<", ">"]
        .iter()
        .any(|operator| loose_equals(value, &Value::from(*operator)))
}

fn bytes(value: &Value) -> &[u8] {
    value.as_string().map_or(&[], PhpString::as_bytes)
}

/// `Doctrine\Tests\Common\Lexer\ConcreteLexer`.
#[derive(Default)]
pub struct ConcreteLexer {
    state: LexerState<&'static str>,
}

impl ConcreteLexer {
    pub const INT: &'static str = "int";

    pub fn new() -> Self {
        Self::default()
    }
}

impl AbstractLexer for ConcreteLexer {
    type Type = &'static str;
    type Value = Value;

    const CLASS: &'static str = "Doctrine\\Tests\\Common\\Lexer\\ConcreteLexer";

    fn lexer(&self) -> &LexerState<&'static str> {
        &self.state
    }

    fn lexer_mut(&mut self) -> &mut LexerState<&'static str> {
        &mut self.state
    }

    fn get_catchable_patterns(&self) -> Vec<PhpString> {
        vec!["=|<|>".into(), "[a-z]+".into(), r"\d+".into()]
    }

    fn get_non_catchable_patterns(&self) -> Vec<PhpString> {
        vec![r"\s+".into(), "(.)".into()]
    }

    fn get_type(&self, value: &mut Value) -> Option<&'static str> {
        if is_numeric(bytes(value)) {
            *value = Value::Int(string_to_int(bytes(value)));

            return Some("int");
        }

        if is_operator(value) {
            return Some("operator");
        }

        Some("string")
    }

    fn constants(&self) -> Vec<(&'static str, &'static str)> {
        vec![("INT", Self::INT)]
    }
}

/// `Doctrine\Tests\Common\Lexer\TokenType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    Int,
    Operator,
    String,
}

impl TokenTypeTrait for TokenType {
    fn literal(&self) -> Literal {
        Literal::UnitEnum {
            class: "Doctrine\\Tests\\Common\\Lexer\\TokenType",
            name: match self {
                Self::Int => "INT",
                Self::Operator => "OPERATOR",
                Self::String => "STRING",
            },
        }
    }
}

/// `Doctrine\Tests\Common\Lexer\EnumLexer`.
#[derive(Default)]
pub struct EnumLexer {
    state: LexerState<TokenType>,
}

impl EnumLexer {
    pub fn new() -> Self {
        Self::default()
    }
}

impl AbstractLexer for EnumLexer {
    type Type = TokenType;
    type Value = Value;

    const CLASS: &'static str = "Doctrine\\Tests\\Common\\Lexer\\EnumLexer";

    fn lexer(&self) -> &LexerState<TokenType> {
        &self.state
    }

    fn lexer_mut(&mut self) -> &mut LexerState<TokenType> {
        &mut self.state
    }

    fn get_catchable_patterns(&self) -> Vec<PhpString> {
        vec!["=|<|>".into(), "[a-z]+".into(), r"\d+".into()]
    }

    fn get_non_catchable_patterns(&self) -> Vec<PhpString> {
        vec![r"\s+".into(), "(.)".into()]
    }

    fn get_type(&self, value: &mut Value) -> Option<TokenType> {
        if is_numeric(bytes(value)) {
            *value = Value::Int(string_to_int(bytes(value)));

            return Some(TokenType::Int);
        }

        if is_operator(value) {
            return Some(TokenType::Operator);
        }

        Some(TokenType::String)
    }
}

/// `Doctrine\Tests\Common\Lexer\MutableLexer`: its values stay strings.
#[derive(Default)]
pub struct MutableLexer {
    state: LexerState<i64, PhpString>,
    catchable_patterns: Vec<PhpString>,
}

impl MutableLexer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_catchable_pattern(&mut self, pattern: &str) {
        self.catchable_patterns.push(pattern.into());
    }
}

impl AbstractLexer for MutableLexer {
    type Type = i64;
    type Value = PhpString;

    const CLASS: &'static str = "Doctrine\\Tests\\Common\\Lexer\\MutableLexer";

    fn lexer(&self) -> &LexerState<i64, PhpString> {
        &self.state
    }

    fn lexer_mut(&mut self) -> &mut LexerState<i64, PhpString> {
        &mut self.state
    }

    fn get_catchable_patterns(&self) -> Vec<PhpString> {
        self.catchable_patterns.clone()
    }

    fn get_non_catchable_patterns(&self) -> Vec<PhpString> {
        vec![r"[\s,]+".into()]
    }

    fn get_type(&self, _value: &mut PhpString) -> Option<i64> {
        Some(1)
    }
}
