//! InflectorFactoryTest.php at doctrine/inflector 6d6c96277ea252fc1304627204c3d5e6e15faa3b.
//! Names and assertions are PHPUnit's; `assertInstanceOf()` is a downcast.
//!
//! Upstream: doctrine/inflector tests/InflectorFactoryTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

use std::any::Any;

use php2rust_doctrine_inflector::rules::{
    english, esperanto, french, norwegian_bokmal, portuguese, spanish, turkish,
};
use php2rust_doctrine_inflector::{
    InflectorError, InflectorFactory, Language, LanguageInflectorFactory,
};

fn is<T: Any>(factory: &dyn LanguageInflectorFactory) -> bool {
    (factory as &dyn Any).is::<T>()
}

#[test]
fn testCreateUsesEnglishByDefault() {
    assert!(is::<english::InflectorFactory>(
        &*InflectorFactory::create().unwrap()
    ));
}

#[test]
fn testCreateForLanguageWithCustomLanguage() {
    type Check = fn(&dyn LanguageInflectorFactory) -> bool;
    let provide_languages: [(&str, Check, &str); 7] = [
        (
            "English",
            is::<english::InflectorFactory>,
            Language::ENGLISH,
        ),
        (
            "Esperanto",
            is::<esperanto::InflectorFactory>,
            Language::ESPERANTO,
        ),
        ("French", is::<french::InflectorFactory>, Language::FRENCH),
        (
            "Norwegian Bokmal",
            is::<norwegian_bokmal::InflectorFactory>,
            Language::NORWEGIAN_BOKMAL,
        ),
        (
            "Portuguese",
            is::<portuguese::InflectorFactory>,
            Language::PORTUGUESE,
        ),
        (
            "Spanish",
            is::<spanish::InflectorFactory>,
            Language::SPANISH,
        ),
        (
            "Turkish",
            is::<turkish::InflectorFactory>,
            Language::TURKISH,
        ),
    ];
    for (name, expected_class, language) in provide_languages {
        assert!(
            expected_class(&*InflectorFactory::create_for_language(language).unwrap()),
            "{name}"
        );
    }
}

#[test]
fn testCreateForLanguageThrowsInvalidArgumentExceptionForUnsupportedLanguage() {
    let Err(InflectorError::Thrown(thrown)) = InflectorFactory::create_for_language("invalid")
    else {
        panic!("expected an exception");
    };
    assert_eq!("InvalidArgumentException", thrown.class);
    assert_eq!(
        "Language \"invalid\" is not supported.",
        thrown.message.to_string_lossy()
    );
}
