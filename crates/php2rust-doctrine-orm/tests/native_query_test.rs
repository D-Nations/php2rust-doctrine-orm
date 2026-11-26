//! NativeQueryTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! The schema is the `cms` model set: the `company` models are only read
//! as metadata here (the joined-inheritance `CompanyPerson` tables are
//! not created, the joined-subclass persister is not ported).
//! `getSQLResultCasing()` keeps the column as given on `SQLite`.
//! `CmsUserDTO`'s constructor is registered by class name, what a
//! compiler would emit. testFluentInterface is pending: the result cache
//! (`expireResultCache()`, `setResultCacheLifetime()`) is not ported.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/NativeQueryTest.php
//! Upstream: doctrine/orm tests/Tests/DbalTypes/UpperCaseStringType.php

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

use std::sync::Arc;

use php_value::{Object, PhpArray, PhpString, Value};
use php2rust_doctrine_dbal::ArrayParameterType;
use php2rust_doctrine_dbal::platforms::AbstractPlatform;
use php2rust_doctrine_dbal::types::{
    ConversionError, Type, TypeArgument, add_type, has_type, override_type,
};
use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::internal::hydration::register_new_object_class;
use php2rust_doctrine_orm::mapping::ordered_map::OrderedMap;
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;
use php2rust_doctrine_orm::query::{ResultSetMapping, ResultSetMappingBuilder};

use orm::*;

const CMS_USER_DTO: &str = "Doctrine\\Tests\\Models\\CMS\\CmsUserDTO";
const COMPANY_CONTRACT: &str = "Doctrine\\Tests\\Models\\Company\\CompanyContract";
const COMPANY_FIX_CONTRACT: &str = "Doctrine\\Tests\\Models\\Company\\CompanyFixContract";
const CUSTOM_TYPE_UPPER_CASE: &str = "Doctrine\\Tests\\Models\\CustomType\\CustomTypeUpperCase";
const DDC3899_USER: &str = "Doctrine\\Tests\\Models\\DDC3899\\DDC3899User";
const DDC3899_FIX_CONTRACT: &str = "Doctrine\\Tests\\Models\\DDC3899\\DDC3899FixContract";

async fn set_up() -> Arc<EntityManager> {
    orm::set_up(&CMS_MODELS).await
}

/// `$this->getSQLResultCasing($this->platform, $column)` on `SQLite`.
fn casing(column: &str) -> &str {
    column
}

/// `assertSQLEquals()`: compares lowercased.
fn assert_sql_equals(expected: &str, actual: &str) {
    assert_eq!(expected.to_lowercase(), actual.to_lowercase());
}

fn no_renames() -> OrderedMap<String> {
    OrderedMap::new()
}

fn renames(pairs: &[(&str, &str)]) -> OrderedMap<String> {
    let mut map = OrderedMap::new();
    for (k, v) in pairs {
        map.insert(*k, (*v).to_owned());
    }
    map
}

fn aliases(pairs: &[(&str, &str)]) -> PhpArray {
    let mut array = PhpArray::new();
    for (k, v) in pairs {
        array.insert(*k, s(v));
    }
    array
}

fn objects(result: &Value) -> Vec<Object> {
    let Value::Array(array) = result else {
        panic!("an array result, got {result:?}");
    };
    array
        .values()
        .map(|v| match v {
            Value::Object(o) => o.clone(),
            other => panic!("an object, got {other:?}"),
        })
        .collect()
}

fn as_object(value: &Value) -> Object {
    match value {
        Value::Object(o) => o.clone(),
        other => panic!("an object, got {other:?}"),
    }
}

fn collection(object: &Object, name: &str) -> Arc<PersistentCollection> {
    PersistentCollection::from_value(&get(object, name)).expect("a persistent collection")
}

/// `self::assertEquals($expected, $actual)` on scalars.
fn assert_loose_eq(expected: &Value, actual: &Value) {
    assert!(
        php_value::compare::loose_equals(expected, actual),
        "{expected:?} != {actual:?}"
    );
}

