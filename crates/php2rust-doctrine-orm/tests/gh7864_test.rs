//! Ticket/GH7864Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models). `map()` is `get_values()` mapped in the test).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH7864Test.php

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

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};

use orm::*;
use ticket::*;

/// `GH7864User::addTweet($tweet)`.
fn add_tweet(user: &Object, tweet: &Object) {
    set_obj(tweet, "user", user);
    coll::add(&get(user, "tweets"), Value::Object(tweet.clone())).unwrap();
}

#[tokio::test]
async fn testExtraLazyRemoveElement() {
    let em = ticket::set_up(&["GH7864User", "GH7864Tweet"]).await;
    let user = new("GH7864User");
    set(&user, "tweets", ArrayCollectionObject::value(PhpArray::new()));
    set(&user, "name", s("test"));

    let tweet1 = new("GH7864Tweet");
    set(&tweet1, "content", s("Hello World!"));
    add_tweet(&user, &tweet1);

    let tweet2 = new("GH7864Tweet");
    set(&tweet2, "content", s("Goodbye, and thanks for all the fish"));
    add_tweet(&user, &tweet2);

    em.persist(&user).unwrap();
    em.persist(&tweet1).unwrap();
    em.persist(&tweet2).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let user = find(&em, "GH7864User", get(&user, "id")).await.unwrap();
    let tweet = find(&em, "GH7864Tweet", get(&tweet1, "id")).await.unwrap();

    coll::remove_element(&get(&user, "tweets"), &Value::Object(tweet))
        .await
        .unwrap();

    let tweets: Vec<Value> = coll::get_values(&get(&user, "tweets"))
        .await
        .unwrap()
        .iter()
        .map(|tweet| get(&obj(tweet), "content"))
        .collect();

    assert_loose_eq(
        &Value::Array(PhpArray::list([s("Goodbye, and thanks for all the fish")])),
        &Value::Array(PhpArray::list(tweets)),
    );
}
