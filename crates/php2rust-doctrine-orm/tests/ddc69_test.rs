//! Ticket/DDC69Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! `initialize_object()` where PHP calls `getType()` on a proxy.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC69Test.php

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
#[path = "common/ticket.rs"]
mod ticket;

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};

use orm::*;
use ticket::*;

/// `new Lemma()` and `setLemma($lemma)`.
fn lemma(name: &str) -> Object {
    let lemma = new("Lemma");
    set(
        &lemma,
        "relations",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(&lemma, "lemma", s(name));
    lemma
}

/// `new RelationType()`, `setType($type)` and `setAbbreviation($abbreviation)`.
fn relation_type(type_name: &str, abbreviation: &str) -> Object {
    let relation_type = new("RelationType");
    set(&relation_type, "type", s(type_name));
    set(&relation_type, "abbreviation", s(abbreviation));
    relation_type
}

/// `new Relation()`, `setParent($parent)`, `setChild($child)` and
/// `setType($type)`.
fn relation(parent: &Object, child: &Object, relation_type: &Object) -> Object {
    let relation = new("Relation");
    set_obj(&relation, "parent", parent);
    set_obj(&relation, "child", child);
    set_obj(&relation, "type", relation_type);
    relation
}

/// `Lemma::addRelation($relation)`.
fn add_relation(lemma: &Object, relation: &Object) {
    coll::add(&get(lemma, "relations"), Value::Object(relation.clone())).unwrap();
    set_obj(relation, "parent", lemma);
}

#[tokio::test]
async fn testIssue() {
    // some of these models are created in other tests, but not all
    let em = ticket::set_up(&["RelationType", "Relation", "Lemma"]).await;

    //setup
    let lemma1 = lemma("foo");
    let lemma2 = lemma("bar");
    let lemma3 = lemma("batz");
    let lemma4 = lemma("bla");

    let type1 = relation_type("nonsense", "non");
    let type2 = relation_type("quatsch", "qu");

    let relation1 = relation(&lemma1, &lemma2, &type1);
    let relation2 = relation(&lemma1, &lemma3, &type1);
    let relation3 = relation(&lemma1, &lemma4, &type2);

    add_relation(&lemma1, &relation1);
    add_relation(&lemma1, &relation2);
    add_relation(&lemma1, &relation3);

    em.persist(&type1).unwrap();
    em.persist(&type2).unwrap();
    em.persist(&lemma1).unwrap();
    em.persist(&lemma2).unwrap();
    em.persist(&lemma3).unwrap();
    em.persist(&lemma4).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();
    //end setup

    // test One To Many
    let query = em.create_query(
        "SELECT l FROM Doctrine\\Tests\\ORM\\Functional\\Ticket\\Lemma l Where l.lemma = 'foo'",
    );
    let res = list(&query.get_result(HYDRATE_OBJECT).await.unwrap());

    let lemma = obj(&res[0]);
    assert_eq!(s("foo"), get(&lemma, "lemma"));
    assert_instance_of("Lemma", &res[0]);
    for relation in elements(&lemma, "relations").await {
        assert_instance_of("Relation", &Value::Object(relation.clone()));
        let relation_type = obj(&get(&relation, "type"));
        em.initialize_object(&relation_type).await.unwrap();
        assert_ne!(s(""), get(&relation_type, "type"));
    }

    em.clear().unwrap();
}
