//! Upstream: doctrine/orm tests/Tests/ORM/Functional/EntityRepositoryTest.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::similar_names,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;
#[path = "common/query_log.rs"]
mod query_log;

use std::sync::Arc;

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_collections::Criteria;
use php2rust_doctrine_dbal::LockMode;
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::exception::{
    INVALID_FIND_BY_CALL, INVALID_MAGIC_METHOD_CALL, INVALID_ORIENTATION,
    OPTIMISTIC_LOCK_EXCEPTION, TRANSACTION_REQUIRED_EXCEPTION, UNRECOGNIZED_FIELD,
    UNRECOGNIZED_IDENTIFIER_FIELDS,
};

use orm::*;

fn array<const N: usize>(pairs: [(&str, Value); N]) -> PhpArray {
    let mut array = PhpArray::new();
    for (key, value) in pairs {
        array.insert(key, value);
    }
    array
}

fn list(values: impl IntoIterator<Item = Value>) -> Value {
    Value::Array(PhpArray::list(values))
}

async fn load_fixture(em: &EntityManager) -> Value {
    let user = cms_user_with("Roman", "romanb", "freak");
    em.persist(&user).unwrap();
    let user2 = cms_user_with("Guilherme", "gblanco", "dev");
    em.persist(&user2).unwrap();
    let user3 = cms_user_with("Benjamin", "beberlei", "");
    set(&user3, "status", Value::Null);
    em.persist(&user3).unwrap();
    let user4 = cms_user_with("Alexander", "asm89", "dev");
    em.persist(&user4).unwrap();
    em.flush().await.unwrap();
    let user1_id = get(&user, "id");
    em.clear().unwrap();
    user1_id
}

async fn load_associated_fixture(em: &EntityManager) -> (Value, Value) {
    let address = cms_address("Germany", "12345", "Berlin");
    set(&address, "street", s("Foostreet"));
    let user = cms_user_with("Roman", "romanb", "freak");
    set_address(&user, &address);
    em.persist(&user).unwrap();
    em.persist(&address).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();
    (get(&user, "id"), get(&address, "id"))
}

async fn load_fixture_user_email(em: &EntityManager) -> Vec<Object> {
    let mut users = Vec::new();
    for i in 1..=3 {
        let user = cms_user_with(&format!("Test {i}"), &format!("test{i}"), "active");
        let email = cms_email(&format!("test{i}@test.com"));
        set_email(&user, &email);
        users.push((user, email));
    }
    for (user, _) in &users {
        em.persist(user).unwrap();
    }
    for (_, email) in &users {
        em.persist(email).unwrap();
    }
    em.flush().await.unwrap();
    em.clear().unwrap();
    users.into_iter().map(|(user, _)| user).collect()
}

async fn build_user(
    em: &EntityManager,
    name: &str,
    username: &str,
    status: &str,
    address: &Object,
) -> Object {
    let user = cms_user_with(name, username, status);
    set_address(&user, address);
    em.persist(&user).unwrap();
    em.flush().await.unwrap();
    user
}

async fn build_address(
    em: &EntityManager,
    country: &str,
    city: &str,
    street: &str,
    zip: &str,
) -> Object {
    let address = cms_address(country, zip, city);
    set(&address, "street", s(street));
    em.persist(&address).unwrap();
    em.flush().await.unwrap();
    address
}

async fn three_users_with_addresses(em: &Arc<EntityManager>) -> (Object, Object, Object) {
    let address1 = build_address(em, "Germany", "Berlim", "Foo st.", "123456").await;
    let user1 = build_user(em, "Benjamin", "beberlei", "dev", &address1).await;
    let address2 = build_address(em, "Brazil", "São Paulo", "Bar st.", "654321").await;
    let user2 = build_user(em, "Guilherme", "guilhermeblanco", "freak", &address2).await;
    let address3 = build_address(em, "USA", "Nashville", "Woo st.", "321654").await;
    let user3 = build_user(em, "Jonathan", "jwage", "dev", &address3).await;
    em.clear().unwrap();
    (user1, user2, user3)
}

fn objects(value: Value) -> Vec<Object> {
    let Value::Array(array) = value else {
        panic!("an array: {value:?}");
    };
    array
        .values()
        .map(|v| match v {
            Value::Object(o) => o.clone(),
            other => panic!("an object: {other:?}"),
        })
        .collect()
}

#[tokio::test]
async fn testBasicFind() {
    let em = set_up(&CMS_MODELS).await;
    let user1_id = load_fixture(&em).await;
    let repos = em.get_repository(CMS_USER).unwrap();

    let user = repos.find(user1_id).await.unwrap().unwrap();
    assert!(user.instance_of(CMS_USER));
    assert_eq!(get(&user, "name"), s("Roman"));
    assert_eq!(get(&user, "status"), s("freak"));
}

#[tokio::test]
async fn testFindByField() {
    let em = set_up(&CMS_MODELS).await;
    load_fixture(&em).await;
    let repos = em.get_repository(CMS_USER).unwrap();

    let users = repos
        .find_by(&array([("status", s("dev"))]), None, None, None)
        .await
        .unwrap();
    assert_eq!(users.len(), 2);
    assert!(users[0].instance_of(CMS_USER));
    assert_eq!(get(&users[0], "name"), s("Guilherme"));
    assert_eq!(get(&users[0], "status"), s("dev"));
}

