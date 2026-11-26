//! Ticket/DDC735Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC735Test.php

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

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};

use orm::*;
use ticket::*;

#[tokio::test]
async fn testRemoveElementAppliesOrphanRemoval() {
    let em = ticket::set_up(&["DDC735Product", "DDC735Review"]).await;

    // Create a product and its first review
    let product = new("DDC735Product");
    set(
        &product,
        "reviews",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    let review = new("DDC735Review");
    set_obj(&review, "product", &product);
    // DDC735Product::addReview()
    coll::add(&get(&product, "reviews"), Value::Object(review.clone())).unwrap();

    // Persist and flush
    em.persist(&product).unwrap();
    em.flush().await.unwrap();

    // Now you see it
    assert_eq!(1, count_of(&product, "reviews").await);

    // Remove the review
    let review_id = get(&review, "id");
    remove_element(&product, "reviews", &review).await;
    em.flush().await.unwrap();

    // Now you don't
    assert_eq!(
        0,
        count_of(&product, "reviews").await,
        "count($reviews) should be 0 after removing its only Review"
    );

    // Refresh
    em.refresh(&product).await.unwrap();

    // It should still be 0
    assert_eq!(
        0,
        count_of(&product, "reviews").await,
        "count($reviews) should still be 0 after the refresh"
    );

    // Review should also not be available anymore
    assert!(find(&em, "DDC735Review", review_id).await.is_none());
}