fn roman() -> Object {
    cms_user_with("Roman", "romanb", "dev")
}

fn berlin() -> Object {
    cms_address("germany", "10827", "Berlin")
}

/// `$user->addPhonenumber($phone)`.
fn add_phone(user: &Object, number: &str) -> Object {
    let phone = cms_phonenumber(number);
    add_phonenumber(user, &phone);
    phone
}

#[tokio::test]
async fn testBasicNativeQuery() {
    let em = set_up().await;
    let user = roman();
    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    let mut rsm = ResultSetMapping::new();
    rsm.add_entity_result(CMS_USER, "u", None);
    rsm.add_field_result("u", casing("id"), "id", None);
    rsm.add_field_result("u", casing("name"), "name", None);

    let query = em
        .create_native_query("SELECT id, name FROM cms_users WHERE username = ?", rsm)
        .unwrap();
    query.set_parameter(1, s("romanb"), None);

    let users = objects(&query.get_object_result().await.unwrap());

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].class(), CMS_USER);
    assert_loose_eq(&s("Roman"), &get(&users[0], "name"));
}

#[tokio::test]
async fn testNativeQueryWithArrayParameter() {
    let em = set_up().await;
    for (name, username) in [
        ("William Shatner", "wshatner"),
        ("Leonard Nimoy", "lnimoy"),
        ("DeForest Kelly", "dkelly"),
    ] {
        em.persist(&cms_user_with(name, username, "dev")).unwrap();
    }
    em.flush().await.unwrap();

    em.clear().unwrap();

    let mut rsm = ResultSetMapping::new();
    rsm.add_entity_result(CMS_USER, "u", None);
    rsm.add_field_result("u", casing("id"), "id", None);
    rsm.add_field_result("u", casing("name"), "name", None);

    let query = em
        .create_native_query(
            "SELECT id, name FROM cms_users WHERE username IN (?) ORDER BY username",
            rsm,
        )
        .unwrap();
    query.set_parameter(
        1,
        Value::Array(PhpArray::list([s("wshatner"), s("lnimoy")])),
        Some(ArrayParameterType::String.into()),
    );

    let users = objects(&query.get_object_result().await.unwrap());

    assert_eq!(users.len(), 2);
    assert_eq!(users[0].class(), CMS_USER);
    assert_loose_eq(&s("Leonard Nimoy"), &get(&users[0], "name"));
    assert_loose_eq(&s("William Shatner"), &get(&users[1], "name"));
}

#[tokio::test]
async fn testBasicNativeQueryWithMetaResult() {
    let em = set_up().await;
    let user = roman();
    let addr = berlin();
    set_address(&user, &addr);

    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    let mut rsm = ResultSetMapping::new();
    rsm.add_entity_result(CMS_ADDRESS, "a", None);
    rsm.add_field_result("a", casing("id"), "id", None);
    rsm.add_field_result("a", casing("country"), "country", None);
    rsm.add_field_result("a", casing("zip"), "zip", None);
    rsm.add_field_result("a", casing("city"), "city", None);
    rsm.add_meta_result("a", casing("user_id"), "user_id", false, Some("integer"));

    let query = em
        .create_native_query(
            "SELECT a.id, a.country, a.zip, a.city, a.user_id FROM cms_addresses a WHERE a.id = ?",
            rsm,
        )
        .unwrap();
    query.set_parameter(1, get(&addr, "id"), None);

    let addresses = objects(&query.get_object_result().await.unwrap());

    assert_eq!(addresses.len(), 1);
    assert_eq!(addresses[0].class(), CMS_ADDRESS);
    assert_loose_eq(&get(&addr, "country"), &get(&addresses[0], "country"));
    assert_loose_eq(&get(&addr, "zip"), &get(&addresses[0], "zip"));
    assert_loose_eq(&get(&addr, "city"), &get(&addresses[0], "city"));
    assert_loose_eq(&get(&addr, "street"), &get(&addresses[0], "street"));
    assert_eq!(
        get_object(&addresses[0], "user").map(|u| u.class().to_owned()),
        Some(CMS_USER.to_owned())
    );
}

