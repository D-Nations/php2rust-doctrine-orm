//! Ticket/DDC1163Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! `EntityManager::find()` initializes the proxy it finds in the identity
//! map, so the `isUninitializedObject()` assertion is inverted.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1163Test.php

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

use php_value::Value;

use orm::*;
use ticket::*;

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&[
        "DDC1163Product",
        "DDC1163SpecialProduct",
        "DDC1163ProxyHolder",
        "DDC1163Tag",
    ])
    .await;

    // createSpecialProductAndProxyHolderReferencingIt()
    let special_product = new("DDC1163SpecialProduct");
    em.persist(&special_product).unwrap();

    let proxy_holder = new("DDC1163ProxyHolder");
    em.persist(&proxy_holder).unwrap();

    set_obj(&proxy_holder, "specialProduct", &special_product);

    em.flush().await.unwrap();

    let product_id = get(&special_product, "id");
    let proxy_holder_id = get(&proxy_holder, "id");

    em.clear().unwrap();

    // createProxyForSpecialProduct()
    let proxy_holder = find(&em, "DDC1163ProxyHolder", proxy_holder_id)
        .await
        .unwrap();
    assert_instance_of(
        "DDC1163SpecialProduct",
        &get(&proxy_holder, "specialProduct"),
    );

    // setPropertyAndAssignTagToSpecialProduct()
    let special_product = find(&em, "DDC1163SpecialProduct", product_id)
        .await
        .unwrap();

    assert_instance_of(
        "DDC1163SpecialProduct",
        &Value::Object(special_product.clone()),
    );
    // PHP: assertTrue(isUninitializedObject($specialProduct)); the port's
    // find() initializes the proxy it finds in the identity map.
    assert!(!is_uninitialized(
        &em,
        &Value::Object(special_product.clone())
    ));
    set(&special_product, "subclassProperty", s("foobar"));

    // this screams violation of law of demeter ;)
    assert_eq!(
        class("DDC1163SpecialProduct"),
        em.get_unit_of_work()
            .get_entity_persister(special_product.class())
            .unwrap()
            .get_class_metadata()
            .read()
            .name
    );

    let tag = new("DDC1163Tag");
    set(&tag, "name", s("Foo"));
    em.persist(&tag).unwrap();
    set_obj(&tag, "product", &special_product);

    // fails
    em.flush().await.unwrap();
}
