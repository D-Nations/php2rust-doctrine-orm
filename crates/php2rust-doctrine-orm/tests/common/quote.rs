//! The `quote` model set and its models' constructors as functions.
//!
//! Upstream: doctrine/orm tests/Tests/Models/Quote/User.php tests/Tests/Models/Quote/Group.php

#![allow(dead_code, reason = "each test binary uses a part of the helpers")]

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;

use crate::orm::{new_object, s, set};

pub const QUOTE_ADDRESS: &str = "Doctrine\\Tests\\Models\\Quote\\Address";
pub const QUOTE_CITY: &str = "Doctrine\\Tests\\Models\\Quote\\City";
pub const QUOTE_FULL_ADDRESS: &str = "Doctrine\\Tests\\Models\\Quote\\FullAddress";
pub const QUOTE_GROUP: &str = "Doctrine\\Tests\\Models\\Quote\\Group";
pub const QUOTE_NUMERIC_ENTITY: &str = "Doctrine\\Tests\\Models\\Quote\\NumericEntity";
pub const QUOTE_PHONE: &str = "Doctrine\\Tests\\Models\\Quote\\Phone";
pub const QUOTE_USER: &str = "Doctrine\\Tests\\Models\\Quote\\User";

/// The `quote` model set.
pub const QUOTE_MODELS: [&str; 7] = [
    QUOTE_ADDRESS,
    QUOTE_CITY,
    QUOTE_FULL_ADDRESS,
    QUOTE_GROUP,
    QUOTE_NUMERIC_ENTITY,
    QUOTE_PHONE,
    QUOTE_USER,
];

/// `new User()` with a name.
pub fn quote_user(name: &str) -> Object {
    let user = new_object(QUOTE_USER);
    set(
        &user,
        "phones",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(
        &user,
        "groups",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(&user, "name", s(name));
    user
}

/// `new Group($name, $parent)`.
pub fn quote_group(name: &str, parent: Option<&Object>) -> Object {
    let group = new_object(QUOTE_GROUP);
    set(&group, "name", s(name));
    set(
        &group,
        "parent",
        parent.map_or(Value::Null, |p| Value::Object(p.clone())),
    );
    group
}