#[tokio::test]
async fn testJoinedOneToManyNativeQuery() {
    let em = set_up().await;
    let user = roman();
    add_phone(&user, "424242");

    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    let mut rsm = ResultSetMapping::new();
    rsm.add_entity_result(CMS_USER, "u", None);
    rsm.add_field_result("u", casing("id"), "id", None);
    rsm.add_field_result("u", casing("name"), "name", None);
    rsm.add_field_result("u", casing("status"), "status", None);
    rsm.add_joined_entity_result(CMS_PHONENUMBER, "p", "u", "phonenumbers", false);
    rsm.add_field_result("p", casing("phonenumber"), "phonenumber", None);

    let query = em
        .create_native_query(
            "SELECT id, name, status, phonenumber FROM cms_users INNER JOIN cms_phonenumbers ON id = user_id WHERE username = ?",
            rsm,
        )
        .unwrap();
    query.set_parameter(1, s("romanb"), None);

    let users = objects(&query.get_object_result().await.unwrap());
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].class(), CMS_USER);
    assert_loose_eq(&s("Roman"), &get(&users[0], "name"));
    let phonenumbers = collection(&users[0], "phonenumbers");
    assert!(phonenumbers.is_initialized());
    assert_eq!(
        coll::count(&get(&users[0], "phonenumbers")).await.unwrap(),
        1
    );
    let phone = as_object(&coll::get(&get(&users[0], "phonenumbers"), 0).await.unwrap());
    assert_loose_eq(&Value::Int(424_242), &get(&phone, "phonenumber"));
    assert!(same_value(&get(&phone, "user"), &users[0]));
}

#[tokio::test]
async fn testMappingAsDto() {
    let em = set_up().await;
    register_new_object_class(
        CMS_USER_DTO,
        Arc::new(|args: Vec<Value>| {
            // public function __construct(public string|null $name = null,
            // public string|null $email = null, public
            // CmsAddressDTO|string|null $address = null, public int|null
            // $phonenumbers = null)
            let dto = new_object(CMS_USER_DTO);
            for (i, name) in ["name", "email", "address", "phonenumbers"]
                .iter()
                .enumerate()
            {
                set(&dto, name, args.get(i).cloned().unwrap_or(Value::Null));
            }
            Ok(Value::Object(dto))
        }),
    );

    let user = roman();
    add_phone(&user, "424242");
    let email = cms_email("fabio.bat.silva@gmail.com");
    set_email(&user, &email);
    let addr = berlin();
    set_address(&user, &addr);

    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    let mut rsm = ResultSetMapping::new();
    rsm.add_scalar_result("name", 1, "string");
    rsm.add_scalar_result("email", 2, "string");
    rsm.add_scalar_result("city", 3, "string");
    for (column, arg_index) in [("name", 0), ("email", 1), ("city", 2)] {
        let mut mapping = PhpArray::new();
        mapping.insert("className", s(CMS_USER_DTO));
        mapping.insert("objIndex", Value::Int(0));
        mapping.insert("argIndex", Value::Int(arg_index));
        rsm.new_object_mappings
            .insert(column, Value::Array(mapping));
    }
    let query = em
        .create_native_query(
            "    SELECT u.name, e.email, a.city
      FROM cms_users u
INNER JOIN cms_phonenumbers p ON u.id = p.user_id
INNER JOIN cms_emails e ON e.id = u.email_id
INNER JOIN cms_addresses a ON u.id = a.user_id
     WHERE username = ?",
            rsm,
        )
        .unwrap();
    query.set_parameter(1, s("romanb"), None);

    let users = objects(&query.get_object_result().await.unwrap());
    assert_eq!(users.len(), 1);
    let user = &users[0];
    assert_eq!(user.class(), CMS_USER_DTO);
    assert_loose_eq(&s("Roman"), &get(user, "name"));
    assert_loose_eq(&s("fabio.bat.silva@gmail.com"), &get(user, "email"));
    assert_loose_eq(&s("Berlin"), &get(user, "address"));
}

