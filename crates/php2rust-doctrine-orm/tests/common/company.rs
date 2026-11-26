//! The `company` model set and its models' constructors and methods as
//! functions, with the bodies of the entity listener methods the ORM calls.
//!
//! Upstream: doctrine/orm tests/Tests/Models/Company/CompanyPerson.php tests/Tests/Models/Company/CompanyEmployee.php
//! Upstream: doctrine/orm tests/Tests/Models/Company/CompanyManager.php tests/Tests/Models/Company/CompanyOrganization.php
//! Upstream: doctrine/orm tests/Tests/Models/Company/CompanyEvent.php tests/Tests/Models/Company/CompanyAuction.php
//! Upstream: doctrine/orm tests/Tests/Models/Company/CompanyRaffle.php tests/Tests/Models/Company/CompanyCar.php
//! Upstream: doctrine/orm tests/Tests/Models/Company/CompanyContract.php tests/Tests/Models/Company/CompanyFixContract.php
//! Upstream: doctrine/orm tests/Tests/Models/Company/CompanyFlexContract.php tests/Tests/Models/Company/CompanyFlexUltraContract.php
//! Upstream: doctrine/orm tests/Tests/Models/Company/CompanyContractListener.php tests/Tests/Models/Company/CompanyFlexUltraContractListener.php

#![allow(dead_code, reason = "each test binary uses a part of the helpers")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::sync::{Arc, Once};

use php_value::{Object, PhpArray, Thrown, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::object::register_listener_method;

use super::orm::{append, get, get_object, new_object, s, same, set, set_obj};

pub const COMPANY_PERSON: &str = "Doctrine\\Tests\\Models\\Company\\CompanyPerson";
pub const COMPANY_EMPLOYEE: &str = "Doctrine\\Tests\\Models\\Company\\CompanyEmployee";
pub const COMPANY_MANAGER: &str = "Doctrine\\Tests\\Models\\Company\\CompanyManager";
pub const COMPANY_ORGANIZATION: &str = "Doctrine\\Tests\\Models\\Company\\CompanyOrganization";
pub const COMPANY_EVENT: &str = "Doctrine\\Tests\\Models\\Company\\CompanyEvent";
pub const COMPANY_AUCTION: &str = "Doctrine\\Tests\\Models\\Company\\CompanyAuction";
pub const COMPANY_RAFFLE: &str = "Doctrine\\Tests\\Models\\Company\\CompanyRaffle";
pub const COMPANY_CAR: &str = "Doctrine\\Tests\\Models\\Company\\CompanyCar";
pub const COMPANY_CONTRACT: &str = "Doctrine\\Tests\\Models\\Company\\CompanyContract";
pub const COMPANY_FIX_CONTRACT: &str = "Doctrine\\Tests\\Models\\Company\\CompanyFixContract";
pub const COMPANY_FLEX_CONTRACT: &str = "Doctrine\\Tests\\Models\\Company\\CompanyFlexContract";
pub const COMPANY_FLEX_ULTRA_CONTRACT: &str =
    "Doctrine\\Tests\\Models\\Company\\CompanyFlexUltraContract";

/// The `company` model set.
pub const COMPANY_MODELS: [&str; 9] = [
    COMPANY_PERSON,
    COMPANY_EMPLOYEE,
    COMPANY_MANAGER,
    COMPANY_ORGANIZATION,
    COMPANY_EVENT,
    COMPANY_AUCTION,
    COMPANY_RAFFLE,
    COMPANY_CAR,
    COMPANY_CONTRACT,
];

/// `$this->$property[] = func_get_args();` (the event arguments are not
/// kept: a list of the entity).
fn record_call(listener: &Object, property: &str, entity: &Object) -> Result<(), Thrown> {
    let mut calls = match get(listener, property) {
        Value::Array(calls) => calls,
        _ => PhpArray::new(),
    };
    calls
        .push(Value::Array(PhpArray::list([Value::Object(
            entity.clone(),
        )])))
        .map_err(|_| Thrown::new("Error", "Cannot add element to the array", 0))?;
    set(listener, property, Value::Array(calls));
    Ok(())
}

/// The entity listener methods of the contracts.
pub fn register_listeners() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let contract_listener = "Doctrine\\Tests\\Models\\Company\\CompanyContractListener";
        for (method, property) in [
            ("postPersistHandler", "postPersistCalls"),
            ("prePersistHandler", "prePersistCalls"),
            ("postUpdateHandler", "postUpdateCalls"),
            ("preUpdateHandler", "preUpdateCalls"),
            ("postRemoveHandler", "postRemoveCalls"),
            ("preRemoveHandler", "preRemoveCalls"),
            ("preFlushHandler", "preFlushCalls"),
            ("postLoadHandler", "postLoadCalls"),
        ] {
            register_listener_method(
                contract_listener,
                method,
                Arc::new(move |listener, entity, _args| record_call(listener, property, entity)),
            );
        }
        let ultra_listener = "Doctrine\\Tests\\Models\\Company\\CompanyFlexUltraContractListener";
        for method in ["prePersistHandler1", "prePersistHandler2"] {
            register_listener_method(
                ultra_listener,
                method,
                Arc::new(|listener, entity, _args| {
                    record_call(listener, "prePersistCalls", entity)
                }),
            );
        }
    });
}

