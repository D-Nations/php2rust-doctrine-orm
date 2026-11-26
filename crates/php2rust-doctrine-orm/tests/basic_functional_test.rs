//! Upstream: doctrine/orm tests/Tests/ORM/Functional/BasicFunctionalTest.php

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use php_value::Value;
use php2rust_doctrine_orm::unit_of_work::{STATE_MANAGED, STATE_NEW, STATE_REMOVED};

use orm::*;

#[tokio::test]
async fn test_basic_units_of_work_with_one_to_many_association() {
    let em = set_up(&CMS_MODELS).await;
    // Create
    let user = cms_user_with("Roman", "romanb", "developer");
    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    let id = get(&user, "id");
    assert!(matches!(id, Value::Int(_)), "{id:?}");
    assert!(em.contains(&user).unwrap());

    // Read
    let user2 = em.find(CMS_USER, id).await.unwrap().unwrap();
    assert!(same(&user, &user2));

    // Add a phonenumber
    let ph = cms_phonenumber("12345");
    add_phonenumber(&user, &ph);
    em.flush().await.unwrap();
    assert!(em.contains(&ph).unwrap());
    assert!(em.contains(&user).unwrap());

    // Update name
    set(&user, "name", s("guilherme"));
    em.flush().await.unwrap();
    assert_eq!(get(&user, "name"), s("guilherme"));

    // Add another phonenumber
    let ph2 = cms_phonenumber("6789");
    add_phonenumber(&user, &ph2);
    em.flush().await.unwrap();
    assert!(em.contains(&ph2).unwrap());

    // Delete
    em.remove(&user).await.unwrap();
    let uow = em.get_unit_of_work();
    assert!(uow.is_scheduled_for_delete(&user));
    assert!(uow.is_scheduled_for_delete(&ph));
    assert!(uow.is_scheduled_for_delete(&ph2));
    em.flush().await.unwrap();
    assert!(!uow.is_scheduled_for_delete(&user));
    assert!(!uow.is_scheduled_for_delete(&ph));
    assert!(!uow.is_scheduled_for_delete(&ph2));
    assert_eq!(uow.get_entity_state(&user).await.unwrap(), STATE_NEW);
    assert_eq!(uow.get_entity_state(&ph).await.unwrap(), STATE_NEW);
    assert_eq!(uow.get_entity_state(&ph2).await.unwrap(), STATE_NEW);
    let _ = (STATE_MANAGED, STATE_REMOVED);
}