/// The assertions on the user with its joined address.
async fn assert_user_with_address(em: &Arc<EntityManager>, users: &[Object]) {
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].class(), CMS_USER);
    assert_loose_eq(&s("Roman"), &get(&users[0], "name"));
    assert!(!collection(&users[0], "phonenumbers").is_initialized());
    let address = get_object(&users[0], "address").expect("an address");
    assert_eq!(address.class(), CMS_ADDRESS);
    em.initialize_object(&address).await.unwrap();
    assert!(same_value(&get(&address, "user"), &users[0]));
    assert_loose_eq(&s("germany"), &get(&address, "country"));
    assert_loose_eq(&Value::Int(10827), &get(&address, "zip"));
    assert_loose_eq(&s("Berlin"), &get(&address, "city"));
}

#[tokio::test]
async fn testJoinedOneToOneNativeQuery() {
    let em = set_up().await;
    let user = roman();
    let addr = berlin();
    set_address(&user, &addr);

    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    let mut rsm = ResultSetMapping::new();
    rsm.add_entity_result(CMS_USER, "u", None);
    rsm.add_field_result("u", casing("id"), "id", None);
    rsm.add_field_result("u", casing("name"), "name", None);
    rsm.add_field_result("u", casing("status"), "status", None);
    rsm.add_joined_entity_result(CMS_ADDRESS, "a", "u", "address", false);
    rsm.add_field_result("a", casing("a_id"), "id", None);
    rsm.add_field_result("a", casing("country"), "country", None);
    rsm.add_field_result("a", casing("zip"), "zip", None);
    rsm.add_field_result("a", casing("city"), "city", None);

    let query = em
        .create_native_query(
            "SELECT u.id, u.name, u.status, a.id AS a_id, a.country, a.zip, a.city FROM cms_users u INNER JOIN cms_addresses a ON u.id = a.user_id WHERE u.username = ?",
            rsm,
        )
        .unwrap();
    query.set_parameter(1, s("romanb"), None);

    let users = objects(&query.get_object_result().await.unwrap());

    assert_user_with_address(&em, &users).await;
}

#[tokio::test]
async fn testJoinedOneToManyNativeQueryWithRSMBuilder() {
    let em = set_up().await;
    let user = roman();
    let phone = add_phone(&user, "424242");

    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    let mut rsm = ResultSetMappingBuilder::new(&em, None);
    rsm.add_root_entity_from_class_metadata(CMS_USER, "u", &no_renames(), None)
        .unwrap();
    rsm.add_joined_entity_from_class_metadata(
        CMS_PHONENUMBER,
        "p",
        "u",
        "phonenumbers",
        &no_renames(),
        None,
    )
    .unwrap();
    let query = em
        .create_native_query(
            "SELECT u.*, p.* FROM cms_users u LEFT JOIN cms_phonenumbers p ON u.id = p.user_id WHERE username = ?",
            rsm.into_result_set_mapping(),
        )
        .unwrap();
    query.set_parameter(1, s("romanb"), None);

    let users = objects(&query.get_object_result().await.unwrap());
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].class(), CMS_USER);
    assert_loose_eq(&s("Roman"), &get(&users[0], "name"));
    assert!(collection(&users[0], "phonenumbers").is_initialized());
    assert_eq!(
        coll::count(&get(&users[0], "phonenumbers")).await.unwrap(),
        1
    );
    let phones = get(&users[0], "phonenumbers");
    let first = as_object(&coll::get(&phones, 0).await.unwrap());
    assert_loose_eq(&Value::Int(424_242), &get(&first, "phonenumber"));
    assert!(same_value(&get(&first, "user"), &users[0]));

    em.clear().unwrap();

    let mut rsm = ResultSetMappingBuilder::new(&em, None);
    rsm.add_root_entity_from_class_metadata(CMS_PHONENUMBER, "p", &no_renames(), None)
        .unwrap();
    let query = em
        .create_native_query(
            "SELECT p.* FROM cms_phonenumbers p WHERE p.phonenumber = ?",
            rsm.into_result_set_mapping(),
        )
        .unwrap();
    query.set_parameter(1, get(&phone, "phonenumber"), None);
    let phone = as_object(&query.get_single_result(None).await.unwrap());

    let phone_user = get_object(&phone, "user").expect("a user");
    em.initialize_object(&phone_user).await.unwrap();
    assert_loose_eq(&get(&user, "name"), &get(&phone_user, "name"));
}

