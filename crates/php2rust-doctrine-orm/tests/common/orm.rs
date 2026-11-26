//! `OrmFunctionalTestCase`: an entity manager on a fresh in-memory
//! `SQLite` database with the schema of a model set, and the test models'
//! constructors and methods as functions.
//!
//! Upstream: doctrine/orm tests/Tests/OrmFunctionalTestCase.php tests/Tests/Models/CMS/CmsUser.php
//! Upstream: doctrine/orm tests/Tests/Models/CMS/CmsPhonenumber.php tests/Tests/Models/CMS/CmsAddress.php
//! Upstream: doctrine/orm tests/Tests/Models/CMS/CmsEmail.php tests/Tests/Models/CMS/CmsGroup.php
//! Upstream: doctrine/orm tests/Tests/Models/CMS/CmsTag.php tests/Tests/Models/CMS/CmsArticle.php
//! Upstream: doctrine/orm tests/Tests/Models/CMS/CmsComment.php

#![allow(dead_code, reason = "each test binary uses a part of the helpers")]

use std::sync::Arc;

use php_value::{Object, PhpArray, Thrown, Value};
use php2rust_doctrine_dbal::{Connection, DriverManager};
use php2rust_doctrine_event_manager::EventManager;
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject, downcast};
use php2rust_doctrine_orm::configuration::Configuration;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::ClassMetadata;
use php2rust_doctrine_orm::mapping::driver::AttributeDriver;
use php2rust_doctrine_orm::object::EntityObject;
use php2rust_doctrine_orm::tools::SchemaTool;

use super::models;

pub const CMS: &str = "Doctrine\\Tests\\Models\\CMS\\";
pub const CMS_USER: &str = "Doctrine\\Tests\\Models\\CMS\\CmsUser";
pub const CMS_PHONENUMBER: &str = "Doctrine\\Tests\\Models\\CMS\\CmsPhonenumber";
pub const CMS_ADDRESS: &str = "Doctrine\\Tests\\Models\\CMS\\CmsAddress";
pub const CMS_EMAIL: &str = "Doctrine\\Tests\\Models\\CMS\\CmsEmail";
pub const CMS_GROUP: &str = "Doctrine\\Tests\\Models\\CMS\\CmsGroup";
pub const CMS_TAG: &str = "Doctrine\\Tests\\Models\\CMS\\CmsTag";
pub const CMS_ARTICLE: &str = "Doctrine\\Tests\\Models\\CMS\\CmsArticle";
pub const CMS_COMMENT: &str = "Doctrine\\Tests\\Models\\CMS\\CmsComment";

/// The `cms` model set.
pub const CMS_MODELS: [&str; 8] = [
    CMS_USER,
    CMS_PHONENUMBER,
    CMS_ADDRESS,
    CMS_EMAIL,
    CMS_GROUP,
    CMS_TAG,
    CMS_ARTICLE,
    CMS_COMMENT,
];

/// `DriverManager::getConnection(['driver' => 'pdo_sqlite', 'memory' => true])`.
pub fn connection() -> Arc<Connection> {
    let mut params = PhpArray::new();
    params.insert("driver", Value::string("pdo_sqlite"));
    params.insert("memory", Value::Bool(true));
    Arc::new(DriverManager::get_connection(params, None).unwrap())
}

/// `getEntityManager()`: the attribute driver, native lazy objects.
pub fn entity_manager(conn: Arc<Connection>, event_manager: Option<EventManager>) -> Arc<EntityManager> {
    models::register();
    let mut config = Configuration::new();
    config.set_metadata_driver_impl(Arc::new(
        AttributeDriver::new(Vec::<String>::new(), true).unwrap(),
    ));
    config.enable_native_lazy_objects(true).unwrap();
    EntityManager::create(conn, config, event_manager).unwrap()
}

/// `setUp()` with `useModelSet()` for the classes: a new database with
/// their schema.
pub async fn set_up(classes: &[&str]) -> Arc<EntityManager> {
    let em = entity_manager(connection(), None);
    create_schema(&em, classes).await;
    em
}

/// `$this->_schemaTool->createSchema(...)`.
pub async fn create_schema(em: &Arc<EntityManager>, classes: &[&str]) {
    let metadata: Vec<Arc<ClassMetadata>> = classes
        .iter()
        .map(|class| em.get_class_metadata(class).unwrap())
        .collect();
    SchemaTool::new(em).create_schema(&metadata).await.unwrap();
}

/// `new $class()` without a constructor body.
pub fn new_object(class: &str) -> Object {
    models::register();
    EntityObject::object(class).unwrap_or_else(|| panic!("class {class} is not declared"))
}

/// `$object->$name`.
pub fn get(object: &Object, name: &str) -> Value {
    let entity = downcast::<EntityObject>(object).expect("an entity object");
    entity.get(name).unwrap_or(Value::Null)
}

/// `$object->$name` as an object.
pub fn get_object(object: &Object, name: &str) -> Option<Object> {
    match get(object, name) {
        Value::Object(o) => Some(o),
        _ => None,
    }
}

/// `$object->$name = $value`.
pub fn set(object: &Object, name: &str, value: impl Into<Value>) {
    let entity = downcast::<EntityObject>(object).expect("an entity object");
    assert!(entity.set(name, value.into()), "no property {name}");
}

