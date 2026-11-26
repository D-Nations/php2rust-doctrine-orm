//! Ticket/DDC748Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the cms models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC748Test.php

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

use php_value::Value;

use orm::*;

const COLLECTION: &str = "Doctrine\\Common\\Collections\\Collection";

/// `self::assertInstanceOf(Collection::class, $value)`.
fn assert_collection(value: &Value) {
    assert!(
        matches!(value, Value::Object(o) if o.instance_of(COLLECTION)),
        "{value:?} is not a Collection"
    );
}

#[tokio::test]
async fn testRefreshWithManyToOne() {
    let em = orm::set_up(&CMS_MODELS).await;

    let user = cms_user_with("beberlei", "beberlei", "active");

    let article = cms_article("bar", "foo");
    // $article->setAuthor($user)
    set_obj(&article, "user", &user);

    em.persist(&user).unwrap();
    em.persist(&article).unwrap();
    em.flush().await.unwrap();

    assert_collection(&get(&user, "articles"));
    em.refresh(&article).await.unwrap();
    assert!(
        !same_value(&get(&user, "articles"), &article),
        "The article should not be replaced on the inverse side of the relation."
    );
    assert_collection(&get(&user, "articles"));
}

#[tokio::test]
async fn testRefreshOneToOne() {
    let em = orm::set_up(&CMS_MODELS).await;

    let user = cms_user_with("beberlei", "beberlei", "active");

    let address = cms_address("Germany", "12345", "Bonn");
    set(&address, "street", s("A street"));
    set(&address, "zip", Value::Int(12345));
    set_address(&user, &address);

    em.persist(&user).unwrap();
    em.persist(&address).unwrap();
    em.flush().await.unwrap();

    em.refresh(&address).await.unwrap();
    assert!(same_value(&get(&address, "user"), &user));
    assert!(same_value(&get(&user, "address"), &address));
}