#[tokio::test]
async fn testFindByAssociationWithIntegerAsParameter() {
    let em = set_up(&CMS_MODELS).await;
    let (user1, user2, _user3) = three_users_with_addresses(&em).await;

    let repository = em.get_repository(CMS_ADDRESS).unwrap();
    let criteria = array([("user", list([get(&user1, "id"), get(&user2, "id")]))]);
    let addresses = repository
        .find_by(&criteria, None, None, None)
        .await
        .unwrap();

    assert_eq!(addresses.len(), 2);
    assert!(addresses[0].instance_of(CMS_ADDRESS));
}

#[tokio::test]
async fn testFindByAssociationWithObjectAsParameter() {
    let em = set_up(&CMS_MODELS).await;
    let (user1, user2, _user3) = three_users_with_addresses(&em).await;

    let repository = em.get_repository(CMS_ADDRESS).unwrap();
    let criteria = array([("user", list([Value::Object(user1), Value::Object(user2)]))]);
    let addresses = repository
        .find_by(&criteria, None, None, None)
        .await
        .unwrap();

    assert_eq!(addresses.len(), 2);
    assert!(addresses[0].instance_of(CMS_ADDRESS));
}

#[tokio::test]
async fn testFindFieldByMagicCall() {
    let em = set_up(&CMS_MODELS).await;
    load_fixture(&em).await;
    let repos = em.get_repository(CMS_USER).unwrap();

    let users = objects(repos.call("findByStatus", vec![s("dev")]).await.unwrap());
    assert_eq!(users.len(), 2);
    assert!(users[0].instance_of(CMS_USER));
    assert_eq!(get(&users[0], "name"), s("Guilherme"));
    assert_eq!(get(&users[0], "status"), s("dev"));
}

#[tokio::test]
async fn testFindAll() {
    let em = set_up(&CMS_MODELS).await;
    load_fixture(&em).await;
    let repos = em.get_repository(CMS_USER).unwrap();

    assert_eq!(repos.find_all().await.unwrap().len(), 4);
}

