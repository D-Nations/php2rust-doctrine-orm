//! Scans a DQL query for tokens.
//!
//! Upstream: doctrine/orm src/Query/Lexer.php

use php_value::PhpString;
use php_value::numeric::is_numeric;
use php2rust_doctrine_lexer::{AbstractLexer, LexerState, PcreError};

use super::token_type::TokenType;

/// `Doctrine\ORM\Query\Lexer`: tokens of type [`TokenType`] whose values
/// stay strings (quoted strings unquoted).
#[derive(Debug, Clone)]
pub struct Lexer {
    state: LexerState<TokenType, PhpString>,
}

impl Lexer {
    /// Creates a new query scanner object.
    ///
    /// `input` is a query string. The DQL patterns are all ones `php_regex`
    /// translates, so the error does not occur.
    pub fn new(input: impl Into<PhpString>) -> Result<Self, PcreError> {
        let mut lexer = Self {
            state: LexerState::new(),
        };
        lexer.set_input(input)?;
        Ok(lexer)
    }
}

impl AbstractLexer for Lexer {
    type Type = TokenType;
    type Value = PhpString;

    const CLASS: &'static str = "Doctrine\\ORM\\Query\\Lexer";

    fn lexer(&self) -> &LexerState<TokenType, PhpString> {
        &self.state
    }

    fn lexer_mut(&mut self) -> &mut LexerState<TokenType, PhpString> {
        &mut self.state
    }

    fn get_catchable_patterns(&self) -> Vec<PhpString> {
        vec![
            r"[a-z_][a-z0-9_]*\:[a-z_][a-z0-9_]*(?:\\[a-z_][a-z0-9_]*)*".into(), // aliased name
            r"[a-z_\\][a-z0-9_]*(?:\\[a-z_][a-z0-9_]*)*".into(), // identifier or qualified name
            r"(?:[0-9]+(?:[\.][0-9]+)*)(?:e[+-]?[0-9]+)?".into(), // numbers
            "'(?:[^']|'')*'".into(),                             // quoted strings
            r"\?[0-9]*|:[a-z_][a-z0-9_]*".into(),                // parameters
        ]
    }

    fn get_non_catchable_patterns(&self) -> Vec<PhpString> {
        vec![r"\s+".into(), "--.*".into(), "(.)".into()]
    }

    fn get_type(&self, value: &mut PhpString) -> Option<TokenType> {
        let mut type_ = TokenType::T_NONE;
        let bytes = value.as_bytes();
        // Tokens are never empty; `$value[0]` of "" would be an error.
        let first = bytes.first().copied().unwrap_or(0);

        // Recognize numeric values
        if is_numeric(bytes) {
            if bytes.contains(&b'.') || bytes.iter().any(|b| b.eq_ignore_ascii_case(&b'e')) {
                return Some(TokenType::T_FLOAT);
            }

            return Some(TokenType::T_INTEGER);
        }

        // Recognize quoted strings
        if first == b'\'' {
            // substr($value, 1, strlen($value) - 2)
            let inner = if bytes.len() >= 2 {
                &bytes[1..bytes.len() - 1]
            } else {
                &[][..]
            };
            *value = PhpString::from(replace_doubled_quotes(inner));

            return Some(TokenType::T_STRING);
        }

        // Recognize identifiers, aliased or qualified names
        if first.is_ascii_alphabetic() || first == b'_' || first == b'\\' {
            let mut name = b"T_".to_vec();
            name.extend(bytes.iter().map(u8::to_ascii_uppercase));

            if let Some(defined) = TokenType::from_name(&name) {
                type_ = defined;

                if type_.value() > 100 {
                    return Some(type_);
                }
            }

            if bytes.contains(&b'\\') {
                return Some(TokenType::T_FULLY_QUALIFIED_NAME);
            }

            return Some(TokenType::T_IDENTIFIER);
        }

        // Recognize input parameters
        if first == b'?' || first == b':' {
            return Some(TokenType::T_INPUT_PARAMETER);
        }

        // Recognize symbols
        let symbol = match bytes {
            b"." => Some(TokenType::T_DOT),
            b"," => Some(TokenType::T_COMMA),
            b"(" => Some(TokenType::T_OPEN_PARENTHESIS),
            b")" => Some(TokenType::T_CLOSE_PARENTHESIS),
            b"=" => Some(TokenType::T_EQUALS),
            b">" => Some(TokenType::T_GREATER_THAN),
            b"<" => Some(TokenType::T_LOWER_THAN),
            b"+" => Some(TokenType::T_PLUS),
            b"-" => Some(TokenType::T_MINUS),
            b"*" => Some(TokenType::T_MULTIPLY),
            b"/" => Some(TokenType::T_DIVIDE),
            b"!" => Some(TokenType::T_NEGATE),
            b"{" => Some(TokenType::T_OPEN_CURLY_BRACE),
            b"}" => Some(TokenType::T_CLOSE_CURLY_BRACE),
            // Default
            _ => None,
        };

        Some(symbol.unwrap_or(type_))
    }
}

/// `str_replace("''", "'", $s)`.
fn replace_doubled_quotes(s: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        out.push(s[i]);
        i += if s[i] == b'\'' && s.get(i + 1) == Some(&b'\'') {
            2
        } else {
            1
        };
    }
    out
}