#[tokio::test]
async fn testJoinedOneToOneNativeQueryWithRSMBuilder() {
    let em = set_up().await;
    let user = roman();
    let addr = berlin();
    set_address(&user, &addr);

    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    let mut rsm = ResultSetMappingBuilder::new(&em, None);
    rsm.add_root_entity_from_class_metadata(CMS_USER, "u", &no_renames(), None)
        .unwrap();
    rsm.add_joined_entity_from_class_metadata(
        CMS_ADDRESS,
        "a",
        "u",
        "address",
        &renames(&[("id", "a_id")]),
        None,
    )
    .unwrap();

    let query = em
        .create_native_query(
            "SELECT u.*, a.*, a.id AS a_id FROM cms_users u INNER JOIN cms_addresses a ON u.id = a.user_id WHERE u.username = ?",
            rsm.into_result_set_mapping(),
        )
        .unwrap();
    query.set_parameter(1, s("romanb"), None);

    let users = objects(&query.get_object_result().await.unwrap());

    assert_user_with_address(&em, &users).await;

    em.clear().unwrap();

    let mut rsm = ResultSetMappingBuilder::new(&em, None);
    rsm.add_root_entity_from_class_metadata(CMS_ADDRESS, "a", &no_renames(), None)
        .unwrap();
    let query = em
        .create_native_query(
            "SELECT a.* FROM cms_addresses a WHERE a.id = ?",
            rsm.into_result_set_mapping(),
        )
        .unwrap();
    query.set_parameter(1, get(&addr, "id"), None);
    let address = as_object(&query.get_single_result(None).await.unwrap());

    let address_user = get_object(&address, "user").expect("a user");
    em.initialize_object(&address_user).await.unwrap();
    assert_loose_eq(&get(&user, "name"), &get(&address_user, "name"));
}

#[tokio::test]
async fn testConcreteClassInSingleTableInheritanceSchemaWithRSMBuilderIsFine() {
    let em = set_up().await;
    let mut rsm = ResultSetMappingBuilder::new(&em, None);
    rsm.add_root_entity_from_class_metadata(COMPANY_FIX_CONTRACT, "c", &no_renames(), None)
        .unwrap();

    assert_eq!(rsm.get_class_name("c"), Some(COMPANY_FIX_CONTRACT));
}

#[tokio::test]
async fn testAbstractClassInSingleTableInheritanceSchemaWithRSMBuilderThrowsException() {
    let em = set_up().await;
    let mut rsm = ResultSetMappingBuilder::new(&em, None);
    let e = rsm
        .add_root_entity_from_class_metadata(COMPANY_CONTRACT, "c", &no_renames(), None)
        .unwrap_err();

    assert_eq!(e.class, "InvalidArgumentException");
    assert_eq!(
        message(&e),
        "ResultSetMapping builder does not currently support your inheritance scheme."
    );
}

#[tokio::test]
async fn testRSMBuilderThrowsExceptionOnColumnConflict() {
    let em = set_up().await;
    let mut rsm = ResultSetMappingBuilder::new(&em, None);
    rsm.add_root_entity_from_class_metadata(CMS_USER, "u", &no_renames(), None)
        .unwrap();
    let e = rsm
        .add_joined_entity_from_class_metadata(
            CMS_ADDRESS,
            "a",
            "u",
            "address",
            &no_renames(),
            None,
        )
        .unwrap_err();

    assert_eq!(e.class, "InvalidArgumentException");
}

