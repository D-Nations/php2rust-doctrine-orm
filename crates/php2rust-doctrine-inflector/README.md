# Doctrine Inflector

Private workspace crate: a port of `doctrine/inflector` 2.1.0 (`6d6c9627`).
MIT; Doctrine source and tests copyright the Doctrine Project. Unofficial,
not affiliated with or endorsed by Doctrine.

```rust
use php2rust_doctrine_inflector::{InflectorFactory, Language};
use php_value::PhpString;

let inflector = InflectorFactory::create().unwrap().build().unwrap();
assert_eq!(PhpString::from("model_name"), inflector.tableize(b"ModelName").unwrap());
assert_eq!(PhpString::from("ModelName"), inflector.classify(b"model_name"));
assert_eq!(PhpString::from("people"), inflector.pluralize(b"person"));
assert_eq!(PhpString::from("Child"), inflector.singularize(b"Children"));

let french = InflectorFactory::create_for_language(Language::FRENCH).unwrap().build().unwrap();
assert_eq!(PhpString::from("chevaux"), french.pluralize(b"cheval"));
```

| Contract | Source | Evidence |
|---|---|---|
| Rules: `Word`, `Pattern` (`/p/i` unless it starts with `/`), `Patterns` (`/^(?:p1\|p2)$/i` of the raw patterns), `Substitution`, `Substitutions` (keyed by word, later wins in place; lower-case lookup; first letter upper-cased again; `getFlippedSubstitutions()`), `Transformation` (`preg_replace()`, `""` for PHP's `null`), `Transformations` (first matching pattern), `Ruleset` | `Rules/*.php` | PatternTest, PatternsTest, RulesetTest, SubstitutionTest, SubstitutionsTest, TransformationTest, TransformationsTest, WordTest (17) |
| `RulesetInflector` (uninflected, then irregular, then regular, ruleset by ruleset), `CachedWordInflector`, `NoopWordInflector` | `RulesetInflector.php`, `CachedWordInflector.php`, `NoopWordInflector.php`, `WordInflector.php` | RulesetInflectorTest (7), CachedWordInflectorTest, NoopWordInflectorTest |
| The eight languages' rules, irregular words and uninflected patterns | `Rules/<Language>/*.php` | the language FunctionalTests (18); 2,712 words singularized and pluralized in each language, recorded from PHP 8.4.27 (`tests/data/inflector.json`, oracle/recorders/inflector.php, `tests/vectors.rs`) |
| `Inflector`: `tableize()` (with PHP's `RuntimeException` for invalid UTF-8), `classify()`, `camelize()`, `capitalize()`, `seemsUtf8()`, `unaccent()` (UTF-8 table or ISO-8859-1), `urlize()`, `singularize()`, `pluralize()`; PHP's `mb_strtolower()`, `ucwords()`, `strtr()` and `str_replace()` on bytes | `Inflector.php` | InflectorTest (9), InflectorFunctionalTest (5); 1,358 recorded strings through each method |
| `InflectorFactory::create()`, `createForLanguage()` and its `InvalidArgumentException`, `Language`, `LanguageInflectorFactory`, `GenericLanguageInflectorFactory` (`withSingularRules()`, `withPluralRules()`, `build()`) | `InflectorFactory.php`, `Language.php`, `LanguageInflectorFactory.php`, `GenericLanguageInflectorFactory.php` | InflectorFactoryTest (3) |

Not reproduced, and so not claimed:

- Patterns `php_regex` does not translate: `Pattern::new()` and
  `Patterns::new()` return `PcreError::Unsupported` (the built-in rules
  all translate). PHP's warning for an invalid pattern.
- `build()` after a reset without new rules returns PHP's
  `ArgumentCountError` without PHP's file and line in the message.
- `mb_strtolower()` uses Rust's Unicode tables, which may be a newer
  Unicode version than PHP 8.4's.
- `Ruleset`'s getters give the parts as trait objects, so test doubles
  can stand in for PHPUnit's mocks.
