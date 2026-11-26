//! Ticket/DDC2252Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2252Test.php

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

/// `DDC2252Membership::addPrivilege($privilege)`.
fn add_privilege(membership: &Object, privilege: &Object) {
    coll::add(
        &get(membership, "privileges"),
        Value::Object(privilege.clone()),
    )
    .unwrap();
}

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&[
        "DDC2252User",
        "DDC2252Privilege",
        "DDC2252Membership",
        "DDC2252MerchantAccount",
    ])
    .await;

    // loadFixtures()
    let user = new("DDC2252User");
    set(
        &user,
        "memberships",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    let merchant = new("DDC2252MerchantAccount");
    // new DDC2252Membership($this->user, $this->merchant)
    let membership = new("DDC2252Membership");
    set_obj(&membership, "userAccount", &user);
    set_obj(&membership, "merchantAccount", &merchant);
    set(
        &membership,
        "privileges",
        ArrayCollectionObject::value(PhpArray::new()),
    );

    let privileges = [
        new("DDC2252Privilege"),
        new("DDC2252Privilege"),
        new("DDC2252Privilege"),
    ];

    add_privilege(&membership, &privileges[0]);
    add_privilege(&membership, &privileges[1]);
    add_privilege(&membership, &privileges[2]);

    em.persist(&user).unwrap();
    em.persist(&merchant).unwrap();
    em.persist(&privileges[0]).unwrap();
    em.persist(&privileges[1]).unwrap();
    em.persist(&privileges[2]).unwrap();
    em.flush().await.unwrap();

    em.persist(&membership).unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    // testIssue()
    let identifier = array(&[
        ("merchantAccount", get(&merchant, "accountid")),
        ("userAccount", get(&user, "uid")),
    ]);

    let membership = find(&em, "DDC2252Membership", identifier.clone())
        .await
        .unwrap();

    assert_instance_of("DDC2252Membership", &Value::Object(membership.clone()));
    assert_eq!(3, count_of(&membership, "privileges").await);

    coll::remove(&get(&membership, "privileges"), 2)
        .await
        .unwrap();
    em.persist(&membership).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let membership = find(&em, "DDC2252Membership", identifier.clone())
        .await
        .unwrap();

    assert_instance_of("DDC2252Membership", &Value::Object(membership.clone()));
    assert_eq!(2, count_of(&membership, "privileges").await);

    coll::clear(&get(&membership, "privileges")).await.unwrap();
    em.persist(&membership).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let membership = find(&em, "DDC2252Membership", identifier.clone())
        .await
        .unwrap();

    assert_instance_of("DDC2252Membership", &Value::Object(membership.clone()));
    assert_eq!(0, count_of(&membership, "privileges").await);

    let privilege3 = new("DDC2252Privilege");
    add_privilege(&membership, &privilege3);
    em.persist(&privilege3).unwrap();
    em.persist(&membership).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let membership = find(&em, "DDC2252Membership", identifier).await.unwrap();

    assert_instance_of("DDC2252Membership", &Value::Object(membership.clone()));
    assert_eq!(1, count_of(&membership, "privileges").await);
}