#[tokio::test]
async fn testUnknownParentAliasThrowsException() {
    let em = set_up().await;
    let mut rsm = ResultSetMappingBuilder::new(&em, None);
    rsm.add_root_entity_from_class_metadata(CMS_USER, "u", &no_renames(), None)
        .unwrap();
    rsm.add_joined_entity_from_class_metadata(
        CMS_ADDRESS,
        "a",
        "un",
        "address",
        &renames(&[("id", "a_id")]),
        None,
    )
    .unwrap();

    let query = em
        .create_native_query(
            "SELECT u.*, a.*, a.id AS a_id FROM cms_users u INNER JOIN cms_addresses a ON u.id = a.user_id WHERE u.username = ?",
            rsm.into_result_set_mapping(),
        )
        .unwrap();
    query.set_parameter(1, s("romanb"), None);

    let e = query.get_object_result().await.unwrap_err();
    assert_eq!(
        e.class,
        "Doctrine\\ORM\\Internal\\Hydration\\HydrationException"
    );
    assert_eq!(
        message(&e),
        "The parent object of entity result with alias 'a' was not found. The parent alias is 'un'."
    );
}

#[tokio::test]
async fn testGenerateSelectClauseNoRenameSingleEntity() {
    let em = set_up().await;
    let mut rsm = ResultSetMappingBuilder::new(&em, None);
    rsm.add_root_entity_from_class_metadata(CMS_USER, "u", &no_renames(), None)
        .unwrap();

    let select_clause = rsm.generate_select_clause(&PhpArray::new()).unwrap();

    assert_sql_equals(
        "u.id AS id, u.status AS status, u.username AS username, u.name AS name, u.email_id AS email_id",
        &select_clause,
    );
}

#[tokio::test]
async fn testGenerateSelectClauseCustomRenames() {
    let em = set_up().await;
    let mut rsm = ResultSetMappingBuilder::new(&em, None);
    rsm.add_root_entity_from_class_metadata(
        CMS_USER,
        "u",
        &renames(&[("id", "id1"), ("username", "username2")]),
        None,
    )
    .unwrap();

    let select_clause = rsm.generate_select_clause(&PhpArray::new()).unwrap();

    assert_sql_equals(
        "u.id AS id1, u.status AS status, u.username AS username2, u.name AS name, u.email_id AS email_id",
        &select_clause,
    );
}

#[tokio::test]
async fn testGenerateSelectClauseRenameTableAlias() {
    let em = set_up().await;
    let mut rsm = ResultSetMappingBuilder::new(&em, None);
    rsm.add_root_entity_from_class_metadata(CMS_USER, "u", &no_renames(), None)
        .unwrap();

    let select_clause = rsm
        .generate_select_clause(&aliases(&[("u", "u1")]))
        .unwrap();

    assert_sql_equals(
        "u1.id AS id, u1.status AS status, u1.username AS username, u1.name AS name, u1.email_id AS email_id",
        &select_clause,
    );
}

#[tokio::test]
async fn testGenerateSelectClauseIncrement() {
    let em = set_up().await;
    let mut rsm = ResultSetMappingBuilder::new(
        &em,
        Some(ResultSetMappingBuilder::COLUMN_RENAMING_INCREMENT),
    );
    rsm.add_root_entity_from_class_metadata(CMS_USER, "u", &no_renames(), None)
        .unwrap();

    let select_clause = rsm.generate_select_clause(&PhpArray::new()).unwrap();

    assert_sql_equals(
        "u.id AS id0, u.status AS status1, u.username AS username2, u.name AS name3, u.email_id AS email_id4",
        &select_clause,
    );
}

#[tokio::test]
async fn testGenerateSelectClauseToString() {
    let em = set_up().await;
    let mut rsm = ResultSetMappingBuilder::new(
        &em,
        Some(ResultSetMappingBuilder::COLUMN_RENAMING_INCREMENT),
    );
    rsm.add_root_entity_from_class_metadata(CMS_USER, "u", &no_renames(), None)
        .unwrap();

    assert_sql_equals(
        "u.id AS id0, u.status AS status1, u.username AS username2, u.name AS name3, u.email_id AS email_id4",
        &rsm.to_string().unwrap(),
    );
}

