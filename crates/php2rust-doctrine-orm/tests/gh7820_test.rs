//! Ticket/GH7820Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models; `GH7820LineTextType` and `GH7820LineText` are Rust types).
//! The functional test case's query cache is a new `ArrayAdapter` per test
//! (PHP shares one between the tests of a run); the anonymous `ArrayAdapter`
//! subclasses are pools over an `ArrayAdapter`.
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

use php_value::{Object, PhpArray, PhpObject, PhpString, Thrown, Value};
use php2rust_doctrine_dbal::platforms::AbstractPlatform;
use php2rust_doctrine_dbal::types::{ConversionError, StringType, Type};
use std::sync::atomic::{AtomicBool, Ordering};

use php2rust_doctrine_orm::cache::pool::{ArrayAdapter, CacheItem, CacheItemPool};
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
    em.get_configuration()
        .set_query_cache(Arc::new(ArrayAdapter::new()));

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

/// GH7837
#[tokio::test]
async fn testWillFindSongsInPaginatorEvenWithCachedQueryParsing() {
    let em = set_up().await;
    // Enable the query cache
    em.get_configuration().get_query_cache().unwrap().clear("");

    let song: Vec<Value> = SONG.iter().map(|line| s(line)).collect();
    let assert_same = |lines: &[Value], message: &str| {
        assert_eq!(song.len(), lines.len(), "{message}");
        for (expected, actual) in song.iter().zip(lines) {
            assert!(expected.identical(actual), "{message}");
        }
    };

    // Fetch song lines with the paginator, also priming the query cache
    let lines = fetch_song_lines_with_paginator(&em).await;
    assert_same(
        &lines,
        "Expected to return expected data before query cache is populated with DQL -> SQL translation. Were SQL parameters translated?",
    );

    // Fetch song lines again
    let lines = fetch_song_lines_with_paginator(&em).await;
    assert_same(
        &lines,
        "Expected to return expected data even when DQL -> SQL translation is present in cache. Were SQL parameters translated again?",
    );
}

/// The `ArrayAdapter` whose `save()` asserts that it does not overwrite an
/// entry.
#[derive(Debug, Default)]
struct NoOverwriteAdapter(ArrayAdapter);

impl CacheItemPool for NoOverwriteAdapter {
    fn get_item(&self, key: &str) -> Result<CacheItem, Thrown> {
        self.0.get_item(key)
    }

    fn get_items(&self, keys: &[String]) -> Result<Vec<CacheItem>, Thrown> {
        self.0.get_items(keys)
    }

    fn has_item(&self, key: &str) -> Result<bool, Thrown> {
        self.0.has_item(key)
    }

    fn clear(&self, prefix: &str) -> bool {
        self.0.clear(prefix)
    }

    fn delete_item(&self, key: &str) -> Result<bool, Thrown> {
        self.0.delete_item(key)
    }

    fn save(&self, item: CacheItem) -> bool {
        assert!(
            !self.has_item(item.get_key()).unwrap(),
            "The cache should not have to overwrite the entry"
        );
        self.0.save(item)
    }
}

#[tokio::test]
async fn testPaginatorDoesNotForceCacheToUpdateEntries() {
    let em = set_up().await;
    em.get_configuration()
        .set_query_cache(Arc::new(NoOverwriteAdapter::default()));

    // "Prime" the cache (in fact, that should not even happen)
    fetch_song_lines_with_paginator(&em).await;

    // Make sure we can query again without overwriting the cache
    fetch_song_lines_with_paginator(&em).await;
}

/// The `ArrayAdapter` whose `getItem()` asserts a hit once
/// `failOnCacheMiss()` is called.
#[derive(Debug, Default)]
struct FailOnMissAdapter {
    inner: ArrayAdapter,
    fail_on_cache_miss: AtomicBool,
}

impl FailOnMissAdapter {
    fn fail_on_cache_miss(&self) {
        self.fail_on_cache_miss.store(true, Ordering::SeqCst);
    }
}

impl CacheItemPool for FailOnMissAdapter {
    fn get_item(&self, key: &str) -> Result<CacheItem, Thrown> {
        let item = self.inner.get_item(key)?;
        assert!(
            !self.fail_on_cache_miss.load(Ordering::SeqCst) || item.is_hit(),
            "cache was missed"
        );
        Ok(item)
    }

    fn get_items(&self, keys: &[String]) -> Result<Vec<CacheItem>, Thrown> {
        self.inner.get_items(keys)
    }

    fn has_item(&self, key: &str) -> Result<bool, Thrown> {
        self.inner.has_item(key)
    }

    fn clear(&self, prefix: &str) -> bool {
        self.inner.clear(prefix)
    }

    fn delete_item(&self, key: &str) -> Result<bool, Thrown> {
        self.inner.delete_item(key)
    }

    fn save(&self, item: CacheItem) -> bool {
        self.inner.save(item)
    }
}

#[tokio::test]
async fn testPaginatorQueriesWillBeCached() {
    let em = set_up().await;
    let cache = Arc::new(FailOnMissAdapter::default());
    em.get_configuration().set_query_cache(cache.clone());

    // Prime the cache
    fetch_song_lines_with_paginator(&em).await;

    cache.fail_on_cache_miss();

    fetch_song_lines_with_paginator(&em).await;
}
