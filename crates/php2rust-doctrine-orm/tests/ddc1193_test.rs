//! Ticket/DDC1193Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//! A lazy proxy's `isUninitializedObject()` is the entity manager's.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1193Test.php

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

const COMPANY: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC1193Company";
const PERSON: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC1193Person";
const ACCOUNT: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC1193Account";

#[tokio::test]
async fn testIssue() {
    let em = orm::set_up(&[COMPANY, PERSON, ACCOUNT]).await;

    let company = new_object(COMPANY);
    let person = new_object(PERSON);
    let account = new_object(ACCOUNT);

    set_obj(&person, "account", &account);
    set_obj(&company, "member", &person);

    em.persist(&company).unwrap();

    em.flush().await.unwrap();

    let company_id = get(&company, "id");

    em.clear().unwrap();

    let company = em.find(COMPANY, company_id).await.unwrap().unwrap();

    let uow = em.get_unit_of_work();
    assert!(
        uow.is_in_identity_map(&company).unwrap(),
        "Company is in identity map."
    );
    let member = get_object(&company, "member").unwrap();
    assert!(
        em.is_uninitialized_object(&Value::Object(member.clone())),
        "Pre-Condition"
    );
    assert!(
        uow.is_in_identity_map(&member).unwrap(),
        "Member is in identity map."
    );

    em.remove(&company).await.unwrap();
    em.flush().await.unwrap();

    assert_eq!(
        0,
        em.get_repository(ACCOUNT)
            .unwrap()
            .find_all()
            .await
            .unwrap()
            .len()
    );
}
