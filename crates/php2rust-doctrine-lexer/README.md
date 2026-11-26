# Doctrine Lexer

Private workspace crate: a port of `doctrine/lexer` 3.0.3 (`71d305ea`), the
base of the ORM's DQL lexer. MIT; Doctrine source and tests copyright the
Doctrine Project. Unofficial, not affiliated with or endorsed by Doctrine.

```rust
use php2rust_doctrine_lexer::{AbstractLexer, LexerState};
use php_value::{PhpString, Value};

#[derive(Default)]
struct Lexer {
    state: LexerState<&'static str>,
}

impl AbstractLexer for Lexer {
    type Type = &'static str;
    type Value = Value;
    const CLASS: &'static str = "App\\Lexer";

    fn lexer(&self) -> &LexerState<&'static str> {
        &self.state
    }
    fn lexer_mut(&mut self) -> &mut LexerState<&'static str> {
        &mut self.state
    }
    fn get_catchable_patterns(&self) -> Vec<PhpString> {
        vec!["[a-z]+".into(), r"\d+".into()]
    }
    fn get_non_catchable_patterns(&self) -> Vec<PhpString> {
        vec![r"\s+".into(), "(.)".into()]
    }
    fn get_type(&self, value: &mut Value) -> Option<&'static str> {
        let digits = value.as_string().is_some_and(|s| s.as_bytes()[0].is_ascii_digit());
        Some(if digits { "int" } else { "word" })
    }
}

let mut lexer = Lexer::default();
lexer.set_input("price = 10").unwrap();
assert!(lexer.move_next());
assert_eq!(Value::from("price"), lexer.lookahead().unwrap().value);
assert_eq!(6, lexer.peek().unwrap().position); // "="
```

| Contract | Source | Evidence |
|---|---|---|
| `Token`: value, type, byte position; `isA()` (strict `in_array()`) | `Token.php` | TokenTest (1) |
| `AbstractLexer` as a trait over a `LexerState`: `setInput()`, `reset()`, `resetPeek()`, `resetPosition()` (a byte offset: the first token at or after it), `getInputUntilPosition()` (`substr()`), `isNextToken()`, `isNextTokenAny()`, `moveNext()`, `skipUntil()`, `isA()`, `peek()`, `glimpse()`, `getLiteral()` (enum case, class constant, or the token) | `AbstractLexer.php` | AbstractLexerTest (18) |
| `scan()`: one PCRE `/(c1)\|(c2)\|n1\|n2/` + `getModifiers()` (`iu`), split as `preg_split()` with `NO_EMPTY \| DELIM_CAPTURE \| OFFSET_CAPTURE` (groups of non-catchable patterns are tokens too; the retry for a non-empty match after an empty one); the whole input as one token when PHP's `preg_split()` returns `false` (invalid UTF-8 under `/u`, an invalid pattern, the backtracking limit); `getType()` may change the value | `AbstractLexer.php` | AbstractLexerTest; 6,034 recorded splits: 431 inputs x 14 lexers (`tests/data/lexer.json`, oracle/recorders/lexer.php, `tests/vectors.rs`) |

Not reproduced, and so not claimed:

- PHP's warning for an invalid pattern (`preg_split(): Compilation failed: ...`).
- Patterns `php_regex` does not translate (recursion, conditionals, verbs,
  ...): `setInput()` returns `PcreError::Unsupported` instead of tokens. So
  does an empty match in patterns that contain a backreference, whose
  non-empty retry this port cannot build.
- Locales: patterns behave the same under every `setlocale()`.
- Reading `$lookahead`, `$token` or the input before the first `setInput()`
  or `reset()` (an `Error` in PHP) gives `None` and `""`.
- `getLiteral()` sees the class constants the lexer lists in
  `AbstractLexer::constants()`, not reflection.
