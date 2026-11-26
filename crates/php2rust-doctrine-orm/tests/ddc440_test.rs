//! Ticket/DDC440Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC440Test.php

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

use php_value::{Object, Value};
use php2rust_doctrine_orm::collection_object as coll;

use orm::*;
use ticket::*;

/// `DDC440Phone::setClient($value)`: also `$value->addPhone($this)`.
fn set_client(phone: &Object, client: &Object) {
    set_obj(phone, "client", client);
    // DDC440Client::addPhone(): $this->phones[] = $value;
    append(client, "phones", Value::Object(phone.clone()));
}

#[tokio::test]
async fn testOriginalEntityDataEmptyWhenProxyLoadedFromTwoAssociations() {
    let em = ticket::set_up(&["DDC440Phone", "DDC440Client"]).await;

    //Initialize some data
    let client = new("DDC440Client");
    set(&client, "name", s("Client1"));

    let phone = new("DDC440Phone");
    set(&phone, "id", Value::Int(1));
    set(&phone, "number", s("418 111-1111"));
    set_client(&phone, &client);

    let phone2 = new("DDC440Phone");
    set(&phone2, "id", Value::Int(2));
    set(&phone2, "number", s("418 222-2222"));
    set_client(&phone2, &client);

    set_obj(&client, "mainPhone", &phone);

    em.persist(&client).unwrap();
    em.flush().await.unwrap();
    let id = get(&client, "id");
    em.clear().unwrap();

    let uw = em.get_unit_of_work();
    let client = find(&em, "DDC440Client", id).await.unwrap();
    let client_phones = get(&client, "phones");

    let p1 = coll::get(&client_phones, 1).await.unwrap();
    let p2 = coll::get(&client_phones, 2).await.unwrap();

    // Test the first phone.  The assertion actually failed because original entity data is not set properly.
    // This was because it is also set as MainPhone and that one is created as a proxy, not the
    // original object when the find on Client is called. However loading proxies did not work correctly.
    assert_instance_of("DDC440Phone", &p1);
    let original_data = uw.get_original_entity_data(&obj(&p1));
    assert_loose_eq(
        &get(&phone, "number"),
        &at(&Value::Array(original_data), "number"),
    );

    //If you comment out previous test, this one should pass
    assert_instance_of("DDC440Phone", &p2);
    let original_data = uw.get_original_entity_data(&obj(&p2));
    assert_loose_eq(
        &get(&phone2, "number"),
        &at(&Value::Array(original_data), "number"),
    );
}
