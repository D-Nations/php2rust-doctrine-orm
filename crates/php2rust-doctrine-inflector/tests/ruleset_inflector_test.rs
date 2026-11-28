//! RulesetInflectorTest.php at doctrine/inflector 6d6c96277ea252fc1304627204c3d5e6e15faa3b.
//! Names and assertions are PHPUnit's. A mocked Ruleset is one made of doubles: what a test does not configure answers as PHPUnit's stubs do (`''`, `false`).
//!
//! Upstream: doctrine/inflector tests/RulesetInflectorTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

mod support;

use std::sync::Arc;

use php_value::PhpString;
use php2rust_doctrine_inflector::rules::{
    Pattern, Patterns, Ruleset, Substitution, Substitutions, Transformations, Word, WordMatcher,
};
use php2rust_doctrine_inflector::{RulesetInflector, WordInflector};
use support::{MockWordInflector, MockWordMatcher};

/// The parts of a mocked `Ruleset`.
#[derive(Default)]
struct MockRuleset {
    regular: Option<Arc<dyn WordInflector>>,
    uninflected: Option<Arc<dyn WordMatcher>>,
    irregular: Option<Arc<dyn WordInflector>>,
}

impl MockRuleset {
    fn build(self) -> Ruleset {
        Ruleset::from_parts(
            self.regular
                .unwrap_or_else(|| Arc::new(MockWordInflector::default())),
            self.uninflected
                .unwrap_or_else(|| Arc::new(MockWordMatcher::default())),
            self.irregular
                .unwrap_or_else(|| Arc::new(MockWordInflector::default())),
        )
    }
}

fn ruleset_inflector(first: MockRuleset, second: MockRuleset) -> RulesetInflector {
    RulesetInflector::new(first.build(), [second.build()])
}

#[test]
fn testInflectIrregularUsesFirstMatch() {
    let first_irregular = Arc::new(MockWordInflector::returning("in", "first"));
    let second_irregular = Arc::new(MockWordInflector::default());

    let inflector = ruleset_inflector(
        MockRuleset {
            irregular: Some(first_irregular.clone()),
            ..MockRuleset::default()
        },
        MockRuleset {
            irregular: Some(second_irregular.clone()),
            ..MockRuleset::default()
        },
    );

    assert_eq!(PhpString::from("first"), inflector.inflect(b"in"));
    assert_eq!(vec!["in"], first_irregular.calls());
    assert_eq!(second_irregular.calls(), [] as [std::string::String; 0]);
}

#[test]
fn testInflectIrregularContinuesIfFirstRulesetReturnsOriginalValue() {
    let first_ruleset = Ruleset::new(
        Transformations::new([]),
        Patterns::new([]).unwrap(),
        Substitutions::new([]),
    );

    let second_ruleset = Ruleset::new(
        Transformations::new([]),
        Patterns::new([]).unwrap(),
        Substitutions::new([Substitution::new(Word::new("in"), Word::new("second"))]),
    );

    let inflector = RulesetInflector::new(first_ruleset, [second_ruleset]);

    assert_eq!(PhpString::from("second"), inflector.inflect(b"in"));
}

#[test]
fn testInflectUninflectedSkipsOnFirstMatch() {
    let first_uninflected = Arc::new(MockWordMatcher::returning("in", true));
    let second_uninflected = Arc::new(MockWordMatcher::default());

    let inflector = ruleset_inflector(
        MockRuleset {
            uninflected: Some(first_uninflected.clone()),
            ..MockRuleset::default()
        },
        MockRuleset {
            uninflected: Some(second_uninflected.clone()),
            ..MockRuleset::default()
        },
    );

    assert_eq!(PhpString::from("in"), inflector.inflect(b"in"));
    assert_eq!(vec!["in"], first_uninflected.calls());
    assert_eq!(second_uninflected.calls(), [] as [std::string::String; 0]);
}

#[test]
fn testIrregularIsInflectedEvenIfLaterRulesetIgnores() {
    let first_irregular =
        Substitutions::new([Substitution::new(Word::new("travel"), Word::new("travels"))]);
    let second_uninflected = Patterns::new([Pattern::new("travel").unwrap()]).unwrap();

    let inflector = ruleset_inflector(
        MockRuleset {
            irregular: Some(Arc::new(first_irregular)),
            ..MockRuleset::default()
        },
        MockRuleset {
            uninflected: Some(Arc::new(second_uninflected)),
            ..MockRuleset::default()
        },
    );

    assert_eq!(PhpString::from("travels"), inflector.inflect(b"travel"));
}

/// The shared irregular and uninflected doubles of the regular tests.
fn unmatched() -> (Arc<MockWordInflector>, Arc<MockWordMatcher>) {
    (
        Arc::new(MockWordInflector::returning("in", "in")),
        Arc::new(MockWordMatcher::returning("in", false)),
    )
}

#[test]
fn testInflectRegularUsesFirstMatch() {
    let (irregular, uninflected) = unmatched();
    let first_regular = Arc::new(MockWordInflector::returning("in", "first"));
    let second_regular = Arc::new(MockWordInflector::default());

    let inflector = ruleset_inflector(
        MockRuleset {
            regular: Some(first_regular.clone()),
            uninflected: Some(uninflected.clone()),
            irregular: Some(irregular.clone()),
        },
        MockRuleset {
            regular: Some(second_regular.clone()),
            uninflected: Some(uninflected),
            irregular: Some(irregular),
        },
    );

    assert_eq!(PhpString::from("first"), inflector.inflect(b"in"));
    assert_eq!(vec!["in"], first_regular.calls());
    assert_eq!(second_regular.calls(), [] as [std::string::String; 0]);
}

#[test]
fn testInflectRegularContinuesIfFirstRulesetReturnsOriginalValue() {
    let (irregular, uninflected) = unmatched();
    let first_regular = Arc::new(MockWordInflector::returning("in", "in"));
    let second_regular = Arc::new(MockWordInflector::returning("in", "second"));

    let inflector = ruleset_inflector(
        MockRuleset {
            regular: Some(first_regular.clone()),
            uninflected: Some(uninflected.clone()),
            irregular: Some(irregular.clone()),
        },
        MockRuleset {
            regular: Some(second_regular.clone()),
            uninflected: Some(uninflected),
            irregular: Some(irregular),
        },
    );

    assert_eq!(PhpString::from("second"), inflector.inflect(b"in"));
    assert_eq!(vec!["in"], first_regular.calls());
    assert_eq!(vec!["in"], second_regular.calls());
}

#[test]
fn testInflectReturnsOriginalValueOnNoMatches() {
    let (irregular, uninflected) = unmatched();
    let regular = Arc::new(MockWordInflector::returning("in", "in"));

    let inflector = ruleset_inflector(
        MockRuleset {
            regular: Some(regular.clone()),
            uninflected: Some(uninflected.clone()),
            irregular: Some(irregular.clone()),
        },
        MockRuleset {
            regular: Some(regular),
            uninflected: Some(uninflected),
            irregular: Some(irregular),
        },
    );

    assert_eq!(PhpString::from("in"), inflector.inflect(b"in"));
}
