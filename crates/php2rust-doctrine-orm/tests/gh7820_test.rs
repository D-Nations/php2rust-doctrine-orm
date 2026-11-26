//! Ticket/GH7820Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models; `GH7820LineTextType` and `GH7820LineText` are Rust types).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH7820Test.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;
#[path = "common/ticket.rs"]
mod ticket;

use std::sync::Arc;

use php_value::{Object, PhpArray, PhpObject, PhpString, Value};
use php2rust_doctrine_dbal::platforms::AbstractPlatform;
use php2rust_doctrine_dbal::types::{ConversionError, StringType, Type};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::query::SortDirection;
use php2rust_doctrine_orm::tools::pagination::{OffsetPaginator, Page, Window};

use orm::*;
use ticket::*;

const GH7820_LINE_TEXT_TYPE: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH7820LineTextType";
const GH7820_LINE_TEXT: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH7820LineText";

const SONG: [&str; 6] = [
    "What is this song all about?",
    "Can't figure any lyrics out",
    "How do the words to it go?",
    "I wish you'd tell me, I don't know",
    "Don't know, don't know, don't know, I don't know!",
    "Don't know, don't know, don't know...",
];

/// `GH7820LineText implements Stringable`.
#[derive(Debug)]
struct GH7820LineText {
    text: PhpString,
}

impl GH7820LineText {
    /// `GH7820LineText::fromText($text)`.
    fn from_text(text: impl Into<PhpString>) -> Value {
        let line: Object = Arc::new(Self { text: text.into() });
        Value::Object(line)
    }

    /// `$value instanceof GH7820LineText ? $value->getText() : null`.
    fn get_text(value: &Value) -> Option<PhpString> {
        match value {
            Value::Object(object) if object.class() == GH7820_LINE_TEXT => object
                .properties()
                .get(&"text".into())
                .and_then(Value::to_php_string),
            _ => None,
        }
    }
}

impl PhpObject for GH7820LineText {
    fn class(&self) -> &str {
        GH7820_LINE_TEXT
    }

    fn properties(&self) -> PhpArray {
        let mut properties = PhpArray::new();
        properties.insert("text", Value::String(self.text.clone()));
        properties
    }

    fn to_php_string(&self) -> Option<PhpString> {
        let mut text = b"Line: ".to_vec();
        text.extend_from_slice(self.text.as_bytes());
        Some(PhpString::from(text))
    }
}

/// `GH7820LineTextType extends StringType`.
#[derive(Debug)]
struct GH7820LineTextType;

impl Type for GH7820LineTextType {
    fn class(&self) -> &'static str {
        GH7820_LINE_TEXT_TYPE
    }

    fn get_sql_declaration(
        &self,
        column: &PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<PhpString, php2rust_doctrine_dbal::Exception> {
        StringType.get_sql_declaration(column, platform)
    }

    fn convert_to_php_value(
        &self,
        value: &Value,
        platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        let text = StringType.convert_to_php_value(value, platform)?;

        let Value::String(text) = text else {
            return Ok(text);
        };

        Ok(GH7820LineText::from_text(text))
    }

    fn convert_to_database_value(
        &self,
        value: &Value,
        platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        let Some(text) = GH7820LineText::get_text(value) else {
            return StringType.convert_to_database_value(value, platform);
        };

        StringType.convert_to_database_value(&Value::String(text), platform)
    }
}

async fn set_up() -> Arc<EntityManager> {
    if !php2rust_doctrine_dbal::types::has_type(GH7820_LINE_TEXT_TYPE) {
        ticket::add_type(GH7820_LINE_TEXT_TYPE, Arc::new(GH7820LineTextType));
    }

    let em = ticket::set_up(&["GH7820Line"]).await;

    em.create_query(&format!("DELETE FROM {} l", class("GH7820Line")))
        .execute(None, None)
        .await
        .unwrap();

    for (index, line) in SONG.iter().enumerate() {
        let entity = new("GH7820Line");
        set(&entity, "text", GH7820LineText::from_text(*line));
        set(
            &entity,
            "lineNumber",
            Value::Int(i64::try_from(index).unwrap()),
        );
        em.persist(&entity).unwrap();
    }

    em.flush().await.unwrap();
    em
}

async fn fetch_song_lines_with_paginator(em: &Arc<EntityManager>) -> Vec<Value> {
    let mut query = em
        .get_repository(&class("GH7820Line"))
        .unwrap()
        .create_query_builder("l", None)
        .unwrap();
    query
        .order_by("l.lineNumber", Some(SortDirection::Ascending.into()))
        .unwrap();

    let page = OffsetPaginator::new(true, None)
        .paginate_window((&query).into(), Window::new(0, 100).unwrap())
        .await
        .unwrap();

    // `$line->toString()`: the text.
    page.get_items()
        .iter()
        .map(|line| Value::String(GH7820LineText::get_text(&get(&obj(line), "text")).unwrap()))
        .collect()
}

#[tokio::test]
async fn testWillFindSongsInPaginator() {
    let em = set_up().await;
    let lines = fetch_song_lines_with_paginator(&em).await;

    let song: Vec<Value> = SONG.iter().map(|line| s(line)).collect();
    assert_eq!(song.len(), lines.len());
    for (expected, actual) in song.iter().zip(&lines) {
        assert!(expected.identical(actual), "{expected:?} !== {actual:?}");
    }
}
