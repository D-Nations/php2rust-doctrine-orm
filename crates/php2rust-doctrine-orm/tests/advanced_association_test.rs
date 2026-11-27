//! Port of `AdvancedAssociationTest`, on `SQLite`. The models' constructors
//! and adders are written here, on their recorded classes.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/AdvancedAssociationTest.php

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

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;

use orm::{
    append, assert_results_are_the_same, contains, count_of, elements, get, get_object, list,
    new_object, obj, s, set,
};

const NS: &str = "Doctrine\\Tests\\ORM\\Functional\\";

fn class(short: &str) -> String {
    format!("{NS}{short}")
}

/// `new $class()` with its collections.
fn entity(short: &str, collections: &[&str]) -> Object {
    let entity = new_object(&class(short));
    for collection in collections {
        set(
            &entity,
            collection,
            ArrayCollectionObject::value(PhpArray::new()),
        );
    }
    entity
}

/// `$phrase->addDefinition($definition)`.
fn add_definition(phrase: &Object, definition: &Object) {
    append(phrase, "definitions", Value::Object(definition.clone()));
    set(definition, "phrase", Value::Object(phrase.clone()));
}

/// `$lemma->addType($type)`, which calls `$type->addLemma($lemma)`.
async fn add_type(lemma: &Object, r#type: &Object) {
    if !contains(lemma, "types", r#type).await {
        append(lemma, "types", Value::Object(r#type.clone()));
        Box::pin(add_lemma(r#type, lemma)).await;
    }
}

/// `$type->addLemma($lemma)`.
async fn add_lemma(r#type: &Object, lemma: &Object) {
    if !contains(r#type, "lemmas", lemma).await {
        append(r#type, "lemmas", Value::Object(lemma.clone()));
        Box::pin(add_type(lemma, r#type)).await;
    }
}

fn persistent_collection(object: &Object, name: &str) -> Arc<PersistentCollection> {
    PersistentCollection::from_value(&get(object, name)).expect("a persistent collection")
}

async fn set_up() -> Arc<EntityManager> {
    let classes = ["Phrase", "PhraseType", "Definition", "Lemma", "Type"].map(class);
    let classes: Vec<&str> = classes.iter().map(String::as_str).collect();
    orm::set_up(&classes).await
}

#[tokio::test]
async fn testIssue() {
    let em = set_up().await;
    //setup
    let phrase = entity("Phrase", &["definitions"]);
    set(&phrase, "phrase", s("lalala"));

    let r#type = entity("PhraseType", &["phrases"]);
    set(&r#type, "type", s("nonsense"));
    set(&r#type, "abbreviation", s("non"));

    let def1 = entity("Definition", &[]);
    set(&def1, "definition", s("def1"));
    let def2 = entity("Definition", &[]);
    set(&def2, "definition", s("def2"));

    set(&phrase, "type", Value::Object(r#type.clone()));
    add_definition(&phrase, &def1);
    add_definition(&phrase, &def2);

    em.persist(&phrase).unwrap();
    em.persist(&r#type).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();
    //end setup

    // test1 - lazy-loading many-to-one after find()
    let phrase2 = em
        .find(&class("Phrase"), get(&phrase, "id"))
        .await
        .unwrap()
        .unwrap();
    let phrase2_type = get_object(&phrase2, "type").unwrap();
    assert!(matches!(get(&phrase2_type, "id"), Value::Int(_)));

    em.clear().unwrap();

    // test2 - eager load in DQL query
    let query =
        em.create_query("SELECT p,t FROM Doctrine\\Tests\\ORM\\Functional\\Phrase p JOIN p.type t");
    let res = list(&query.get_result(HYDRATE_OBJECT).await.unwrap());
    assert_eq!(res.len(), 1);
    let res_type = get_object(&obj(&res[0]), "type").unwrap();
    assert_eq!(res_type.class(), class("PhraseType"));
    assert!(!persistent_collection(&res_type, "phrases").is_initialized());

    em.clear().unwrap();

    assert_results_are_the_same(&query).await;

    em.clear().unwrap();

    // test2 - eager load in DQL query with double-join back and forth
    let query = em.create_query(
        "SELECT p,t,pp FROM Doctrine\\Tests\\ORM\\Functional\\Phrase p JOIN p.type t JOIN t.phrases pp",
    );
    let res = list(&query.get_result(HYDRATE_OBJECT).await.unwrap());
    assert_eq!(res.len(), 1);
    let res_type = get_object(&obj(&res[0]), "type").unwrap();
    assert_eq!(res_type.class(), class("PhraseType"));
    assert!(persistent_collection(&res_type, "phrases").is_initialized());

    em.clear().unwrap();

    // test3 - lazy-loading one-to-many after find()
    let phrase3 = em
        .find(&class("Phrase"), get(&phrase, "id"))
        .await
        .unwrap()
        .unwrap();
    persistent_collection(&phrase3, "definitions");
    let definitions = elements(&phrase3, "definitions").await;
    assert_eq!(definitions[0].class(), class("Definition"));

    em.clear().unwrap();

    // test4 - lazy-loading after DQL query
    let query = em.create_query("SELECT p FROM Doctrine\\Tests\\ORM\\Functional\\Phrase p");
    let res = list(&query.get_result(HYDRATE_OBJECT).await.unwrap());
    let res0 = obj(&res[0]);
    let definitions = elements(&res0, "definitions").await;

    assert_eq!(res.len(), 1);

    assert_eq!(definitions[0].class(), class("Definition"));
    assert_eq!(count_of(&res0, "definitions").await, 2);

    em.clear().unwrap();

    assert_results_are_the_same(&query).await;
}

#[tokio::test]
async fn testManyToMany() {
    let em = set_up().await;
    let lemma = entity("Lemma", &["types"]);
    set(&lemma, "lemma", s("abu"));

    let r#type = entity("Type", &["lemmas"]);
    set(&r#type, "type", s("nonsense"));
    set(&r#type, "abbreviation", s("non"));

    add_type(&lemma, &r#type).await;

    em.persist(&lemma).unwrap();
    em.persist(&r#type).unwrap();
    em.flush().await.unwrap();

    // test5 ManyToMany
    let query = em.create_query("SELECT l FROM Doctrine\\Tests\\ORM\\Functional\\Lemma l");
    let res = list(&query.get_result(HYDRATE_OBJECT).await.unwrap());
    let types = elements(&obj(&res[0]), "types").await;

    assert_eq!(types[0].class(), class("Type"));

    em.clear().unwrap();

    assert_results_are_the_same(&query).await;
}
