//! The DQL token types.
//!
//! Upstream: doctrine/orm src/Query/TokenType.php

use php2rust_doctrine_lexer::{Literal, TokenType as LexerTokenType};

macro_rules! token_types {
    ($($(#[$doc:meta])* $name:ident = $value:literal,)*) => {
        /// `Doctrine\ORM\Query\TokenType`, an int-backed enum.
        #[allow(non_camel_case_types, reason = "upstream's case names")]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(i64)]
        pub enum TokenType {
            $($(#[$doc])* $name = $value,)*
        }

        impl TokenType {
            /// `TokenType::cases()`, in declaration order.
            pub const CASES: &'static [TokenType] = &[$(TokenType::$name,)*];

            /// The case's name (`$case->name`).
            pub fn name(self) -> &'static str {
                match self {
                    $(TokenType::$name => stringify!($name),)*
                }
            }
        }
    };
}

token_types! {
    // All tokens that are not valid identifiers must be < 100
    T_NONE = 1,
    T_INTEGER = 2,
    T_STRING = 3,
    T_INPUT_PARAMETER = 4,
    T_FLOAT = 5,
    T_CLOSE_PARENTHESIS = 6,
    T_OPEN_PARENTHESIS = 7,
    T_COMMA = 8,
    T_DIVIDE = 9,
    T_DOT = 10,
    T_EQUALS = 11,
    T_GREATER_THAN = 12,
    T_LOWER_THAN = 13,
    T_MINUS = 14,
    T_MULTIPLY = 15,
    T_NEGATE = 16,
    T_PLUS = 17,
    T_OPEN_CURLY_BRACE = 18,
    T_CLOSE_CURLY_BRACE = 19,

    // All tokens that are identifiers or keywords that could be considered as identifiers should be >= 100
    T_FULLY_QUALIFIED_NAME = 101,
    T_IDENTIFIER = 102,

    // All keyword tokens should be >= 200
    T_ALL = 200,
    T_AND = 201,
    T_ANY = 202,
    T_AS = 203,
    T_ASC = 204,
    T_AVG = 205,
    T_BETWEEN = 206,
    T_BOTH = 207,
    T_BY = 208,
    T_CASE = 209,
    T_COALESCE = 210,
    T_COUNT = 211,
    T_DELETE = 212,
    T_DESC = 213,
    T_DISTINCT = 214,
    T_ELSE = 215,
    T_EMPTY = 216,
    T_END = 217,
    T_ESCAPE = 218,
    T_EXISTS = 219,
    T_FALSE = 220,
    T_FROM = 221,
    T_GROUP = 222,
    T_HAVING = 223,
    T_HIDDEN = 224,
    T_IN = 225,
    T_INDEX = 226,
    T_INNER = 227,
    T_INSTANCE = 228,
    T_IS = 229,
    T_JOIN = 230,
    T_LEADING = 231,
    T_LEFT = 232,
    T_LIKE = 233,
    T_MAX = 234,
    T_MEMBER = 235,
    T_MIN = 236,
    T_NEW = 237,
    T_NOT = 238,
    T_NULL = 239,
    T_NULLIF = 240,
    T_OF = 241,
    T_OR = 242,
    T_ORDER = 243,
    T_OUTER = 244,
    T_PARTIAL = 245,
    T_SELECT = 246,
    T_SET = 247,
    T_SOME = 248,
    T_SUM = 249,
    T_THEN = 250,
    T_TRAILING = 251,
    T_TRUE = 252,
    T_UPDATE = 253,
    T_WHEN = 254,
    T_WHERE = 255,
    T_WITH = 256,
    T_NAMED = 257,
    T_ON = 258,
}

impl TokenType {
    /// The backing value (`$case->value`).
    pub fn value(self) -> i64 {
        self as i64
    }

    /// `TokenType::tryFrom($value)`.
    pub fn try_from_value(value: i64) -> Option<Self> {
        Self::CASES
            .iter()
            .copied()
            .find(|case| case.value() == value)
    }

    /// The case named `name`, as `defined()` and `constant()` find
    /// `TokenType::<name>`.
    pub fn from_name(name: &[u8]) -> Option<Self> {
        Self::CASES
            .iter()
            .copied()
            .find(|case| case.name().as_bytes() == name)
    }
}

impl LexerTokenType for TokenType {
    fn literal(&self) -> Literal {
        Literal::UnitEnum {
            class: "Doctrine\\ORM\\Query\\TokenType",
            name: self.name(),
        }
    }
}