fn new_collection() -> Value {
    ArrayCollectionObject::value(PhpArray::new())
}

// Persons

/// `new CompanyPerson()` (or a subclass) with a name.
pub fn company_person_of(class: &str, name: &str) -> Object {
    let person = new_object(class);
    set(&person, "friends", new_collection());
    set(&person, "name", s(name));
    person
}

/// `new CompanyPerson()` with a name.
pub fn company_person(name: &str) -> Object {
    company_person_of(COMPANY_PERSON, name)
}

/// `new CompanyEmployee()` with a name, salary and department.
pub fn company_employee(name: &str, salary: i64, department: &str) -> Object {
    let employee = company_person_of(COMPANY_EMPLOYEE, name);
    set(&employee, "salary", Value::Int(salary));
    set(&employee, "department", s(department));
    employee
}

/// `new CompanyManager()` with a name, salary, department and title.
pub fn company_manager(name: &str, salary: i64, department: &str, title: &str) -> Object {
    let manager = company_person_of(COMPANY_MANAGER, name);
    set(&manager, "salary", Value::Int(salary));
    set(&manager, "department", s(department));
    set(&manager, "title", s(title));
    manager
}

/// `$person->setSpouse($spouse)`.
pub fn set_spouse(person: &Object, spouse: &Object) {
    if !get_object(person, "spouse").is_some_and(|current| same(&current, spouse)) {
        set_obj(person, "spouse", spouse);
        set_spouse(spouse, person);
    }
}

/// `$person->addFriend($friend)`.
pub async fn add_friend(person: &Object, friend: &Object) {
    let friends = get(person, "friends");
    if !coll::contains(&friends, &Value::Object(friend.clone()))
        .await
        .unwrap()
    {
        coll::add(&friends, Value::Object(friend.clone())).unwrap();
        Box::pin(add_friend(friend, person)).await;
    }
}

// Events

/// `new CompanyOrganization()`.
pub fn company_organization() -> Object {
    new_object(COMPANY_ORGANIZATION)
}

/// `$org->addEvent($event)`.
pub fn add_event(org: &Object, event: &Object) {
    append(org, "events", Value::Object(event.clone()));
    set_obj(event, "organization", org);
}

/// `new CompanyAuction()` / `new CompanyRaffle()` with its data.
pub fn company_event(class: &str, data: &str) -> Object {
    let event = new_object(class);
    set(&event, "data", s(data));
    event
}

// Contracts

/// `new CompanyFixContract()` (or another contract).
pub fn company_contract(class: &str) -> Object {
    let contract = new_object(class);
    set(&contract, "engineers", new_collection());
    contract
}

/// `$contract->addEngineer($engineer)`.
pub fn add_engineer(contract: &Object, engineer: &Object) {
    append(contract, "engineers", Value::Object(engineer.clone()));
}

/// `$contract->markCompleted()`.
pub fn mark_completed(contract: &Object) {
    set(contract, "completed", Value::Bool(true));
}