#[tokio::test]
async fn testCount() {
    let em = set_up(&CMS_MODELS).await;
    load_fixture(&em).await;
    let repos = em.get_repository(CMS_USER).unwrap();

    assert_eq!(repos.count(&PhpArray::new()).await.unwrap(), 4);
    assert_eq!(
        repos.count(&array([("status", s("dev"))])).await.unwrap(),
        2
    );
    assert_eq!(
        repos
            .count(&array([("status", s("nonexistent"))]))
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn testCountBy() {
    let em = set_up(&CMS_MODELS).await;
    load_fixture(&em).await;
    let repos = em.get_repository(CMS_USER).unwrap();

    assert_eq!(
        repos.call("countByStatus", vec![s("dev")]).await.unwrap(),
        Value::Int(2)
    );
}

#[tokio::test]
async fn testExceptionIsThrownWhenCallingFindByWithoutParameter() {
    let em = set_up(&CMS_MODELS).await;
    let e = em
        .get_repository(CMS_USER)
        .unwrap()
        .call("findByStatus", vec![])
        .await
        .unwrap_err();
    assert_eq!(e.class, INVALID_MAGIC_METHOD_CALL);
}

#[tokio::test]
async fn testExceptionIsThrownWhenUsingInvalidFieldName() {
    let em = set_up(&CMS_MODELS).await;
    let e = em
        .get_repository(CMS_USER)
        .unwrap()
        .call("findByThisFieldDoesNotExist", vec![s("testvalue")])
        .await
        .unwrap_err();
    assert_eq!(e.class, INVALID_MAGIC_METHOD_CALL);
}

/// locking, DDC-178
#[tokio::test]
async fn testPessimisticReadLockWithoutTransactionThrowsException() {
    let em = set_up(&CMS_MODELS).await;
    let e = em
        .get_repository(CMS_USER)
        .unwrap()
        .find_with_lock(Value::Int(1), LockMode::PessimisticRead, None)
        .await
        .unwrap_err();
    assert_eq!(e.class, TRANSACTION_REQUIRED_EXCEPTION);
}

/// locking, DDC-178
#[tokio::test]
async fn testPessimisticWriteLockWithoutTransactionThrowsException() {
    let em = set_up(&CMS_MODELS).await;
    let e = em
        .get_repository(CMS_USER)
        .unwrap()
        .find_with_lock(Value::Int(1), LockMode::PessimisticWrite, None)
        .await
        .unwrap_err();
    assert_eq!(e.class, TRANSACTION_REQUIRED_EXCEPTION);
}

/// locking, DDC-178
#[tokio::test]
async fn testOptimisticLockUnversionedEntityThrowsException() {
    let em = set_up(&CMS_MODELS).await;
    let e = em
        .get_repository(CMS_USER)
        .unwrap()
        .find_with_lock(Value::Int(1), LockMode::Optimistic, None)
        .await
        .unwrap_err();
    assert_eq!(e.class, OPTIMISTIC_LOCK_EXCEPTION);
}

/// locking, DDC-178
#[tokio::test]
async fn testIdentityMappedOptimisticLockUnversionedEntityThrowsException() {
    let em = set_up(&CMS_MODELS).await;
    let user = cms_user_with("Roman", "romanb", "freak");
    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    let user_id = get(&user, "id");

    em.find(CMS_USER, user_id.clone()).await.unwrap();

    let e = em
        .find_with_lock(CMS_USER, user_id, LockMode::Optimistic, None)
        .await
        .unwrap_err();
    assert_eq!(e.class, OPTIMISTIC_LOCK_EXCEPTION);
}

/// DDC-819
#[tokio::test]
async fn testFindMagicCallByNullValue() {
    let em = set_up(&CMS_MODELS).await;
    load_fixture(&em).await;
    let repos = em.get_repository(CMS_USER).unwrap();

    let users = objects(repos.call("findByStatus", vec![Value::Null]).await.unwrap());
    assert_eq!(users.len(), 1);
}

/// DDC-819
#[tokio::test]
async fn testInvalidMagicCall() {
    let em = set_up(&CMS_MODELS).await;
    let repos = em.get_repository(CMS_USER).unwrap();
    let e = repos.call("foo", vec![]).await.unwrap_err();
    assert_eq!(e.class, "BadMethodCallException");
}

/// DDC-817
#[tokio::test]
async fn testFindByAssociationKeyExceptionOnInverseSide() {
    let em = set_up(&CMS_MODELS).await;
    let (_user_id, address_id) = load_associated_fixture(&em).await;
    let repos = em.get_repository(CMS_USER).unwrap();

    let e = repos
        .find_by(&array([("address", address_id)]), None, None, None)
        .await
        .unwrap_err();
    assert_eq!(e.class, INVALID_FIND_BY_CALL);
    assert_eq!(
        message(&e),
        "You cannot search for the association field 'Doctrine\\Tests\\Models\\CMS\\CmsUser#address', because it is the inverse side of an association. Find methods only work on owning side associations."
    );
}

/// DDC-817
#[tokio::test]
async fn testFindOneByAssociationKey() {
    let em = set_up(&CMS_MODELS).await;
    let (user_id, address_id) = load_associated_fixture(&em).await;
    let repos = em.get_repository(CMS_ADDRESS).unwrap();
    let address = repos
        .find_one_by(&array([("user", user_id)]), None)
        .await
        .unwrap()
        .unwrap();

    assert!(address.instance_of(CMS_ADDRESS));
    assert_eq!(get(&address, "id"), address_id);
}

/// DDC-1241
#[tokio::test]
async fn testFindOneByOrderBy() {
    let em = set_up(&CMS_MODELS).await;
    load_fixture(&em).await;

    let repos = em.get_repository(CMS_USER).unwrap();
    let user_asc = repos
        .find_one_by(&PhpArray::new(), Some(&array([("username", s("ASC"))])))
        .await
        .unwrap()
        .unwrap();
    let user_desc = repos
        .find_one_by(&PhpArray::new(), Some(&array([("username", s("DESC"))])))
        .await
        .unwrap()
        .unwrap();

    assert!(!same(&user_asc, &user_desc));
}

/// DDC-817
#[tokio::test]
async fn testFindByAssociationKey() {
    let em = set_up(&CMS_MODELS).await;
    let (user_id, address_id) = load_associated_fixture(&em).await;
    let repos = em.get_repository(CMS_ADDRESS).unwrap();
    let addresses = repos
        .find_by(&array([("user", user_id)]), None, None, None)
        .await
        .unwrap();

    assert!(addresses.iter().all(|a| a.instance_of(CMS_ADDRESS)));
    assert_eq!(addresses.len(), 1);
    assert_eq!(get(&addresses[0], "id"), address_id);
}

/// DDC-817
#[tokio::test]
async fn testFindAssociationByMagicCall() {
    let em = set_up(&CMS_MODELS).await;
    let (user_id, address_id) = load_associated_fixture(&em).await;
    let repos = em.get_repository(CMS_ADDRESS).unwrap();
    let addresses = objects(repos.call("findByUser", vec![user_id]).await.unwrap());

    assert!(addresses.iter().all(|a| a.instance_of(CMS_ADDRESS)));
    assert_eq!(addresses.len(), 1);
    assert_eq!(get(&addresses[0], "id"), address_id);
}

/// DDC-817
#[tokio::test]
async fn testFindOneAssociationByMagicCall() {
    let em = set_up(&CMS_MODELS).await;
    let (user_id, address_id) = load_associated_fixture(&em).await;
    let repos = em.get_repository(CMS_ADDRESS).unwrap();
    let Value::Object(address) = repos.call("findOneByUser", vec![user_id]).await.unwrap() else {
        panic!("an address");
    };

    assert!(address.instance_of(CMS_ADDRESS));
    assert_eq!(get(&address, "id"), address_id);
}

#[tokio::test]
async fn testFindByLimitOffset() {
    let em = set_up(&CMS_MODELS).await;
    load_fixture(&em).await;
    let repos = em.get_repository(CMS_USER).unwrap();

    let users1 = repos
        .find_by(&PhpArray::new(), None, Some(1), Some(0))
        .await
        .unwrap();
    let users2 = repos
        .find_by(&PhpArray::new(), None, Some(1), Some(1))
        .await
        .unwrap();

    assert_eq!(
        repos
            .find_by(&PhpArray::new(), None, None, None)
            .await
            .unwrap()
            .len(),
        4
    );
    assert_eq!(users1.len(), 1);
    assert_eq!(users2.len(), 1);
    assert!(!same(&users1[0], &users2[0]));
}

#[tokio::test]
async fn testFindByOrderBy() {
    let em = set_up(&CMS_MODELS).await;
    load_fixture(&em).await;
    let repos = em.get_repository(CMS_USER).unwrap();
    let users_asc = repos
        .find_by(
            &PhpArray::new(),
            Some(&array([("username", s("ASC"))])),
            None,
            None,
        )
        .await
        .unwrap();
    let users_desc = repos
        .find_by(
            &PhpArray::new(),
            Some(&array([("username", s("DESC"))])),
            None,
            None,
        )
        .await
        .unwrap();

    assert_eq!(
        users_asc.len(),
        4,
        "Pre-condition: only four users in fixture"
    );
    assert_eq!(
        users_desc.len(),
        4,
        "Pre-condition: only four users in fixture"
    );
    assert!(same(&users_asc[0], &users_desc[3]));
    assert!(same(&users_asc[3], &users_desc[0]));
}

#[tokio::test]
async fn testFindByOrderByAssociation() {
    let em = set_up(&CMS_MODELS).await;
    load_fixture_user_email(&em).await;

    let repository = em.get_repository(CMS_USER).unwrap();
    let result_asc = repository
        .find_by(
            &PhpArray::new(),
            Some(&array([("email", s("ASC"))])),
            None,
            None,
        )
        .await
        .unwrap();
    let result_desc = repository
        .find_by(
            &PhpArray::new(),
            Some(&array([("email", s("DESC"))])),
            None,
            None,
        )
        .await
        .unwrap();

    assert_eq!(result_asc.len(), 3);
    assert_eq!(result_desc.len(), 3);

    let email_id = |user: &Object| {
        let email = get_object(user, "email").unwrap();
        // $user->getEmail()->getId(): the proxy's identifier
        get(&email, "id")
    };
    assert_eq!(email_id(&result_asc[0]), email_id(&result_desc[2]));
    assert_eq!(email_id(&result_asc[2]), email_id(&result_desc[0]));
}

#[tokio::test]
async fn testFindFieldByMagicCallOrderBy() {
    let em = set_up(&CMS_MODELS).await;
    load_fixture(&em).await;
    let repos = em.get_repository(CMS_USER).unwrap();

    let users_asc = objects(
        repos
            .call(
                "findByStatus",
                vec![s("dev"), Value::Array(array([("username", s("ASC"))]))],
            )
            .await
            .unwrap(),
    );
    let users_desc = objects(
        repos
            .call(
                "findByStatus",
                vec![s("dev"), Value::Array(array([("username", s("DESC"))]))],
            )
            .await
            .unwrap(),
    );

    assert_eq!(users_asc.len(), 2);
    assert_eq!(users_desc.len(), 2);

    assert!(users_asc[0].instance_of(CMS_USER));
    assert_eq!(get(&users_asc[0], "name"), s("Alexander"));
    assert_eq!(get(&users_asc[0], "status"), s("dev"));

    assert!(same(&users_asc[0], &users_desc[1]));
    assert!(same(&users_asc[1], &users_desc[0]));
}

#[tokio::test]
async fn testFindFieldByMagicCallLimitOffset() {
    let em = set_up(&CMS_MODELS).await;
    load_fixture(&em).await;
    let repos = em.get_repository(CMS_USER).unwrap();

    let empty = Value::Array(PhpArray::new());
    let users1 = objects(
        repos
            .call(
                "findByStatus",
                vec![s("dev"), empty.clone(), Value::Int(1), Value::Int(0)],
            )
            .await
            .unwrap(),
    );
    let users2 = objects(
        repos
            .call(
                "findByStatus",
                vec![s("dev"), empty, Value::Int(1), Value::Int(1)],
            )
            .await
            .unwrap(),
    );

    assert_eq!(users1.len(), 1);
    assert_eq!(users2.len(), 1);
    assert!(!same(&users1[0], &users2[0]));
}

#[tokio::test]
async fn testCanRetrieveRepositoryFromClassNameWithLeadingBackslash() {
    let em = set_up(&CMS_MODELS).await;
    assert!(Arc::ptr_eq(
        &em.get_repository(&format!("\\{CMS_USER}")).unwrap(),
        &em.get_repository(CMS_USER).unwrap(),
    ));
}

#[tokio::test]
async fn testInvalidOrderByAssociation() {
    let em = set_up(&CMS_MODELS).await;
    let e = em
        .get_repository(CMS_USER)
        .unwrap()
        .find_by(
            &array([("status", s("test"))]),
            Some(&array([("address", s("ASC"))])),
            None,
            None,
        )
        .await
        .unwrap_err();
    assert_eq!(e.class, INVALID_FIND_BY_CALL);
    assert!(message(&e).contains(
        "You cannot search for the association field 'Doctrine\\Tests\\Models\\CMS\\CmsUser#address', because it is the inverse side of an association."
    ));
}

#[tokio::test]
async fn testInvalidOrientation() {
    let em = set_up(&CMS_MODELS).await;
    let repo = em.get_repository(CMS_USER).unwrap();
    let e = repo
        .find_by(
            &array([("status", s("test"))]),
            Some(&array([("username", s("INVALID"))])),
            None,
            None,
        )
        .await
        .unwrap_err();
    assert_eq!(e.class, INVALID_ORIENTATION);
    assert!(message(&e).contains(
        "Invalid order by orientation specified for Doctrine\\Tests\\Models\\CMS\\CmsUser#username"
    ));
}

#[tokio::test]
async fn testFindByAssociationArray() {
    let em = set_up(&CMS_MODELS).await;
    let mut users = Vec::new();
    for (country, zip, city, username, name) in [
        ("Germany", "12345", "Berlin", "nifnif", "Nif-Nif"),
        ("France", "54321", "Paris", "noufnouf", "Nouf-Nouf"),
        ("Spain", "98765", "Madrid", "nafnaf", "Naf-Naf"),
        ("United Kingdom", "32145", "London", "wolf", "Wolf"),
    ] {
        let address = cms_address(country, zip, city);
        let user = cms_user();
        set(&user, "username", s(username));
        set(&user, "name", s(name));
        set_address(&user, &address);
        users.push(user);
    }
    for user in &users {
        em.persist(user).unwrap();
    }
    em.flush().await.unwrap();

    let ids = list(users[..3].iter().map(|u| get(u, "id")));
    em.clear().unwrap();

    let repo = em.get_repository(CMS_ADDRESS).unwrap();
    let addresses = repo
        .find_by(&array([("user", ids)]), None, None, None)
        .await
        .unwrap();

    assert_eq!(addresses.len(), 3);
    for address in &addresses {
        assert!([s("12345"), s("54321"), s("98765")].contains(&get(address, "zip")));
    }
}

#[tokio::test]
async fn testFindByFieldInjectionPrevented() {
    let em = set_up(&CMS_MODELS).await;
    let repository = em.get_repository(CMS_USER).unwrap();
    let e = repository
        .find_by(
            &array([(
                "username = ?; DELETE FROM cms_users; SELECT 1 WHERE 1",
                s("test"),
            )]),
            None,
            None,
            None,
        )
        .await
        .unwrap_err();
    assert_eq!(e.class, UNRECOGNIZED_FIELD);
    assert!(message(&e).contains("Unrecognized field: "));
}

#[tokio::test]
async fn testFindOneByFieldInjectionPrevented() {
    let em = set_up(&CMS_MODELS).await;
    let repository = em.get_repository(CMS_USER).unwrap();
    let e = repository
        .find_one_by(
            &array([(
                "username = ?; DELETE FROM cms_users; SELECT 1 WHERE 1",
                s("test"),
            )]),
            None,
        )
        .await
        .unwrap_err();
    // UnrecognizedField is an ORMException.
    assert_eq!(e.class, UNRECOGNIZED_FIELD);
    assert!(message(&e).contains("Unrecognized field: "));
}

#[tokio::test]
async fn testFindInjectionPrevented() {
    let em = set_up(&CMS_MODELS).await;
    let repository = em.get_repository(CMS_USER).unwrap();
    let e = repository
        .find(Value::Array(array([
            (
                "username = ?; DELETE FROM cms_users; SELECT 1 WHERE 1",
                s("test"),
            ),
            ("id", Value::Int(1)),
        ])))
        .await
        .unwrap_err();
    assert_eq!(e.class, UNRECOGNIZED_IDENTIFIER_FIELDS);
    assert!(message(&e).contains("Unrecognized identifier fields: "));
}

#[tokio::test]
#[allow(clippy::similar_names, reason = "upstream's names")]
async fn testFindByNullValueInInCondition() {
    let em = set_up(&CMS_MODELS).await;
    let user1 = cms_user();
    let user2 = cms_user();

    set(&user1, "username", s("ocramius"));
    set(&user1, "name", s("Marco"));
    set(&user2, "status", Value::Null);
    set(&user2, "username", s("deeky666"));
    set(&user2, "name", s("Steve"));
    set(&user2, "status", s("dbal maintainer"));

    em.persist(&user1).unwrap();
    em.persist(&user2).unwrap();
    em.flush().await.unwrap();

    let users = em
        .get_repository(CMS_USER)
        .unwrap()
        .find_by(&array([("status", list([Value::Null]))]), None, None, None)
        .await
        .unwrap();

    assert_eq!(users.len(), 1);
    assert!(same(&user1, &users[0]));
}

// Criteria and matching()

/// `$repository->matching(Criteria::create()->where($expression))`.
fn matching(em: &Arc<EntityManager>, class: &str, criteria: &Criteria) -> Value {
    em.get_repository(class)
        .unwrap()
        .matching(criteria)
        .unwrap()
        .value()
}

fn criteria_where(
    expression: impl Into<php2rust_doctrine_collections::expr::Expression>,
) -> Criteria {
    let mut criteria = Criteria::create();
    criteria.r#where(expression);
    criteria
}

/// `count($collection)`.
async fn count(collection: &Value) -> usize {
    coll::count(collection).await.unwrap()
}

#[tokio::test]
async fn testIsNullCriteriaDoesNotGenerateAParameter() {
    let (em, log) = query_log::set_up_logged(&CMS_MODELS).await;
    log.reset_and_enable();
    let repos = em.get_repository(CMS_USER).unwrap();
    let _users = repos
        .find_by(
            &array([("status", Value::Null), ("username", s("romanb"))]),
            None,
            None,
            None,
        )
        .await
        .unwrap();

    let params = log.last_params();
    assert_eq!(params.len(), 1, "Should only execute with one parameter.");
    assert!(loose(
        &Value::Array(PhpArray::list(params.values().cloned())),
        &list([s("romanb")])
    ));
}

fn loose(a: &Value, b: &Value) -> bool {
    php_value::compare::loose_equals(a, b)
}

#[tokio::test]
async fn testIsNullCriteria() {
    let em = set_up(&CMS_MODELS).await;
    load_fixture(&em).await;

    let repos = em.get_repository(CMS_USER).unwrap();

    let users = repos
        .find_by(&array([("status", Value::Null)]), None, None, None)
        .await
        .unwrap();
    assert_eq!(users.len(), 1);
}

#[tokio::test]
async fn testMatchingEmptyCriteria() {
    let em = set_up(&CMS_MODELS).await;
    load_fixture(&em).await;

    let users = matching(&em, CMS_USER, &Criteria::create());

    assert_eq!(count(&users).await, 4);
}

#[tokio::test]
async fn testMatchingCriteriaEqComparison() {
    let em = set_up(&CMS_MODELS).await;
    load_fixture(&em).await;

    let users = matching(
        &em,
        CMS_USER,
        &criteria_where(Criteria::expr().eq("username", s("beberlei"))),
    );

    assert_eq!(count(&users).await, 1);
}

#[tokio::test]
async fn testMatchingCriteriaNeqComparison() {
    let em = set_up(&CMS_MODELS).await;
    load_fixture(&em).await;

    let users = matching(
        &em,
        CMS_USER,
        &criteria_where(Criteria::expr().neq("username", s("beberlei"))),
    );

    assert_eq!(count(&users).await, 3);
}

#[tokio::test]
async fn testMatchingCriteriaInComparison() {
    let em = set_up(&CMS_MODELS).await;
    load_fixture(&em).await;

    let users = matching(
        &em,
        CMS_USER,
        &criteria_where(
            Criteria::expr().r#in("username", PhpArray::list([s("beberlei"), s("gblanco")])),
        ),
    );

    assert_eq!(count(&users).await, 2);
}

#[tokio::test]
async fn testMatchingCriteriaNotInComparison() {
    let em = set_up(&CMS_MODELS).await;
    load_fixture(&em).await;

    let users = matching(
        &em,
        CMS_USER,
        &criteria_where(Criteria::expr().not_in(
            "username",
            PhpArray::list([s("beberlei"), s("gblanco"), s("asm89")]),
        )),
    );

    assert_eq!(count(&users).await, 1);
}

fn plus(id: &Value, n: i64) -> Value {
    match id {
        Value::Int(i) => Value::Int(i + n),
        other => panic!("an int id, got {other:?}"),
    }
}

#[tokio::test]
async fn testMatchingCriteriaLtComparison() {
    let em = set_up(&CMS_MODELS).await;
    let first_user_id = load_fixture(&em).await;

    let users = matching(
        &em,
        CMS_USER,
        &criteria_where(Criteria::expr().lt("id", plus(&first_user_id, 1))),
    );

    assert_eq!(count(&users).await, 1);
}

#[tokio::test]
async fn testMatchingCriteriaLeComparison() {
    let em = set_up(&CMS_MODELS).await;
    let first_user_id = load_fixture(&em).await;

    let users = matching(
        &em,
        CMS_USER,
        &criteria_where(Criteria::expr().lte("id", plus(&first_user_id, 1))),
    );

    assert_eq!(count(&users).await, 2);
}

#[tokio::test]
async fn testMatchingCriteriaGtComparison() {
    let em = set_up(&CMS_MODELS).await;
    let first_user_id = load_fixture(&em).await;

    let users = matching(
        &em,
        CMS_USER,
        &criteria_where(Criteria::expr().gt("id", first_user_id)),
    );

    assert_eq!(count(&users).await, 3);
}

#[tokio::test]
async fn testMatchingCriteriaGteComparison() {
    let em = set_up(&CMS_MODELS).await;
    let first_user_id = load_fixture(&em).await;

    let users = matching(
        &em,
        CMS_USER,
        &criteria_where(Criteria::expr().gte("id", first_user_id)),
    );

    assert_eq!(count(&users).await, 4);
}

#[tokio::test]
async fn testMatchingCriteriaAssocationByObjectInMemory() {
    let em = set_up(&CMS_MODELS).await;
    let (user_id, _address_id) = load_associated_fixture(&em).await;

    let user = em.find(CMS_USER, user_id).await.unwrap().unwrap();

    let criteria = criteria_where(Criteria::expr().eq("user", Value::Object(user)));

    let addresses = matching(&em, CMS_ADDRESS, &criteria);

    assert_eq!(count(&addresses).await, 1);

    let all = em
        .get_repository(CMS_ADDRESS)
        .unwrap()
        .find_all()
        .await
        .unwrap();
    let addresses =
        ArrayCollectionObject::value(PhpArray::list(all.into_iter().map(Value::Object)));

    assert_eq!(
        count(&coll::matching(&addresses, &criteria).await.unwrap()).await,
        1
    );
}

#[tokio::test]
async fn testMatchingCriteriaAssocationInWithArray() {
    let em = set_up(&CMS_MODELS).await;
    let (user_id, _address_id) = load_associated_fixture(&em).await;

    let user = em.find(CMS_USER, user_id).await.unwrap().unwrap();

    let criteria =
        criteria_where(Criteria::expr().r#in("user", PhpArray::list([Value::Object(user)])));

    let addresses = matching(&em, CMS_ADDRESS, &criteria);

    assert_eq!(count(&addresses).await, 1);

    let all = em
        .get_repository(CMS_ADDRESS)
        .unwrap()
        .find_all()
        .await
        .unwrap();
    let addresses =
        ArrayCollectionObject::value(PhpArray::list(all.into_iter().map(Value::Object)));

    assert_eq!(
        count(&coll::matching(&addresses, &criteria).await.unwrap()).await,
        1
    );
}

#[tokio::test]
async fn testMatchingCriteriaContainsComparison() {
    let em = set_up(&CMS_MODELS).await;
    load_fixture(&em).await;

    let users = matching(
        &em,
        CMS_USER,
        &criteria_where(Criteria::expr().contains("name", s("Foobar"))),
    );
    assert_eq!(count(&users).await, 0);

    let users = matching(
        &em,
        CMS_USER,
        &criteria_where(Criteria::expr().contains("name", s("Rom"))),
    );
    assert_eq!(count(&users).await, 1);

    let users = matching(
        &em,
        CMS_USER,
        &criteria_where(Criteria::expr().contains("status", s("dev"))),
    );
    assert_eq!(count(&users).await, 2);
}

#[tokio::test]
async fn testMatchingCriteriaStartsWithComparison() {
    let em = set_up(&CMS_MODELS).await;
    load_fixture(&em).await;

    let users = matching(
        &em,
        CMS_USER,
        &criteria_where(Criteria::expr().starts_with("name", s("Foo"))),
    );
    assert_eq!(count(&users).await, 0);

    let users = matching(
        &em,
        CMS_USER,
        &criteria_where(Criteria::expr().starts_with("name", s("R"))),
    );
    assert_eq!(count(&users).await, 1);

    let users = matching(
        &em,
        CMS_USER,
        &criteria_where(Criteria::expr().starts_with("status", s("de"))),
    );
    assert_eq!(count(&users).await, 2);
}

#[tokio::test]
async fn testMatchingCriteriaEndsWithComparison() {
    let em = set_up(&CMS_MODELS).await;
    load_fixture(&em).await;

    let users = matching(
        &em,
        CMS_USER,
        &criteria_where(Criteria::expr().ends_with("name", s("foo"))),
    );
    assert_eq!(count(&users).await, 0);

    let users = matching(
        &em,
        CMS_USER,
        &criteria_where(Criteria::expr().ends_with("name", s("oman"))),
    );
    assert_eq!(count(&users).await, 1);

    let users = matching(
        &em,
        CMS_USER,
        &criteria_where(Criteria::expr().ends_with("status", s("ev"))),
    );
    assert_eq!(count(&users).await, 2);
}

#[tokio::test]
async fn testMatchingCriteriaNullAssocComparison() {
    let em = set_up(&CMS_MODELS).await;
    let fixtures = load_fixture_user_email(&em).await;
    let user = em
        .find(CMS_USER, get(&fixtures[0], "id"))
        .await
        .unwrap()
        .unwrap();
    let criteria_is_null = criteria_where(Criteria::expr().is_null("email"));
    let criteria_eq_null = criteria_where(Criteria::expr().eq("email", Value::Null));

    // `$user->setEmail(null)`.
    set(&user, "email", Value::Null);
    em.persist(&user).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let users_is_null = matching(&em, CMS_USER, &criteria_is_null);
    let users_eq_null = matching(&em, CMS_USER, &criteria_eq_null);

    assert_eq!(count(&users_is_null).await, 1);
    assert_eq!(count(&users_eq_null).await, 1);

    let first_is_null = coll::get(&users_is_null, 0).await.unwrap();
    let first_eq_null = coll::get(&users_eq_null, 0).await.unwrap();
    let (Value::Object(first_is_null), Value::Object(first_eq_null)) =
        (first_is_null, first_eq_null)
    else {
        panic!("CmsUser objects expected");
    };
    assert_eq!(first_is_null.class(), CMS_USER);
    assert_eq!(first_eq_null.class(), CMS_USER);

    assert!(get(&first_is_null, "email").is_null());
    assert!(get(&first_eq_null, "email").is_null());
}

#[tokio::test]
async fn testCreateResultSetMappingBuilder() {
    let em = set_up(&CMS_MODELS).await;
    let repository = em.get_repository(CMS_USER).unwrap();
    let rsm = repository.create_result_set_mapping_builder("u").unwrap();

    let alias_map: Vec<(String, String)> = rsm
        .alias_map
        .iter()
        .map(|(k, v)| (k.to_owned(), v.clone()))
        .collect();
    assert_eq!(alias_map, vec![("u".to_owned(), CMS_USER.to_owned())]);
}

#[tokio::test]
async fn testMatchingInjectionPrevented() {
    let em = set_up(&CMS_MODELS).await;
    let repository = em.get_repository(CMS_USER).unwrap();
    let result = repository
        .matching(&criteria_where(Criteria::expr().eq(
            "username = ?; DELETE FROM cms_users; SELECT 1 WHERE 1",
            s("beberlei"),
        )))
        .unwrap();

    // Because repository returns a lazy collection, we call toArray to
    // force initialization
    let e = result.to_array().await.unwrap_err();
    assert_eq!(e.class, UNRECOGNIZED_FIELD);
    assert!(message(&e).contains("Unrecognized field: "));
}

#[tokio::test]
async fn testFindByNullValueInMultipleInCriteriaValues() {
    let em = set_up(&CMS_MODELS).await;
    let user1 = cms_user();
    let user2 = cms_user();

    set(&user1, "username", s("ocramius"));
    set(&user1, "name", s("Marco"));
    set(&user2, "status", Value::Null);
    set(&user2, "username", s("deeky666"));
    set(&user2, "name", s("Steve"));
    set(&user2, "status", s("dbal maintainer"));

    em.persist(&user1).unwrap();
    em.persist(&user2).unwrap();
    em.flush().await.unwrap();

    let users = em
        .get_repository(CMS_USER)
        .unwrap()
        .find_by(
            &array([("status", list([s("foo"), Value::Null]))]),
            None,
            None,
            None,
        )
        .await
        .unwrap();

    assert_eq!(users.len(), 1);
    assert!(same(&user1, &users[0]));
}

#[tokio::test]
async fn testFindMultipleByNullValueInMultipleInCriteriaValues() {
    let em = set_up(&CMS_MODELS).await;
    let user1 = cms_user();
    let user2 = cms_user();

    set(&user1, "username", s("ocramius"));
    set(&user1, "name", s("Marco"));
    set(&user2, "status", Value::Null);
    set(&user2, "username", s("deeky666"));
    set(&user2, "name", s("Steve"));
    set(&user2, "status", s("dbal maintainer"));

    em.persist(&user1).unwrap();
    em.persist(&user2).unwrap();
    em.flush().await.unwrap();

    let users = em
        .get_repository(CMS_USER)
        .unwrap()
        .find_by(
            &array([("status", list([s("dbal maintainer"), Value::Null]))]),
            None,
            None,
            None,
        )
        .await
        .unwrap();

    assert_eq!(users.len(), 2);

    for user in &users {
        assert!(same(user, &user1) || same(user, &user2));
    }
}

/// The configuration is the entity manager's own here, which cannot change
/// after it is created: the default repository class is set on the one an
/// entity manager is created with. `isDefaultRepository()` and
/// `isCustomRepository()` (which return `true`) are PHP methods of the
/// repository classes: the repository's class is what is compared.
#[tokio::test]
async fn testDefaultRepositoryClassName() {
    const ENTITY_REPOSITORY: &str = "Doctrine\\ORM\\EntityRepository";
    const DDC753: &str = "Doctrine\\Tests\\Models\\DDC753\\";
    let default_repository = format!("{DDC753}DDC753DefaultRepository");

    let em = orm::entity_manager(orm::connection(), None);
    assert_eq!(
        em.get_configuration().get_default_repository_class_name(),
        ENTITY_REPOSITORY
    );
    let em = entity_manager_with(orm::connection(), |config| {
        config
            .set_default_repository_class_name(&default_repository)
            .unwrap();
    });
    assert_eq!(
        em.get_configuration().get_default_repository_class_name(),
        default_repository
    );

    let repos = em
        .get_repository(&format!("{DDC753}DDC753EntityWithDefaultCustomRepository"))
        .unwrap();
    assert_eq!(repos.get_repository_class(), default_repository);

    let repos = em
        .get_repository(&format!("{DDC753}DDC753EntityWithCustomRepository"))
        .unwrap();
    assert_eq!(
        repos.get_repository_class(),
        format!("{DDC753}DDC753CustomRepository")
    );

    assert_eq!(
        em.get_configuration().get_default_repository_class_name(),
        default_repository
    );
}
