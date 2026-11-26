//! A token of the input: its value, its type and its byte offset.
//!
//! Upstream: doctrine/lexer src/Token.php

use php_value::{PhpString, Value};

/// What `AbstractLexer::getLiteral()` needs to know about a token type
/// (`T of UnitEnum|string|int`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Literal {
    /// An int token type.
    Int(i64),
    /// A string token type.
    String(PhpString),
    /// A `UnitEnum` case: its class (`$token::class`) and case name.
    UnitEnum {
        /// The enum's fully qualified class name.
        class: &'static str,
        /// The case's name (`$token->name`).
        name: &'static str,
    },
}

/// A token type: `UnitEnum|string|int`. `PartialEq` is `===`.
pub trait TokenType: Clone + PartialEq {
    /// The type as a PHP value or enum case.
    fn literal(&self) -> Literal;
}

impl TokenType for i64 {
    fn literal(&self) -> Literal {
        Literal::Int(*self)
    }
}

impl TokenType for &'static str {
    fn literal(&self) -> Literal {
        Literal::String(PhpString::from(*self))
    }
}

impl TokenType for String {
    fn literal(&self) -> Literal {
        Literal::String(PhpString::from(self.as_str()))
    }
}

impl TokenType for PhpString {
    fn literal(&self) -> Literal {
        Literal::String(self.clone())
    }
}

/// `Doctrine\Common\Lexer\Token`: `T` is the type, `V` the value
/// (`string|int|float|bool`; by default a PHP [`Value`]).
#[derive(Debug, Clone, PartialEq)]
pub struct Token<T, V = Value> {
    /// The string value of the token in the input string
    pub value: V,
    /// The type of the token (identifier, numeric, string, input parameter, none)
    pub type_: Option<T>,
    /// The position of the token in the input string
    pub position: usize,
}

impl<T: PartialEq, V> Token<T, V> {
    pub fn new(value: impl Into<V>, type_: Option<T>, position: usize) -> Self {
        Self {
            value: value.into(),
            type_,
            position,
        }
    }

    /// `isA(...$types)`: whether the type is one of `types` (`in_array()`
    /// in strict mode; a token without a type is none of them).
    pub fn is_a(&self, types: &[T]) -> bool {
        self.type_.as_ref().is_some_and(|t| types.contains(t))
    }
}
