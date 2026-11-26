//! OneToOneSingleTableInheritanceTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/OneToOneSingleTableInheritanceTest.php

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

use php2rust_doctrine_persistence::php_reflection;

use orm::*;

const PET: &str = "Doctrine\\Tests\\Models\\OneToOneSingleTableInheritance\\Pet";
const CAT: &str = "Doctrine\\Tests\\Models\\OneToOneSingleTableInheritance\\Cat";
const LITTER_BOX: &str = "Doctrine\\Tests\\Models\\OneToOneSingleTableInheritance\\LitterBox";

/// Tests a unidirectional one-to-one association mapping from an
/// inheritance child class
#[tokio::test]
async fn testFindFromOneToOneOwningSideJoinedTableInheritance() {
    let em = set_up(&[PET, CAT, LITTER_BOX]).await;
    let cat = new_object(CAT);
    let litter_box = new_object(LITTER_BOX);
    set_obj(&cat, "litterBox", &litter_box);

    em.persist(&cat).unwrap();
    em.persist(&litter_box).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let found_cat = em.find(PET, get(&cat, "id")).await.unwrap().unwrap();
    assert!(php_reflection::is_a(found_cat.class(), CAT));

    assert!(get(&cat, "id").identical(&get(&found_cat, "id")));
    let found_box = get_object(&found_cat, "litterBox").unwrap();
    assert!(php_reflection::is_a(found_box.class(), LITTER_BOX));
    assert!(get(&litter_box, "id").identical(&get(&found_box, "id")));
}
