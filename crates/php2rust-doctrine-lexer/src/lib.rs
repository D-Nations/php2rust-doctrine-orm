//! Port of doctrine/lexer 3.0.3: the base lexer that splits an input with
//! one PCRE built from a lexer's patterns, and its tokens. See README.md
//! for the contract.
//!
//! Doctrine source copyright (c) Doctrine Project; distributed under MIT.
//!
//! Upstream: none

#![forbid(unsafe_code)]

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeExample;

mod abstract_lexer;
mod preg_split;
mod token;

pub use abstract_lexer::{AbstractLexer, LexerState};
pub use php_regex::PcreError;
pub use token::{Literal, Token, TokenType};
