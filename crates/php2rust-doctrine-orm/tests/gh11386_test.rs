//! Ticket/GH11386/GH11386Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models). `clone $cart->getCustomer()` is the initialized proxy copied
//! into a new object of its class (PHP 8.4 initializes a lazy object before
//! cloning it).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11386/GH11386Test.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11386/GH11386EntityCart.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11386/GH11386EntityCustomer.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11386/GH11386EnumType.php

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

const GH11386_ENTITY_CART: &str =
    "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH11386\\GH11386EntityCart";
const GH11386_ENTITY_CUSTOMER: &str =
    "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH11386\\GH11386EntityCustomer";
const GH11386_ENUM_TYPE: &str =
    "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH11386\\GH11386EnumType";

#[tokio::test]
async fn testInitializeClonedProxy() {
    let em = orm::set_up(&[GH11386_ENTITY_CART, GH11386_ENTITY_CUSTOMER]).await;
    let cart = new_object(GH11386_ENTITY_CART);
    set(&cart, "amount", Value::Int(1000));
    let customer = new_object(GH11386_ENTITY_CUSTOMER);
    set(&customer, "name", s("John Doe"));
    set(&customer, "type", ticket::case(GH11386_ENUM_TYPE, "MALE"));
    // $customer->setCart($cart)
    set_obj(&cart, "customer", &customer);
    set_obj(&customer, "cart", &cart);

    em.persist(&cart).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let cart = em
        .find(GH11386_ENTITY_CART, Value::Int(1))
        .await
        .unwrap()
        .expect("a cart");
    let proxy = get_object(&cart, "customer").unwrap();
    em.initialize_object(&proxy).await.unwrap();
    let customer = new_object(proxy.class());
    for (key, value) in proxy.properties().iter() {
        set(
            &customer,
            &String::from_utf8_lossy(&key.bytes()),
            value.clone(),
        );
    }

    assert_loose_eq(&s("John Doe"), &get(&customer, "name"));
}
