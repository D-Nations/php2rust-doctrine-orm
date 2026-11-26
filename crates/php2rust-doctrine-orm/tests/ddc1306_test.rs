//! Ticket/DDC1306Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the cms models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1306Test.php

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

#[tokio::test]
async fn testIssue() {
    let em = orm::set_up(&CMS_MODELS).await;

    let phone = cms_phonenumber("1234");

    // puts user and phone into commit order calculator
    em.persist(&phone).unwrap();
    em.flush().await.unwrap();

    let address = cms_address("Germany", "12345", "bonn");
    set(&address, "street", s("somestreet!"));
    set(&address, "zip", Value::Int(12345));
    em.persist(&address).unwrap();

    let user = cms_user_with("benjamin", "beberlei", "active");
    set_address(&user, &address);

    // puts user and address into commit order calculator, but does not calculate user dependencies new
    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    em.remove(&get_object(&user, "address").unwrap())
        .await
        .unwrap();
    em.remove(&user).await.unwrap();
    em.flush().await.unwrap();

    assert!(
        em.get_repository(CMS_ADDRESS)
            .unwrap()
            .find_all()
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        em.get_repository(CMS_USER)
            .unwrap()
            .find_all()
            .await
            .unwrap()
            .is_empty()
    );
}
