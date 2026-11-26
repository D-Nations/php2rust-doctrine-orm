//! Port of doctrine/lexer 3.0.3: the base lexer that splits an input with
//! one PCRE built from a lexer's patterns, and its tokens.
//!
//! Doctrine source copyright (c) Doctrine Project; distributed under MIT.
//!
//! Upstream: none

#![forbid(unsafe_code)]

mod token;

pub use token::{Literal, Token, TokenType};
