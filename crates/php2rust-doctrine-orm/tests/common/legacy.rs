//! The `legacy` model set and its models' constructors and methods as
//! functions.
//!
//! Upstream: doctrine/orm tests/Tests/Models/Legacy/LegacyUser.php tests/Tests/Models/Legacy/LegacyUserReference.php
//! Upstream: doctrine/orm tests/Tests/Models/Legacy/LegacyArticle.php tests/Tests/Models/Legacy/LegacyCar.php

#![allow(dead_code, reason = "each test binary uses a part of the helpers")]

use php_value::datetime::{DateTime, DateTimeImmutable, TimeZone};
use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;

use crate::orm::{append, new_object, s, set};

pub const LEGACY_USER: &str = "Doctrine\\Tests\\Models\\Legacy\\LegacyUser";
pub const LEGACY_USER_REFERENCE: &str = "Doctrine\\Tests\\Models\\Legacy\\LegacyUserReference";
pub const LEGACY_ARTICLE: &str = "Doctrine\\Tests\\Models\\Legacy\\LegacyArticle";
pub const LEGACY_CAR: &str = "Doctrine\\Tests\\Models\\Legacy\\LegacyCar";

/// The `legacy` model set.
pub const LEGACY_MODELS: [&str; 4] = [
    LEGACY_USER,
    LEGACY_USER_REFERENCE,
    LEGACY_ARTICLE,
    LEGACY_CAR,
];

/// `new LegacyUser()` with a username and name.
pub fn legacy_user(username: &str, name: &str) -> Object {
    let user = new_object(LEGACY_USER);
    for collection in ["articles", "references", "cars"] {
        set(
            &user,
            collection,
            ArrayCollectionObject::value(PhpArray::new()),
        );
    }
    set(&user, "username", s(username));
    set(&user, "name", s(name));
    user
}

/// `new LegacyArticle()` with a topic and text.
pub fn legacy_article(topic: &str, text: &str) -> Object {
    let article = new_object(LEGACY_ARTICLE);
    set(&article, "topic", s(topic));
    set(&article, "text", s(text));
    article
}

/// `new LegacyCar()` with a description.
pub fn legacy_car(description: &str) -> Object {
    let car = new_object(LEGACY_CAR);
    set(&car, "description", s(description));
    car
}

/// `$user->addCar($car)`, which calls `$car->addUser($user)`.
pub fn add_car(user: &Object, car: &Object) {
    append(user, "cars", Value::Object(car.clone()));
    append(car, "users", Value::Object(user.clone()));
}

/// `new LegacyUserReference($source, $target, $description)`.
pub fn legacy_user_reference(source: &Object, target: &Object, description: &str) -> Object {
    let reference = new_object(LEGACY_USER_REFERENCE);
    set(&reference, "_description", s(description));
    append(source, "references", Value::Object(reference.clone()));
    append(target, "references", Value::Object(reference.clone()));
    set(&reference, "_source", Value::Object(source.clone()));
    set(&reference, "_target", Value::Object(target.clone()));
    set(
        &reference,
        "created",
        DateTime::object(DateTimeImmutable::now(TimeZone::Utc)),
    );
    reference
}
