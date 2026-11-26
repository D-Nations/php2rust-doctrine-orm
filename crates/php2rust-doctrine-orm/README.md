# Doctrine ORM

Private workspace crate: a port of `doctrine/orm` 3.7.5 (`00c2fd70`), in
progress. MIT; Doctrine source and tests copyright the Doctrine Project.
Unofficial, not affiliated with or endorsed by Doctrine.

```rust
use php2rust_doctrine_lexer::AbstractLexer;
use php2rust_doctrine_orm::query::{Lexer, TokenType};
use php_value::PhpString;

let mut lexer = Lexer::new("SELECT u FROM App\\User u WHERE u.name = 'O''Neil'").unwrap();
let mut types = Vec::new();
while lexer.move_next() {
    types.push(lexer.lookahead().unwrap().type_.unwrap());
}
assert_eq!(TokenType::T_SELECT, types[0]);
assert_eq!(TokenType::T_FULLY_QUALIFIED_NAME, types[3]);
assert_eq!(TokenType::T_STRING, types[10]);
assert_eq!(PhpString::from("O'Neil"), lexer.token().unwrap().value);
```

| Contract | Source | Evidence |
|---|---|---|
| `Query\TokenType`: every case with its value and name, `cases()`, `tryFrom()`, the case of a name | `Query/TokenType.php` | Query LexerTest; recorded tokens |
| `Query\Lexer`: the DQL patterns on doctrine/lexer; `getType()`: numbers (`T_FLOAT` with `.` or `e`), quoted strings unquoted (`''` to `'`), keywords and the names `T_<NAME>` of cases above 100 in any ASCII case, fully qualified names, input parameters, symbols, `T_NONE`; values stay strings | `Query/Lexer.php` | Query LexerTest (15); 1,201 inputs recorded from PHP 8.4.27 (`tests/data/query_lexer.json`, oracle/recorders/orm_query_lexer.php, `tests/query_lexer_vectors.rs`) |

`Lexer::new()` returns doctrine/lexer's `PcreError`, which the DQL
patterns never cause.
