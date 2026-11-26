//! Base class for writing simple lexers, i.e. for creating small DSLs.
//!
//! PHP's abstract class is a trait here: an implementor holds a
//! [`LexerState`] (the private properties and the public `$lookahead` and
//! `$token`) and gives the abstract methods; the rest are provided.
//!
//! Upstream: doctrine/lexer src/AbstractLexer.php

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use php_regex::PcreError;
use php_value::{PhpString, Value};

use crate::preg_split::{SplitError, Splitter};
use crate::token::{Literal, Token, TokenType};

/// The properties of an `AbstractLexer`.
#[derive(Debug, Clone)]
pub struct LexerState<T, V = Value> {
    /// Lexer original input string.
    input: PhpString,
    /// Array of scanned tokens.
    tokens: Vec<Token<T, V>>,
    /// Current lexer position in input string.
    position: usize,
    /// Current peek of current lexer position.
    peek: usize,
    /// The next token in the input.
    pub lookahead: Option<Token<T, V>>,
    /// The last matched/seen token.
    pub token: Option<Token<T, V>>,
    /// Composed regex for input parsing, compiled on the first scan.
    regex: Option<Compiled>,
}

#[derive(Debug, Clone)]
enum Compiled {
    Regex(Arc<Splitter>),
    /// PHP warns and `preg_split()` returns `false`.
    Invalid,
    /// A pattern `php_regex` does not translate.
    Unsupported(PcreError),
}

impl<T, V> Default for LexerState<T, V> {
    fn default() -> Self {
        Self {
            input: PhpString::default(),
            tokens: Vec::new(),
            position: 0,
            peek: 0,
            lookahead: None,
            token: None,
            regex: None,
        }
    }
}

impl<T, V> LexerState<T, V> {
    pub fn new() -> Self {
        Self::default()
    }
}

/// `Doctrine\Common\Lexer\AbstractLexer`.
///
/// `scan()` (and so `setInput()`) returns `Err` only for a pattern that
/// PHP accepts and `php_regex` does not translate
/// ([`PcreError::Unsupported`]); a pattern PHP rejects makes the whole
/// input one token, as PHP's `false` from `preg_split()` does.
pub trait AbstractLexer {
    /// `T`, the token type.
    type Type: TokenType;
    /// `V`, the token value: the matched text, then what `getType()` makes
    /// of it.
    type Value: Clone + From<PhpString>;

    /// `static::class`, for `getLiteral()`.
    const CLASS: &'static str;

    /// The lexer's properties.
    fn lexer(&self) -> &LexerState<Self::Type, Self::Value>;

    /// The lexer's properties.
    fn lexer_mut(&mut self) -> &mut LexerState<Self::Type, Self::Value>;

    /// Lexical catchable patterns.
    fn get_catchable_patterns(&self) -> Vec<PhpString>;

    /// Lexical non-catchable patterns.
    fn get_non_catchable_patterns(&self) -> Vec<PhpString>;

    /// Retrieve token type. Also processes the token value if necessary.
    fn get_type(&self, value: &mut Self::Value) -> Option<Self::Type>;

    /// Regex modifiers
    fn get_modifiers(&self) -> PhpString {
        PhpString::from("iu")
    }

    /// The class constants of `Self::Type` (`ReflectionClass::getConstants()`
    /// in declaration order, inherited ones after the class's own), for
    /// `getLiteral()`; constants of other types can never be `===` a token.
    fn constants(&self) -> Vec<(&'static str, Self::Type)> {
        Vec::new()
    }

    /// The next token in the input (`$lookahead`).
    fn lookahead(&self) -> Option<&Token<Self::Type, Self::Value>> {
        self.lexer().lookahead.as_ref()
    }

    /// The last matched/seen token (`$token`).
    fn token(&self) -> Option<&Token<Self::Type, Self::Value>> {
        self.lexer().token.as_ref()
    }