#[tokio::test]
async fn testGenerateSelectClauseWithDiscriminatorColumn() {
    let em = set_up().await;
    let mut rsm = ResultSetMappingBuilder::new(
        &em,
        Some(ResultSetMappingBuilder::COLUMN_RENAMING_INCREMENT),
    );
    rsm.add_entity_result(DDC3899_USER, "u", None);
    rsm.add_joined_entity_result(DDC3899_FIX_CONTRACT, "c", "u", "contracts", false);
    rsm.add_field_result("u", casing("id"), "id", None);
    rsm.set_discriminator_column("c", casing("discr"));

    let select_clause = rsm
        .generate_select_clause(&aliases(&[("u", "u1"), ("c", "c1")]))
        .unwrap();

    assert_sql_equals("u1.id as id, c1.discr as discr", &select_clause);
}

/// `Doctrine\Tests\DbalTypes\UpperCaseStringType`.
#[derive(Debug)]
struct UpperCaseStringType;

impl Type for UpperCaseStringType {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\DbalTypes\\UpperCaseStringType"
    }

    fn get_sql_declaration(
        &self,
        column: &PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<PhpString, php2rust_doctrine_dbal::Exception> {
        platform.get_string_type_declaration_sql(column)
    }

    fn convert_to_database_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        Ok(value.clone())
    }

    fn convert_to_php_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        Ok(value.clone())
    }

    fn convert_to_database_value_sql(
        &self,
        sql_expr: &[u8],
        _platform: &dyn AbstractPlatform,
    ) -> PhpString {
        let mut out = b"UPPER(".to_vec();
        out.extend_from_slice(sql_expr);
        out.push(b')');
        PhpString::from(out)
    }

    fn convert_to_php_value_sql(
        &self,
        sql_expr: &[u8],
        _platform: &dyn AbstractPlatform,
    ) -> PhpString {
        let mut out = b"LOWER(".to_vec();
        out.extend_from_slice(sql_expr);
        out.push(b')');
        PhpString::from(out)
    }
}

/// `DBALType::addType('upper_case_string', ...)` (or `overrideType()`).
fn register_upper_case_string_type() {
    let type_ = || TypeArgument::Instance(Arc::new(UpperCaseStringType));
    if has_type("upper_case_string") {
        override_type("upper_case_string", type_()).unwrap();
    } else if add_type("upper_case_string", type_()).is_err() {
        // Another test thread may have added it meanwhile.
        override_type("upper_case_string", type_()).unwrap();
    }
}

#[tokio::test]
async fn testGenerateSelectClauseWithCustomTypeUsingEntityFromClassMetadata() {
    let em = set_up().await;
    register_upper_case_string_type();

    let mut rsm = ResultSetMappingBuilder::new(
        &em,
        Some(ResultSetMappingBuilder::COLUMN_RENAMING_INCREMENT),
    );

    rsm.add_root_entity_from_class_metadata(CUSTOM_TYPE_UPPER_CASE, "ct", &no_renames(), None)
        .unwrap();

    let select_clause = rsm
        .generate_select_clause(&aliases(&[("ct", "ct1")]))
        .unwrap();

    assert_sql_equals(
        "ct1.id as id0, lower(ct1.lowercasestring) as lowercasestring1, lower(ct1.named_lower_case_string) as named_lower_case_string2",
        &select_clause,
    );
}

#[tokio::test]
async fn testGenerateSelectClauseWithCustomTypeUsingAddFieldResult() {
    let em = set_up().await;
    register_upper_case_string_type();

    let mut rsm = ResultSetMappingBuilder::new(
        &em,
        Some(ResultSetMappingBuilder::COLUMN_RENAMING_INCREMENT),
    );
    rsm.add_entity_result(CUSTOM_TYPE_UPPER_CASE, "ct", None);
    rsm.add_field_result("ct", casing("lowercasestring"), "lowerCaseString", None);

    let select_clause = rsm
        .generate_select_clause(&aliases(&[("ct", "ct1")]))
        .unwrap();

    assert_sql_equals(
        "lower(ct1.lowercasestring) as lowercasestring",
        &select_clause,
    );
}