pub fn s(text: &str) -> Value {
    Value::string(text)
}

pub fn same(a: &Object, b: &Object) -> bool {
    Arc::ptr_eq(a, b)
}

pub fn same_value(a: &Value, b: &Object) -> bool {
    matches!(a, Value::Object(a) if same(a, b))
}

/// `$object->$name[] = $value`: a collection's `add()`, or an array
/// (made from `null`) appended to.
pub fn append(object: &Object, name: &str, value: Value) {
    match get(object, name) {
        Value::Array(mut array) => {
            array.push(value).unwrap();
            set(object, name, Value::Array(array));
        }
        Value::Null => set(object, name, Value::Array(PhpArray::list([value]))),
        collection => coll::add(&collection, value).unwrap(),
    }
}

/// `$user->addTag($tag)`.
pub fn add_tag(user: &Object, tag: &Object) {
    append(user, "tags", Value::Object(tag.clone()));
    append(tag, "users", Value::Object(user.clone()));
}

// Models

/// `new CmsUser()`.
pub fn cms_user() -> Object {
    let user = new_object(CMS_USER);
    for collection in ["phonenumbers", "articles", "groups", "tags"] {
        set(&user, collection, ArrayCollectionObject::value(PhpArray::new()));
    }
    user
}

/// A `CmsUser` with a name, username and status.
pub fn cms_user_with(name: &str, username: &str, status: &str) -> Object {
    let user = cms_user();
    set(&user, "name", s(name));
    set(&user, "username", s(username));
    set(&user, "status", s(status));
    user
}

/// `$user->addPhonenumber($phone)`.
pub fn add_phonenumber(user: &Object, phone: &Object) {
    append(user, "phonenumbers", Value::Object(phone.clone()));
    set(phone, "user", Value::Object(user.clone()));
}

/// `new CmsPhonenumber()` with a number.
pub fn cms_phonenumber(number: &str) -> Object {
    let phone = new_object(CMS_PHONENUMBER);
    set(&phone, "phonenumber", s(number));
    phone
}

/// `$user->addArticle($article)`.
pub fn add_article(user: &Object, article: &Object) {
    append(user, "articles", Value::Object(article.clone()));
    set(article, "user", Value::Object(user.clone()));
}

/// `new CmsGroup()`.
pub fn cms_group(name: &str) -> Object {
    let group = new_object(CMS_GROUP);
    set(&group, "users", ArrayCollectionObject::value(PhpArray::new()));
    set(&group, "name", s(name));
    group
}

/// `$user->addGroup($group)`.
pub fn add_group(user: &Object, group: &Object) {
    append(user, "groups", Value::Object(group.clone()));
    append(group, "users", Value::Object(user.clone()));
}

/// `new CmsTag()`.
pub fn cms_tag(name: &str) -> Object {
    let tag = new_object(CMS_TAG);
    set(&tag, "name", s(name));
    tag
}

/// `new CmsAddress()`.
pub fn cms_address(country: &str, zip: &str, city: &str) -> Object {
    let address = new_object(CMS_ADDRESS);
    set(&address, "country", s(country));
    set(&address, "zip", s(zip));
    set(&address, "city", s(city));
    address
}

/// `$address->setUser($user)` / `$user->setAddress($address)`.
pub fn set_address(user: &Object, address: &Object) {
    if !same_value(&get(user, "address"), address) {
        set(user, "address", Value::Object(address.clone()));
        if !same_value(&get(address, "user"), user) {
            set(address, "user", Value::Object(user.clone()));
            set(user, "address", Value::Object(address.clone()));
        }
    }
}

/// `new CmsEmail()`.
pub fn cms_email(email: &str) -> Object {
    let e = new_object(CMS_EMAIL);
    set(&e, "email", s(email));
    e
}

/// `$user->setEmail($email)`.
pub fn set_email(user: &Object, email: &Object) {
    if !same_value(&get(user, "email"), email) {
        set(user, "email", Value::Object(email.clone()));
        set(email, "user", Value::Object(user.clone()));
    }
}

/// `new CmsArticle()`.
pub fn cms_article(topic: &str, text: &str) -> Object {
    let article = new_object(CMS_ARTICLE);
    set(&article, "topic", s(topic));
    set(&article, "text", s(text));
    article
}

/// `new CmsComment()`.
pub fn cms_comment(topic: &str, text: &str) -> Object {
    let comment = new_object(CMS_COMMENT);
    set(&comment, "topic", s(topic));
    set(&comment, "text", s(text));
    comment
}

/// `$article->addComment($comment)`.
pub fn add_comment(article: &Object, comment: &Object) {
    append(article, "comments", Value::Object(comment.clone()));
    set(comment, "article", Value::Object(article.clone()));
}

/// The value of a count query.
pub async fn fetch_one(em: &EntityManager, sql: &str) -> Value {
    em.get_connection()
        .fetch_one(sql, &PhpArray::new(), &Default::default())
        .await
        .unwrap()
        .unwrap_or(Value::Null)
}

/// The message of an error.
pub fn message(e: &Thrown) -> String {
    e.message.to_string()
}