    /// Sets the input data to be tokenized.
    ///
    /// The Lexer is immediately reset and the new input tokenized.
    /// Any unprocessed tokens from any previous input are lost.
    fn set_input(&mut self, input: impl Into<PhpString>) -> Result<(), PcreError>
    where
        Self: Sized,
    {
        let input = input.into();
        let state = self.lexer_mut();
        state.input = input.clone();
        state.tokens = Vec::new();

        self.reset();
        self.scan(input.as_bytes())
    }

    /// Resets the lexer.
    fn reset(&mut self) {
        let state = self.lexer_mut();
        state.lookahead = None;
        state.token = None;
        state.peek = 0;
        state.position = 0;
    }

    /// Resets the peek pointer to 0.
    fn reset_peek(&mut self) {
        self.lexer_mut().peek = 0;
    }

    /// Resets the lexer position on the input to the given position.
    ///
    /// `position` is a character offset into the input, as found in
    /// `Token::$position`: the lexer goes to the first token at or after
    /// it, or past the last token when there is none.
    fn reset_position(&mut self, position: i64) {
        let state = self.lexer_mut();
        state.position = state
            .tokens
            .iter()
            .position(|token| i64::try_from(token.position).unwrap_or(i64::MAX) >= position)
            .unwrap_or(state.tokens.len());
    }

    /// Retrieve the original lexer's input until a given position
    /// (`substr($input, 0, $position)`).
    fn get_input_until_position(&self, position: i64) -> PhpString {
        let input = self.lexer().input.as_bytes();
        let len = input.len();
        let end = if position < 0 {
            len.saturating_sub(usize::try_from(position.unsigned_abs()).unwrap_or(usize::MAX))
        } else {
            usize::try_from(position).map_or(len, |p| p.min(len))
        };
        PhpString::from(&input[..end])
    }

    /// Checks whether a given token matches the current lookahead.
    fn is_next_token(&self, type_: &Self::Type) -> bool {
        self.lexer()
            .lookahead
            .as_ref()
            .is_some_and(|t| t.is_a(std::slice::from_ref(type_)))
    }

    /// Checks whether any of the given tokens matches the current lookahead.
    fn is_next_token_any(&self, types: &[Self::Type]) -> bool {
        self.lexer()
            .lookahead
            .as_ref()
            .is_some_and(|t| t.is_a(types))
    }

    /// Moves to the next token in the input string.
    fn move_next(&mut self) -> bool {
        let state = self.lexer_mut();
        state.peek = 0;
        state.token = state.lookahead.take();
        state.lookahead = state.tokens.get(state.position).cloned();
        if state.lookahead.is_some() {
            state.position += 1;
        }

        state.lookahead.is_some()
    }

    /// Tells the lexer to skip input tokens until it sees a token with the given value.
    fn skip_until(&mut self, type_: &Self::Type) {
        while self
            .lexer()
            .lookahead
            .as_ref()
            .is_some_and(|t| !t.is_a(std::slice::from_ref(type_)))
        {
            self.move_next();
        }
    }

    /// Checks if given value is identical to the given token.
    fn is_a(&self, value: &[u8], token: &Self::Type) -> bool {
        let mut value = Self::Value::from(PhpString::from(value));
        self.get_type(&mut value).as_ref() == Some(token)
    }

    /// Moves the lookahead token forward: the next token, or `None` if
    /// there are no more tokens ahead.
    fn peek(&mut self) -> Option<Token<Self::Type, Self::Value>> {
        let state = self.lexer_mut();
        let token = state.tokens.get(state.position + state.peek).cloned();
        if token.is_some() {
            state.peek += 1;
        }
        token
    }

    /// Peeks at the next token, returns it and immediately resets the peek.
    fn glimpse(&mut self) -> Option<Token<Self::Type, Self::Value>> {
        let peek = self.peek();
        self.lexer_mut().peek = 0;

        peek
    }

