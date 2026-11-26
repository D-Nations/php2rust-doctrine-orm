//! PersistentCollectionCriteriaTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! The schema has the `tweet` and `vct_manytomany_extralazy` model sets;
//! the `quote` set is left out (no test here uses it, and its fixture
//! method is not called). `Rot13Type` is the tests' DBAL type.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/PersistentCollectionCriteriaTest.php
//! Upstream: doctrine/orm tests/Tests/DbalTypes/Rot13Type.php

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

use std::sync::Arc;

use php_value::{PhpArray, PhpString, Value};
use php2rust_doctrine_collections::Criteria;
use php2rust_doctrine_dbal::platforms::AbstractPlatform;
use php2rust_doctrine_dbal::types::{ConversionError, Type, TypeArgument, add_type, has_type, override_type};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject, CollectionRef};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;

const TWEET: &str = "Doctrine\\Tests\\Models\\Tweet\\Tweet";
const TWEET_USER: &str = "Doctrine\\Tests\\Models\\Tweet\\User";
const TWEET_USER_LIST: &str = "Doctrine\\Tests\\Models\\Tweet\\UserList";
const OWNING: &str = "Doctrine\\Tests\\Models\\ValueConversionType\\OwningManyToManyExtraLazyEntity";
const INVERSED: &str =
    "Doctrine\\Tests\\Models\\ValueConversionType\\InversedManyToManyExtraLazyEntity";

/// `Doctrine\Tests\DbalTypes\Rot13Type`: shifts every letter by 13 places
/// in the alphabet (ROT13 encoding).
#[derive(Debug)]
struct Rot13Type;

fn str_rot13(value: &Value) -> Value {
    let Value::String(s) = value else {
        return value.clone();
    };
    let shifted: Vec<u8> = s
        .as_bytes()
        .iter()
        .map(|&b| match b {
            b'a'..=b'z' => (b - b'a' + 13) % 26 + b'a',
            b'A'..=b'Z' => (b - b'A' + 13) % 26 + b'A',
            other => other,
        })
        .collect();
    Value::String(PhpString::from(shifted))
}

impl Type for Rot13Type {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\DbalTypes\\Rot13Type"
    }

    fn get_sql_declaration(
        &self,
        column: &PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<PhpString, php2rust_doctrine_dbal::Exception> {
        platform.get_string_type_declaration_sql(column)
    }

    fn convert_to_database_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        Ok(str_rot13(value))
    }

    fn convert_to_php_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        Ok(str_rot13(value))
    }
}

/// `Type::addType('rot13', Rot13Type::class)` (or `overrideType()`).
fn register_rot13_type() {
    let type_ = || TypeArgument::Instance(Arc::new(Rot13Type));
    if has_type("rot13") {
        override_type("rot13", type_()).unwrap();
    } else if add_type("rot13", type_()).is_err() {
        // Another test thread may have added it meanwhile.
        override_type("rot13", type_()).unwrap();
    }
}

async fn set_up_models() -> Arc<EntityManager> {
    register_rot13_type();
    set_up(&[TWEET_USER, TWEET, TWEET_USER_LIST, INVERSED, OWNING]).await
}

fn tweet_user(name: &str) -> php_value::Object {
    let user = new_object(TWEET_USER);
    set(&user, "tweets", ArrayCollectionObject::value(PhpArray::new()));
    set(
        &user,
        "userLists",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(&user, "name", s(name));
    user
}

/// `$user->addTweet($tweet)`.
fn add_tweet(user: &php_value::Object, tweet: &php_value::Object) {
    set_obj(tweet, "author", user);
    append(user, "tweets", Value::Object(tweet.clone()));
}

async fn load_tweet_fixture(em: &EntityManager) {
    let author = tweet_user("ngal");
    em.persist(&author).unwrap();

    let tweet1 = new_object(TWEET);
    set(&tweet1, "content", s("Foo"));
    add_tweet(&author, &tweet1);

    let tweet2 = new_object(TWEET);
    set(&tweet2, "content", s("Bar"));
    add_tweet(&author, &tweet2);

    em.flush().await.unwrap();
    em.clear().unwrap();
}

#[tokio::test]
async fn testCanCountWithoutLoadingPersistentCollection() {
    let em = set_up_models().await;
    load_tweet_fixture(&em).await;

    let repository = em.get_repository(TWEET_USER).unwrap();

    let mut criteria = PhpArray::new();
    criteria.insert("name", s("ngal"));
    let user = repository.find_one_by(&criteria, None).await.unwrap().unwrap();
    let tweets = coll::matching(&get(&user, "tweets"), &Criteria::create())
        .await
        .unwrap();

    let Some(CollectionRef::Lazy(tweets)) = CollectionRef::of(&tweets) else {
        panic!("a LazyCriteriaCollection expected");
    };
    assert!(!tweets.is_initialized());
    assert_eq!(tweets.count().await.unwrap(), 2);
    assert!(!tweets.is_initialized());

    // Make sure it works with constraints
    let mut criteria = Criteria::create();
    criteria.r#where(Criteria::expr().eq("content", s("Foo")));
    let tweets = coll::matching(&get(&user, "tweets"), &criteria)
        .await
        .unwrap();

    let Some(CollectionRef::Lazy(tweets)) = CollectionRef::of(&tweets) else {
        panic!("a LazyCriteriaCollection expected");
    };
    assert!(!tweets.is_initialized());
    assert_eq!(tweets.count().await.unwrap(), 1);
    assert!(!tweets.is_initialized());
}

#[tokio::test]
async fn testCanHandleComplexTypesOnAssociation() {
    let em = set_up_models().await;
    let parent = new_object(OWNING);
    set(
        &parent,
        "associatedEntities",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(&parent, "id2", s("Alice"));

    em.persist(&parent).unwrap();

    let child = new_object(INVERSED);
    set(
        &child,
        "associatedEntities",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(&child, "id1", s("Bob"));

    em.persist(&child).unwrap();

    append(&parent, "associatedEntities", Value::Object(child));

    em.flush().await.unwrap();
    em.clear().unwrap();

    let parent = em.find(OWNING, get(&parent, "id2")).await.unwrap().unwrap();

    let mut criteria = Criteria::create();
    criteria.r#where(Criteria::expr().eq("id1", s("Bob")));

    let result = coll::matching(&get(&parent, "associatedEntities"), &criteria)
        .await
        .unwrap();

    assert_eq!(coll::count(&result).await.unwrap(), 1);
    let Value::Object(first) = coll::get(&result, 0).await.unwrap() else {
        panic!("an entity expected");
    };
    assert!(php_value::compare::loose_equals(&get(&first, "id1"), &s("Bob")));
}
