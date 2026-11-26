//! Port of `ParserTest`: `AbstractSchemaName()` and `match()` of the DQL
//! parser on the test entity manager (`tests/common/dql.rs`). The data
//! providers are loops over their cases.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Query/ParserTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/dql.rs"]
mod dql;
#[path = "common/models.rs"]
mod models;

use php2rust_doctrine_lexer::AbstractLexer;
use php2rust_doctrine_orm::query::{Parser, TokenType};
use php2rust_doctrine_persistence::php_reflection::thrown_is_a;

use dql::{TestEntityManager, TestQuery, thrown};

const QUERY_EXCEPTION: &str = r"Doctrine\ORM\Query\QueryException";
const CMS_USER: &str = r"Doctrine\Tests\Models\CMS\CmsUser";

/// `createParser($dql)`: a parser whose lexer has moved to the first token.
fn with_parser<R>(dql: &str, f: impl FnOnce(&mut Parser<'_>) -> R) -> R {
    let entity_manager = TestEntityManager::new();
    let query = TestQuery::new(&entity_manager, dql);
    let mut parser = Parser::new(&query).unwrap();
    parser.get_lexer().move_next();
    f(&mut parser)
}

#[test]
fn testAbstractSchemaNameSupportsFQCN() {
    with_parser(CMS_USER, |parser| {
        assert_eq!(CMS_USER, parser.abstract_schema_name().unwrap());
    });
}

#[test]
fn testAbstractSchemaNameSupportsClassnamesWithLeadingBackslash() {
    let name = format!("\\{CMS_USER}");
    with_parser(&name, |parser| {
        assert_eq!(name, parser.abstract_schema_name().unwrap());
    });
}

#[test]
fn testAbstractSchemaNameSupportsIdentifier() {
    with_parser("stdClass", |parser| {
        assert_eq!("stdClass", parser.abstract_schema_name().unwrap());
    });
}

/// `validMatches()`.
const VALID_MATCHES: [(TokenType, &str); 5] = [
    (TokenType::T_WHERE, "where"), // keyword
    (TokenType::T_DOT, "."),       // token that cannot be an identifier
    (TokenType::T_IDENTIFIER, "someIdentifier"),
    (TokenType::T_IDENTIFIER, "from"), // also a terminal string (the "FROM" keyword) as in DDC-505
    (TokenType::T_IDENTIFIER, "comma"),
    // not even a terminal string, but the name of a constant in the Lexer (whitebox test)
];

/// `invalidMatches()`.
const INVALID_MATCHES: [(TokenType, &str); 6] = [
    (TokenType::T_DOT, "ALL"), // ALL is a terminal string (reserved keyword) and also possibly an identifier
    (TokenType::T_DOT, ","),   // "," is a token on its own, but cannot be used as identifier
    (TokenType::T_WHERE, "WITH"), // as in DDC-3697
    (TokenType::T_WHERE, "."),
    // The following are qualified or aliased names and must not be accepted where only an Identifier is expected
    (TokenType::T_IDENTIFIER, r"\Some\Class"),
    (TokenType::T_IDENTIFIER, r"Some\Class"),
];

#[test]
fn testMatch() {
    for (expected_token, input_string) in VALID_MATCHES {
        with_parser(input_string, |parser| {
            // throws exception if not matched
            if let Err(error) = parser.match_token(expected_token) {
                panic!("{input_string}: {:?}", thrown(&error));
            }
        });
    }
}

#[test]
fn testMatchFailure() {
    for (expected_token, input_string) in INVALID_MATCHES {
        with_parser(input_string, |parser| {
            match parser.match_token(expected_token) {
                Ok(()) => panic!("{input_string} was matched"),
                Err(error) => {
                    assert!(thrown_is_a(&error, QUERY_EXCEPTION), "{:?}", thrown(&error));
                }
            }
        });
    }
}

#[test]
fn testNullLookahead() {
    let entity_manager = TestEntityManager::new();
    let query = TestQuery::new(&entity_manager, "SELECT CURRENT_TIMESTAMP()");
    let mut parser = Parser::new(&query).unwrap();
    let error = parser.match_token(TokenType::T_SELECT).unwrap_err();
    assert!(thrown_is_a(&error, QUERY_EXCEPTION), "{:?}", thrown(&error));
}
