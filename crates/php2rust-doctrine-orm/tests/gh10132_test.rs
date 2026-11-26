//! Ticket/GH10132Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the Models/GH10132 models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH10132Test.php
//! Upstream: doctrine/orm tests/Tests/Models/GH10132/ComplexChild.php

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

const COMPLEX: &str = "Doctrine\\Tests\\Models\\GH10132\\Complex";
const COMPLEX_CHILD: &str = "Doctrine\\Tests\\Models\\GH10132\\ComplexChild";
const SUIT: &str = "Doctrine\\Tests\\Models\\Enums\\Suit";

#[tokio::test]
async fn testQueryBackedEnumInCompositeKeyJoin() {
    let em = orm::set_up(&[COMPLEX, COMPLEX_CHILD]).await;

    let complex = new_object(COMPLEX);
    set(
        &complex,
        "complexChildren",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(&complex, "type", ticket::case(SUIT, "Clubs"));

    let complex_child = new_object(COMPLEX_CHILD);
    // ComplexChild::setComplex()
    coll::add(
        &get(&complex, "complexChildren"),
        Value::Object(complex_child.clone()),
    )
    .unwrap();
    set(&complex_child, "complexType", get(&complex, "type"));
    set_obj(&complex_child, "complex", &complex);

    em.persist(&complex).unwrap();
    em.persist(&complex_child).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let mut qb = em.create_query_builder();
    qb.select(vec![s("s")])
        .unwrap()
        .from(COMPLEX_CHILD, "s", None)
        .unwrap()
        .r#where(vec![s("s.complexType = :complexType")])
        .unwrap();
    qb.set_parameter("complexType", ticket::case(SUIT, "Clubs"), None);

    assert!(!matches!(
        qb.get_query()
            .unwrap()
            .get_one_or_null_result(None)
            .await
            .unwrap(),
        Value::Null
    ));
}
