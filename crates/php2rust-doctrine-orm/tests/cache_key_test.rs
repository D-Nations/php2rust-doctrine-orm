//! Upstream: doctrine/orm tests/Tests/ORM/Cache/CacheKeyTest.php
//!
//! `assertEquals()`/`assertNotEquals()` of the hashes; the anonymous
//! `CacheKey` subclass is a key of another class.

#![allow(non_snake_case)]

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::cache::{CacheKey, CollectionCacheKey, EntityCacheKey};

fn id(values: &[(&str, Value)]) -> PhpArray {
    values
        .iter()
        .map(|(key, value)| (*key, value.clone()))
        .collect()
}

#[test]
fn testEntityCacheKeyIdentifierCollision() {
    let key1 = EntityCacheKey::new("Foo", &id(&[("id", Value::Int(1))]));
    let key2 = EntityCacheKey::new("Bar", &id(&[("id", Value::Int(1))]));
    assert_ne!(key1.hash, key2.hash);
}

#[test]
fn testEntityCacheKeyIdentifierType() {
    let key1 = EntityCacheKey::new("Foo", &id(&[("id", Value::Int(1))]));
    let key2 = EntityCacheKey::new("Foo", &id(&[("id", Value::string("1"))]));
    assert_eq!(key1.hash, key2.hash);
}

#[test]
fn testEntityCacheKeyIdentifierOrder() {
    let key1 = EntityCacheKey::new(
        "Foo",
        &id(&[("foo_bar", Value::Int(1)), ("bar_foo", Value::Int(2))]),
    );
    let key2 = EntityCacheKey::new(
        "Foo",
        &id(&[("bar_foo", Value::Int(2)), ("foo_bar", Value::Int(1))]),
    );
    assert_eq!(key1.hash, key2.hash);
}

#[test]
fn testCollectionCacheKeyIdentifierType() {
    let key1 = CollectionCacheKey::new("Foo", "assoc", &id(&[("id", Value::Int(1))]));
    let key2 = CollectionCacheKey::new("Foo", "assoc", &id(&[("id", Value::string("1"))]));
    assert_eq!(key1.hash, key2.hash);
}

#[test]
fn testCollectionCacheKeyIdentifierOrder() {
    let key1 = CollectionCacheKey::new(
        "Foo",
        "assoc",
        &id(&[("foo_bar", Value::Int(1)), ("bar_foo", Value::Int(2))]),
    );
    let key2 = CollectionCacheKey::new(
        "Foo",
        "assoc",
        &id(&[("bar_foo", Value::Int(2)), ("foo_bar", Value::Int(1))]),
    );
    assert_eq!(key1.hash, key2.hash);
}

#[test]
fn testCollectionCacheKeyIdentifierCollision() {
    let key1 = CollectionCacheKey::new("Foo", "assoc", &id(&[("id", Value::Int(1))]));
    let key2 = CollectionCacheKey::new("Bar", "assoc", &id(&[("id", Value::Int(1))]));
    assert_ne!(key1.hash, key2.hash);
}

#[test]
fn testCollectionCacheKeyAssociationCollision() {
    let key1 = CollectionCacheKey::new("Foo", "assoc1", &id(&[("id", Value::Int(1))]));
    let key2 = CollectionCacheKey::new("Foo", "assoc2", &id(&[("id", Value::Int(1))]));
    assert_ne!(key1.hash, key2.hash);
}

#[test]
fn testConstructor() {
    let key = CacheKey::other("CacheKey@anonymous", "my-hash");
    assert_eq!("my-hash", key.hash());
}