    /// Scans the input string for tokens.
    fn scan(&mut self, input: &[u8]) -> Result<(), PcreError> {
        if self.lexer().regex.is_none() {
            let compiled = compile(
                &self.get_catchable_patterns(),
                &self.get_non_catchable_patterns(),
                self.get_modifiers().as_bytes(),
            );
            self.lexer_mut().regex = Some(compiled);
        }

        let matches = match &self.lexer().regex {
            Some(Compiled::Regex(splitter)) => match splitter.split(input) {
                Ok(matches) => matches,
                // Work around https://bugs.php.net/78122
                Err(SplitError::Failed) => vec![(0, input.len())],
                Err(SplitError::Unsupported(error)) => return Err(error),
            },
            Some(Compiled::Unsupported(error)) => return Err(error.clone()),
            Some(Compiled::Invalid) | None => vec![(0, input.len())],
        };

        for (start, end) in matches {
            // Must remain before 'value' assignment since it can change content
            let mut first_match = Self::Value::from(PhpString::from(&input[start..end]));
            let type_ = self.get_type(&mut first_match);

            self.lexer_mut().tokens.push(Token {
                value: first_match,
                type_,
                position: start,
            });
        }
        Ok(())
    }

    /// Gets the literal for a given token: `Enum::CASE` for an enum case,
    /// `Class::NAME` for a constant of the lexer's class with that value,
    /// else the token itself.
    fn get_literal(&self, token: &Self::Type) -> Value {
        let literal = token.literal();
        if let Literal::UnitEnum { class, name } = literal {
            return Value::from(format!("{class}::{name}"));
        }

        let class_name = Self::CLASS;

        for (name, value) in self.constants() {
            if &value == token {
                return Value::from(format!("{class_name}::{name}"));
            }
        }

        match literal {
            Literal::Int(i) => Value::Int(i),
            Literal::String(s) => Value::String(s),
            Literal::UnitEnum { .. } => Value::Null,
        }
    }
}

/// `sprintf('/(%s)|%s/%s', implode(')|(', $catchable), implode('|',
/// $nonCatchable), $modifiers)`, compiled.
fn compile(catchable: &[PhpString], non_catchable: &[PhpString], modifiers: &[u8]) -> Compiled {
    let mut body = b"(".to_vec();
    join(&mut body, catchable, b")|(");
    body.extend_from_slice(b")|");
    join(&mut body, non_catchable, b"|");
    match cached_splitter(&body, modifiers) {
        Ok(splitter) => Compiled::Regex(splitter),
        Err(PcreError::Compile(_)) => Compiled::Invalid,
        Err(error @ PcreError::Unsupported(_)) => Compiled::Unsupported(error),
    }
}

/// Compiled split patterns by body and modifiers.
type SplitterCache = HashMap<(Vec<u8>, Vec<u8>), Arc<Splitter>>;

/// PHP keeps compiled patterns in a per-process cache (4096 entries); so
/// does this port, so that a new lexer for the same patterns (the ORM makes
/// one per query) does not compile them again.
fn cached_splitter(body: &[u8], modifiers: &[u8]) -> Result<Arc<Splitter>, PcreError> {
    const LIMIT: usize = 4096;
    static CACHE: OnceLock<Mutex<SplitterCache>> = OnceLock::new();

    let cache = CACHE.get_or_init(Mutex::default);
    let key = (body.to_vec(), modifiers.to_vec());
    if let Some(splitter) = cache
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&key)
    {
        return Ok(splitter.clone());
    }
    let splitter = Arc::new(Splitter::new(body, modifiers)?);
    let mut cache = cache.lock().unwrap_or_else(PoisonError::into_inner);
    if cache.len() >= LIMIT {
        cache.clear();
    }
    cache.insert(key, splitter.clone());
    Ok(splitter)
}

fn join(out: &mut Vec<u8>, parts: &[PhpString], glue: &[u8]) {
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            out.extend_from_slice(glue);
        }
        out.extend_from_slice(part.as_bytes());
    }
}
