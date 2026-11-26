//! Upstream: doctrine/orm tests/Tests/ORM/Hydration/ArrayHydratorTest.php
//!
//! Translated line by line over the PHP value helpers of
//! `common/php_test.rs`; a data provider's cases run in a loop.

#![allow(non_snake_case)]
#![allow(
    unused_mut,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::redundant_clone,
    clippy::similar_names,
    clippy::too_many_lines,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;
#[path = "common/php_test.rs"]
mod php_test;

use php_value::Value;
use php2rust_doctrine_orm::abstract_query::HYDRATE_ARRAY;
use php2rust_doctrine_orm::query::ResultSetMapping;

use php_test::*;

fn entity_manager() -> std::sync::Arc<php2rust_doctrine_orm::entity_manager::EntityManager> {
    orm::entity_manager(orm::connection(), None)
}

#[tokio::test]
async fn testSimpleEntityQuery() {
    let em = entity_manager();
    let mut rsm = ResultSetMapping::new();
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[s("Doctrine\\Tests\\Models\\CMS\\CmsUser"), s("u")],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("u"), s("u__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("u"), s("u__name"), s("name")],
    );
    let mut result_set = list_of(vec![
        array_of(vec![(s("u__id"), s("1")), (s("u__name"), s("romanb"))]),
        array_of(vec![(s("u__id"), s("2")), (s("u__name"), s("jwage"))]),
    ]);
    let result = php_hydrate_all(&em, HYDRATE_ARRAY, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_count(&Value::Int(2), &result.clone());
    assert_is_array(&result.clone());
    assert_equals(
        &Value::Int(1),
        &idx(&idx(&result.clone(), &Value::Int(0)), &s("id")),
    );
    assert_equals(
        &s("romanb"),
        &idx(&idx(&result.clone(), &Value::Int(0)), &s("name")),
    );
    assert_equals(
        &Value::Int(2),
        &idx(&idx(&result.clone(), &Value::Int(1)), &s("id")),
    );
    assert_equals(
        &s("jwage"),
        &idx(&idx(&result.clone(), &Value::Int(1)), &s("name")),
    );
}

#[tokio::test]
async fn testSimpleEntityWithScalarQuery() {
    let em = entity_manager();
    for user_entity_key in [Value::Int(0), s("user"), s("scalars"), s("newObjects")] {
        let mut alias = elvis(user_entity_key.clone(), s("u"));
        let mut rsm = ResultSetMapping::new();
        rsm_call(
            &mut rsm,
            "addEntityResult",
            &[s("Doctrine\\Tests\\Models\\CMS\\CmsUser"), alias.clone()],
        );
        rsm_call(
            &mut rsm,
            "addFieldResult",
            &[alias.clone(), s("s__id"), s("id")],
        );
        rsm_call(
            &mut rsm,
            "addFieldResult",
            &[alias.clone(), s("s__name"), s("name")],
        );
        rsm_call(
            &mut rsm,
            "addScalarResult",
            &[s("sclr0"), s("nameUpper"), s("string")],
        );
        let mut result_set = list_of(vec![
            array_of(vec![
                (s("s__id"), s("1")),
                (s("s__name"), s("romanb")),
                (s("sclr0"), s("ROMANB")),
            ]),
            array_of(vec![
                (s("s__id"), s("2")),
                (s("s__name"), s("jwage")),
                (s("sclr0"), s("JWAGE")),
            ]),
        ]);
        let result = php_hydrate_all(&em, HYDRATE_ARRAY, &result_set, &rsm, &Value::Null)
            .await
            .unwrap();
        assert_count(&Value::Int(2), &result.clone());
        assert_is_array(&result.clone());
        assert_array_has_key(&s("nameUpper"), &idx(&result.clone(), &Value::Int(0)));
        assert_array_not_has_key(&s("id"), &idx(&result.clone(), &Value::Int(0)));
        assert_array_not_has_key(&s("name"), &idx(&result.clone(), &Value::Int(0)));
        assert_array_has_key(&Value::Int(0), &idx(&result.clone(), &Value::Int(0)));
        assert_array_has_key(
            &s("id"),
            &idx(&idx(&result.clone(), &Value::Int(0)), &Value::Int(0)),
        );
        assert_array_has_key(
            &s("name"),
            &idx(&idx(&result.clone(), &Value::Int(0)), &Value::Int(0)),
        );
        assert_array_has_key(&s("nameUpper"), &idx(&result.clone(), &Value::Int(1)));
        assert_array_not_has_key(&s("id"), &idx(&result.clone(), &Value::Int(1)));
        assert_array_not_has_key(&s("name"), &idx(&result.clone(), &Value::Int(1)));
        assert_array_has_key(&Value::Int(0), &idx(&result.clone(), &Value::Int(1)));
        assert_array_has_key(
            &s("id"),
            &idx(&idx(&result.clone(), &Value::Int(1)), &Value::Int(0)),
        );
        assert_array_has_key(
            &s("name"),
            &idx(&idx(&result.clone(), &Value::Int(1)), &Value::Int(0)),
        );
    }
}

#[tokio::test]
async fn testSimpleEntityQueryWithAliasedUserEntity() {
    let em = entity_manager();
    let mut rsm = ResultSetMapping::new();
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[
            s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
            s("u"),
            s("user"),
        ],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("u"), s("u__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("u"), s("u__name"), s("name")],
    );
    let mut result_set = list_of(vec![
        array_of(vec![(s("u__id"), s("1")), (s("u__name"), s("romanb"))]),
        array_of(vec![(s("u__id"), s("2")), (s("u__name"), s("jwage"))]),
    ]);
    let result = php_hydrate_all(&em, HYDRATE_ARRAY, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_count(&Value::Int(2), &result.clone());
    assert_is_array(&result.clone());
    assert_array_has_key(&s("user"), &idx(&result.clone(), &Value::Int(0)));
    assert_equals(
        &Value::Int(1),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(0)), &s("user")),
            &s("id"),
        ),
    );
    assert_equals(
        &s("romanb"),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(0)), &s("user")),
            &s("name"),
        ),
    );
    assert_array_has_key(&s("user"), &idx(&result.clone(), &Value::Int(1)));
    assert_equals(
        &Value::Int(2),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(1)), &s("user")),
            &s("id"),
        ),
    );
    assert_equals(
        &s("jwage"),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(1)), &s("user")),
            &s("name"),
        ),
    );
}

#[tokio::test]
async fn testSimpleMultipleRootEntityQuery() {
    let em = entity_manager();
    let mut rsm = ResultSetMapping::new();
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[s("Doctrine\\Tests\\Models\\CMS\\CmsUser"), s("u")],
    );
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[s("Doctrine\\Tests\\Models\\CMS\\CmsArticle"), s("a")],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("u"), s("u__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("u"), s("u__name"), s("name")],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("a"), s("a__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("a"), s("a__topic"), s("topic")],
    );
    let mut result_set = list_of(vec![
        array_of(vec![
            (s("u__id"), s("1")),
            (s("u__name"), s("romanb")),
            (s("a__id"), s("1")),
            (s("a__topic"), s("Cool things.")),
        ]),
        array_of(vec![
            (s("u__id"), s("2")),
            (s("u__name"), s("jwage")),
            (s("a__id"), s("2")),
            (s("a__topic"), s("Cool things II.")),
        ]),
    ]);
    let result = php_hydrate_all(&em, HYDRATE_ARRAY, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_count(&Value::Int(4), &result.clone());
    assert_equals(
        &Value::Int(1),
        &idx(&idx(&result.clone(), &Value::Int(0)), &s("id")),
    );
    assert_equals(
        &s("romanb"),
        &idx(&idx(&result.clone(), &Value::Int(0)), &s("name")),
    );
    assert_equals(
        &Value::Int(1),
        &idx(&idx(&result.clone(), &Value::Int(1)), &s("id")),
    );
    assert_equals(
        &s("Cool things."),
        &idx(&idx(&result.clone(), &Value::Int(1)), &s("topic")),
    );
    assert_equals(
        &Value::Int(2),
        &idx(&idx(&result.clone(), &Value::Int(2)), &s("id")),
    );
    assert_equals(
        &s("jwage"),
        &idx(&idx(&result.clone(), &Value::Int(2)), &s("name")),
    );
    assert_equals(
        &Value::Int(2),
        &idx(&idx(&result.clone(), &Value::Int(3)), &s("id")),
    );
    assert_equals(
        &s("Cool things II."),
        &idx(&idx(&result.clone(), &Value::Int(3)), &s("topic")),
    );
}

#[tokio::test]
async fn testSimpleMultipleRootEntityQueryWithAliasedUserEntity() {
    let em = entity_manager();
    let mut rsm = ResultSetMapping::new();
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[
            s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
            s("u"),
            s("user"),
        ],
    );
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[s("Doctrine\\Tests\\Models\\CMS\\CmsArticle"), s("a")],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("u"), s("u__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("u"), s("u__name"), s("name")],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("a"), s("a__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("a"), s("a__topic"), s("topic")],
    );
    let mut result_set = list_of(vec![
        array_of(vec![
            (s("u__id"), s("1")),
            (s("u__name"), s("romanb")),
            (s("a__id"), s("1")),
            (s("a__topic"), s("Cool things.")),
        ]),
        array_of(vec![
            (s("u__id"), s("2")),
            (s("u__name"), s("jwage")),
            (s("a__id"), s("2")),
            (s("a__topic"), s("Cool things II.")),
        ]),
    ]);
    let result = php_hydrate_all(&em, HYDRATE_ARRAY, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_count(&Value::Int(4), &result.clone());
    assert_array_has_key(&s("user"), &idx(&result.clone(), &Value::Int(0)));
    assert_equals(
        &Value::Int(1),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(0)), &s("user")),
            &s("id"),
        ),
    );
    assert_equals(
        &s("romanb"),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(0)), &s("user")),
            &s("name"),
        ),
    );
    assert_array_has_key(&Value::Int(0), &idx(&result.clone(), &Value::Int(1)));
    assert_equals(
        &Value::Int(1),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(1)), &Value::Int(0)),
            &s("id"),
        ),
    );
    assert_equals(
        &s("Cool things."),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(1)), &Value::Int(0)),
            &s("topic"),
        ),
    );
    assert_array_has_key(&s("user"), &idx(&result.clone(), &Value::Int(2)));
    assert_equals(
        &Value::Int(2),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(2)), &s("user")),
            &s("id"),
        ),
    );
    assert_equals(
        &s("jwage"),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(2)), &s("user")),
            &s("name"),
        ),
    );
    assert_array_has_key(&Value::Int(0), &idx(&result.clone(), &Value::Int(3)));
    assert_equals(
        &Value::Int(2),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(3)), &Value::Int(0)),
            &s("id"),
        ),
    );
    assert_equals(
        &s("Cool things II."),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(3)), &Value::Int(0)),
            &s("topic"),
        ),
    );
}

#[tokio::test]
async fn testSimpleMultipleRootEntityQueryWithAliasedArticleEntity() {
    let em = entity_manager();
    let mut rsm = ResultSetMapping::new();
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[s("Doctrine\\Tests\\Models\\CMS\\CmsUser"), s("u")],
    );
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[
            s("Doctrine\\Tests\\Models\\CMS\\CmsArticle"),
            s("a"),
            s("article"),
        ],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("u"), s("u__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("u"), s("u__name"), s("name")],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("a"), s("a__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("a"), s("a__topic"), s("topic")],
    );
    let mut result_set = list_of(vec![
        array_of(vec![
            (s("u__id"), s("1")),
            (s("u__name"), s("romanb")),
            (s("a__id"), s("1")),
            (s("a__topic"), s("Cool things.")),
        ]),
        array_of(vec![
            (s("u__id"), s("2")),
            (s("u__name"), s("jwage")),
            (s("a__id"), s("2")),
            (s("a__topic"), s("Cool things II.")),
        ]),
    ]);
    let result = php_hydrate_all(&em, HYDRATE_ARRAY, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_count(&Value::Int(4), &result.clone());
    assert_array_has_key(&Value::Int(0), &idx(&result.clone(), &Value::Int(0)));
    assert_equals(
        &Value::Int(1),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(0)), &Value::Int(0)),
            &s("id"),
        ),
    );
    assert_equals(
        &s("romanb"),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(0)), &Value::Int(0)),
            &s("name"),
        ),
    );
    assert_array_has_key(&s("article"), &idx(&result.clone(), &Value::Int(1)));
    assert_equals(
        &Value::Int(1),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(1)), &s("article")),
            &s("id"),
        ),
    );
    assert_equals(
        &s("Cool things."),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(1)), &s("article")),
            &s("topic"),
        ),
    );
    assert_array_has_key(&Value::Int(0), &idx(&result.clone(), &Value::Int(2)));
    assert_equals(
        &Value::Int(2),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(2)), &Value::Int(0)),
            &s("id"),
        ),
    );
    assert_equals(
        &s("jwage"),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(2)), &Value::Int(0)),
            &s("name"),
        ),
    );
    assert_array_has_key(&s("article"), &idx(&result.clone(), &Value::Int(3)));
    assert_equals(
        &Value::Int(2),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(3)), &s("article")),
            &s("id"),
        ),
    );
    assert_equals(
        &s("Cool things II."),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(3)), &s("article")),
            &s("topic"),
        ),
    );
}

#[tokio::test]
async fn testSimpleMultipleRootEntityQueryWithAliasedEntities() {
    let em = entity_manager();
    let mut rsm = ResultSetMapping::new();
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[
            s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
            s("u"),
            s("user"),
        ],
    );
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[
            s("Doctrine\\Tests\\Models\\CMS\\CmsArticle"),
            s("a"),
            s("article"),
        ],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("u"), s("u__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("u"), s("u__name"), s("name")],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("a"), s("a__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("a"), s("a__topic"), s("topic")],
    );
    let mut result_set = list_of(vec![
        array_of(vec![
            (s("u__id"), s("1")),
            (s("u__name"), s("romanb")),
            (s("a__id"), s("1")),
            (s("a__topic"), s("Cool things.")),
        ]),
        array_of(vec![
            (s("u__id"), s("2")),
            (s("u__name"), s("jwage")),
            (s("a__id"), s("2")),
            (s("a__topic"), s("Cool things II.")),
        ]),
    ]);
    let result = php_hydrate_all(&em, HYDRATE_ARRAY, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_count(&Value::Int(4), &result.clone());
    assert_array_has_key(&s("user"), &idx(&result.clone(), &Value::Int(0)));
    assert_equals(
        &Value::Int(1),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(0)), &s("user")),
            &s("id"),
        ),
    );
    assert_equals(
        &s("romanb"),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(0)), &s("user")),
            &s("name"),
        ),
    );
    assert_array_has_key(&s("article"), &idx(&result.clone(), &Value::Int(1)));
    assert_equals(
        &Value::Int(1),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(1)), &s("article")),
            &s("id"),
        ),
    );
    assert_equals(
        &s("Cool things."),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(1)), &s("article")),
            &s("topic"),
        ),
    );
    assert_array_has_key(&s("user"), &idx(&result.clone(), &Value::Int(2)));
    assert_equals(
        &Value::Int(2),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(2)), &s("user")),
            &s("id"),
        ),
    );
    assert_equals(
        &s("jwage"),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(2)), &s("user")),
            &s("name"),
        ),
    );
    assert_array_has_key(&s("article"), &idx(&result.clone(), &Value::Int(3)));
    assert_equals(
        &Value::Int(2),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(3)), &s("article")),
            &s("id"),
        ),
    );
    assert_equals(
        &s("Cool things II."),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(3)), &s("article")),
            &s("topic"),
        ),
    );
}

#[tokio::test]
async fn testMixedQueryNormalJoin() {
    let em = entity_manager();
    for user_entity_key in [Value::Int(0), s("user"), s("scalars"), s("newObjects")] {
        let mut rsm = ResultSetMapping::new();
        rsm_call(
            &mut rsm,
            "addEntityResult",
            &[
                s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
                s("u"),
                elvis(user_entity_key.clone(), Value::Null),
            ],
        );
        rsm_call(&mut rsm, "addFieldResult", &[s("u"), s("u__id"), s("id")]);
        rsm_call(
            &mut rsm,
            "addFieldResult",
            &[s("u"), s("u__status"), s("status")],
        );
        rsm_call(
            &mut rsm,
            "addScalarResult",
            &[s("sclr0"), s("numPhones"), s("integer")],
        );
        let mut result_set = list_of(vec![
            array_of(vec![
                (s("u__id"), s("1")),
                (s("u__status"), s("developer")),
                (s("sclr0"), s("2")),
            ]),
            array_of(vec![
                (s("u__id"), s("2")),
                (s("u__status"), s("developer")),
                (s("sclr0"), s("1")),
            ]),
        ]);
        let result = php_hydrate_all(&em, HYDRATE_ARRAY, &result_set, &rsm, &Value::Null)
            .await
            .unwrap();
        assert_count(&Value::Int(2), &result.clone());
        assert_is_array(&result.clone());
        assert_is_array(&idx(&result.clone(), &Value::Int(0)));
        assert_is_array(&idx(&result.clone(), &Value::Int(1)));
        assert_array_has_key(
            &user_entity_key.clone(),
            &idx(&result.clone(), &Value::Int(0)),
        );
        assert_equals(
            &Value::Int(2),
            &idx(&idx(&result.clone(), &Value::Int(0)), &s("numPhones")),
        );
        assert_array_has_key(
            &user_entity_key.clone(),
            &idx(&result.clone(), &Value::Int(1)),
        );
        assert_equals(
            &Value::Int(1),
            &idx(&idx(&result.clone(), &Value::Int(1)), &s("numPhones")),
        );
    }
}

#[tokio::test]
async fn testMixedQueryFetchJoin() {
    let em = entity_manager();
    for user_entity_key in [Value::Int(0), s("user"), s("scalars"), s("newObjects")] {
        let mut rsm = ResultSetMapping::new();
        rsm_call(
            &mut rsm,
            "addEntityResult",
            &[
                s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
                s("u"),
                elvis(user_entity_key.clone(), Value::Null),
            ],
        );
        rsm_call(
            &mut rsm,
            "addJoinedEntityResult",
            &[
                s("Doctrine\\Tests\\Models\\CMS\\CmsPhonenumber"),
                s("p"),
                s("u"),
                s("phonenumbers"),
            ],
        );
        rsm_call(&mut rsm, "addFieldResult", &[s("u"), s("u__id"), s("id")]);
        rsm_call(
            &mut rsm,
            "addFieldResult",
            &[s("u"), s("u__status"), s("status")],
        );
        rsm_call(
            &mut rsm,
            "addScalarResult",
            &[s("sclr0"), s("nameUpper"), s("string")],
        );
        rsm_call(
            &mut rsm,
            "addFieldResult",
            &[s("p"), s("p__phonenumber"), s("phonenumber")],
        );
        let mut result_set = list_of(vec![
            array_of(vec![
                (s("u__id"), s("1")),
                (s("u__status"), s("developer")),
                (s("sclr0"), s("ROMANB")),
                (s("p__phonenumber"), s("42")),
            ]),
            array_of(vec![
                (s("u__id"), s("1")),
                (s("u__status"), s("developer")),
                (s("sclr0"), s("ROMANB")),
                (s("p__phonenumber"), s("43")),
            ]),
            array_of(vec![
                (s("u__id"), s("2")),
                (s("u__status"), s("developer")),
                (s("sclr0"), s("JWAGE")),
                (s("p__phonenumber"), s("91")),
            ]),
        ]);
        let result = php_hydrate_all(&em, HYDRATE_ARRAY, &result_set, &rsm, &Value::Null)
            .await
            .unwrap();
        assert_count(&Value::Int(2), &result.clone());
        assert_is_array(&result.clone());
        assert_is_array(&idx(&result.clone(), &Value::Int(0)));
        assert_is_array(&idx(&result.clone(), &Value::Int(1)));
        assert_count(
            &Value::Int(2),
            &idx(
                &idx(
                    &idx(&result.clone(), &Value::Int(0)),
                    &user_entity_key.clone(),
                ),
                &s("phonenumbers"),
            ),
        );
        assert_equals(
            &s("ROMANB"),
            &idx(&idx(&result.clone(), &Value::Int(0)), &s("nameUpper")),
        );
        assert_count(
            &Value::Int(1),
            &idx(
                &idx(
                    &idx(&result.clone(), &Value::Int(1)),
                    &user_entity_key.clone(),
                ),
                &s("phonenumbers"),
            ),
        );
        assert_equals(
            &s("JWAGE"),
            &idx(&idx(&result.clone(), &Value::Int(1)), &s("nameUpper")),
        );
        assert_equals(
            &Value::Int(42),
            &idx(
                &idx(
                    &idx(
                        &idx(
                            &idx(&result.clone(), &Value::Int(0)),
                            &user_entity_key.clone(),
                        ),
                        &s("phonenumbers"),
                    ),
                    &Value::Int(0),
                ),
                &s("phonenumber"),
            ),
        );
        assert_equals(
            &Value::Int(43),
            &idx(
                &idx(
                    &idx(
                        &idx(
                            &idx(&result.clone(), &Value::Int(0)),
                            &user_entity_key.clone(),
                        ),
                        &s("phonenumbers"),
                    ),
                    &Value::Int(1),
                ),
                &s("phonenumber"),
            ),
        );
        assert_equals(
            &Value::Int(91),
            &idx(
                &idx(
                    &idx(
                        &idx(
                            &idx(&result.clone(), &Value::Int(1)),
                            &user_entity_key.clone(),
                        ),
                        &s("phonenumbers"),
                    ),
                    &Value::Int(0),
                ),
                &s("phonenumber"),
            ),
        );
    }
}

#[tokio::test]
async fn testMixedQueryFetchJoinCustomIndex() {
    let em = entity_manager();
    for user_entity_key in [Value::Int(0), s("user"), s("scalars"), s("newObjects")] {
        let mut rsm = ResultSetMapping::new();
        rsm_call(
            &mut rsm,
            "addEntityResult",
            &[
                s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
                s("u"),
                elvis(user_entity_key.clone(), Value::Null),
            ],
        );
        rsm_call(
            &mut rsm,
            "addJoinedEntityResult",
            &[
                s("Doctrine\\Tests\\Models\\CMS\\CmsPhonenumber"),
                s("p"),
                s("u"),
                s("phonenumbers"),
            ],
        );
        rsm_call(&mut rsm, "addFieldResult", &[s("u"), s("u__id"), s("id")]);
        rsm_call(
            &mut rsm,
            "addFieldResult",
            &[s("u"), s("u__status"), s("status")],
        );
        rsm_call(
            &mut rsm,
            "addScalarResult",
            &[s("sclr0"), s("nameUpper"), s("string")],
        );
        rsm_call(
            &mut rsm,
            "addFieldResult",
            &[s("p"), s("p__phonenumber"), s("phonenumber")],
        );
        rsm_call(&mut rsm, "addIndexBy", &[s("u"), s("id")]);
        rsm_call(&mut rsm, "addIndexBy", &[s("p"), s("phonenumber")]);
        let mut result_set = list_of(vec![
            array_of(vec![
                (s("u__id"), s("1")),
                (s("u__status"), s("developer")),
                (s("sclr0"), s("ROMANB")),
                (s("p__phonenumber"), s("42")),
            ]),
            array_of(vec![
                (s("u__id"), s("1")),
                (s("u__status"), s("developer")),
                (s("sclr0"), s("ROMANB")),
                (s("p__phonenumber"), s("43")),
            ]),
            array_of(vec![
                (s("u__id"), s("2")),
                (s("u__status"), s("developer")),
                (s("sclr0"), s("JWAGE")),
                (s("p__phonenumber"), s("91")),
            ]),
        ]);
        let result = php_hydrate_all(&em, HYDRATE_ARRAY, &result_set, &rsm, &Value::Null)
            .await
            .unwrap();
        assert_count(&Value::Int(2), &result.clone());
        assert_is_array(&result.clone());
        assert_is_array(&idx(&result.clone(), &Value::Int(1)));
        assert_is_array(&idx(&result.clone(), &Value::Int(2)));
        assert_equals(
            &s("ROMANB"),
            &idx(&idx(&result.clone(), &Value::Int(1)), &s("nameUpper")),
        );
        assert_equals(
            &s("JWAGE"),
            &idx(&idx(&result.clone(), &Value::Int(2)), &s("nameUpper")),
        );
        assert_count(
            &Value::Int(2),
            &idx(
                &idx(
                    &idx(&result.clone(), &Value::Int(1)),
                    &user_entity_key.clone(),
                ),
                &s("phonenumbers"),
            ),
        );
        assert_count(
            &Value::Int(1),
            &idx(
                &idx(
                    &idx(&result.clone(), &Value::Int(2)),
                    &user_entity_key.clone(),
                ),
                &s("phonenumbers"),
            ),
        );
        assert_true(&php_isset(&[idx(
            &idx(
                &idx(
                    &idx(&result.clone(), &Value::Int(1)),
                    &user_entity_key.clone(),
                ),
                &s("phonenumbers"),
            ),
            &s("42"),
        )]));
        assert_true(&php_isset(&[idx(
            &idx(
                &idx(
                    &idx(&result.clone(), &Value::Int(1)),
                    &user_entity_key.clone(),
                ),
                &s("phonenumbers"),
            ),
            &s("43"),
        )]));
        assert_true(&php_isset(&[idx(
            &idx(
                &idx(
                    &idx(&result.clone(), &Value::Int(2)),
                    &user_entity_key.clone(),
                ),
                &s("phonenumbers"),
            ),
            &s("91"),
        )]));
    }
}

#[tokio::test]
async fn testMixedQueryMultipleFetchJoin() {
    let em = entity_manager();
    let mut rsm = ResultSetMapping::new();
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[s("Doctrine\\Tests\\Models\\CMS\\CmsUser"), s("u")],
    );
    rsm_call(
        &mut rsm,
        "addJoinedEntityResult",
        &[
            s("Doctrine\\Tests\\Models\\CMS\\CmsPhonenumber"),
            s("p"),
            s("u"),
            s("phonenumbers"),
        ],
    );
    rsm_call(
        &mut rsm,
        "addJoinedEntityResult",
        &[
            s("Doctrine\\Tests\\Models\\CMS\\CmsArticle"),
            s("a"),
            s("u"),
            s("articles"),
        ],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("u"), s("u__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("u"), s("u__status"), s("status")],
    );
    rsm_call(
        &mut rsm,
        "addScalarResult",
        &[s("sclr0"), s("nameUpper"), s("string")],
    );
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("p"), s("p__phonenumber"), s("phonenumber")],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("a"), s("a__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("a"), s("a__topic"), s("topic")],
    );
    let mut result_set = list_of(vec![
        array_of(vec![
            (s("u__id"), s("1")),
            (s("u__status"), s("developer")),
            (s("sclr0"), s("ROMANB")),
            (s("p__phonenumber"), s("42")),
            (s("a__id"), s("1")),
            (s("a__topic"), s("Getting things done!")),
        ]),
        array_of(vec![
            (s("u__id"), s("1")),
            (s("u__status"), s("developer")),
            (s("sclr0"), s("ROMANB")),
            (s("p__phonenumber"), s("43")),
            (s("a__id"), s("1")),
            (s("a__topic"), s("Getting things done!")),
        ]),
        array_of(vec![
            (s("u__id"), s("1")),
            (s("u__status"), s("developer")),
            (s("sclr0"), s("ROMANB")),
            (s("p__phonenumber"), s("42")),
            (s("a__id"), s("2")),
            (s("a__topic"), s("ZendCon")),
        ]),
        array_of(vec![
            (s("u__id"), s("1")),
            (s("u__status"), s("developer")),
            (s("sclr0"), s("ROMANB")),
            (s("p__phonenumber"), s("43")),
            (s("a__id"), s("2")),
            (s("a__topic"), s("ZendCon")),
        ]),
        array_of(vec![
            (s("u__id"), s("2")),
            (s("u__status"), s("developer")),
            (s("sclr0"), s("JWAGE")),
            (s("p__phonenumber"), s("91")),
            (s("a__id"), s("3")),
            (s("a__topic"), s("LINQ")),
        ]),
        array_of(vec![
            (s("u__id"), s("2")),
            (s("u__status"), s("developer")),
            (s("sclr0"), s("JWAGE")),
            (s("p__phonenumber"), s("91")),
            (s("a__id"), s("4")),
            (s("a__topic"), s("PHP7")),
        ]),
    ]);
    let result = php_hydrate_all(&em, HYDRATE_ARRAY, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_count(&Value::Int(2), &result.clone());
    assert_is_array(&result.clone());
    assert_is_array(&idx(&result.clone(), &Value::Int(0)));
    assert_is_array(&idx(&result.clone(), &Value::Int(1)));
    assert_count(
        &Value::Int(2),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(0)), &Value::Int(0)),
            &s("phonenumbers"),
        ),
    );
    assert_count(
        &Value::Int(2),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(0)), &Value::Int(0)),
            &s("articles"),
        ),
    );
    assert_equals(
        &s("ROMANB"),
        &idx(&idx(&result.clone(), &Value::Int(0)), &s("nameUpper")),
    );
    assert_count(
        &Value::Int(1),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(1)), &Value::Int(0)),
            &s("phonenumbers"),
        ),
    );
    assert_count(
        &Value::Int(2),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(1)), &Value::Int(0)),
            &s("articles"),
        ),
    );
    assert_equals(
        &s("JWAGE"),
        &idx(&idx(&result.clone(), &Value::Int(1)), &s("nameUpper")),
    );
    assert_equals(
        &Value::Int(42),
        &idx(
            &idx(
                &idx(
                    &idx(&idx(&result.clone(), &Value::Int(0)), &Value::Int(0)),
                    &s("phonenumbers"),
                ),
                &Value::Int(0),
            ),
            &s("phonenumber"),
        ),
    );
    assert_equals(
        &Value::Int(43),
        &idx(
            &idx(
                &idx(
                    &idx(&idx(&result.clone(), &Value::Int(0)), &Value::Int(0)),
                    &s("phonenumbers"),
                ),
                &Value::Int(1),
            ),
            &s("phonenumber"),
        ),
    );
    assert_equals(
        &Value::Int(91),
        &idx(
            &idx(
                &idx(
                    &idx(&idx(&result.clone(), &Value::Int(1)), &Value::Int(0)),
                    &s("phonenumbers"),
                ),
                &Value::Int(0),
            ),
            &s("phonenumber"),
        ),
    );
    assert_equals(
        &s("Getting things done!"),
        &idx(
            &idx(
                &idx(
                    &idx(&idx(&result.clone(), &Value::Int(0)), &Value::Int(0)),
                    &s("articles"),
                ),
                &Value::Int(0),
            ),
            &s("topic"),
        ),
    );
    assert_equals(
        &s("ZendCon"),
        &idx(
            &idx(
                &idx(
                    &idx(&idx(&result.clone(), &Value::Int(0)), &Value::Int(0)),
                    &s("articles"),
                ),
                &Value::Int(1),
            ),
            &s("topic"),
        ),
    );
    assert_equals(
        &s("LINQ"),
        &idx(
            &idx(
                &idx(
                    &idx(&idx(&result.clone(), &Value::Int(1)), &Value::Int(0)),
                    &s("articles"),
                ),
                &Value::Int(0),
            ),
            &s("topic"),
        ),
    );
    assert_equals(
        &s("PHP7"),
        &idx(
            &idx(
                &idx(
                    &idx(&idx(&result.clone(), &Value::Int(1)), &Value::Int(0)),
                    &s("articles"),
                ),
                &Value::Int(1),
            ),
            &s("topic"),
        ),
    );
}

#[tokio::test]
async fn testMixedQueryMultipleDeepMixedFetchJoin() {
    let em = entity_manager();
    let mut rsm = ResultSetMapping::new();
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[s("Doctrine\\Tests\\Models\\CMS\\CmsUser"), s("u")],
    );
    rsm_call(
        &mut rsm,
        "addJoinedEntityResult",
        &[
            s("Doctrine\\Tests\\Models\\CMS\\CmsPhonenumber"),
            s("p"),
            s("u"),
            s("phonenumbers"),
        ],
    );
    rsm_call(
        &mut rsm,
        "addJoinedEntityResult",
        &[
            s("Doctrine\\Tests\\Models\\CMS\\CmsArticle"),
            s("a"),
            s("u"),
            s("articles"),
        ],
    );
    rsm_call(
        &mut rsm,
        "addJoinedEntityResult",
        &[
            s("Doctrine\\Tests\\Models\\CMS\\CmsComment"),
            s("c"),
            s("a"),
            s("comments"),
        ],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("u"), s("u__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("u"), s("u__status"), s("status")],
    );
    rsm_call(
        &mut rsm,
        "addScalarResult",
        &[s("sclr0"), s("nameUpper"), s("string")],
    );
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("p"), s("p__phonenumber"), s("phonenumber")],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("a"), s("a__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("a"), s("a__topic"), s("topic")],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("c"), s("c__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("c"), s("c__topic"), s("topic")],
    );
    let mut result_set = list_of(vec![
        array_of(vec![
            (s("u__id"), s("1")),
            (s("u__status"), s("developer")),
            (s("sclr0"), s("ROMANB")),
            (s("p__phonenumber"), s("42")),
            (s("a__id"), s("1")),
            (s("a__topic"), s("Getting things done!")),
            (s("c__id"), s("1")),
            (s("c__topic"), s("First!")),
        ]),
        array_of(vec![
            (s("u__id"), s("1")),
            (s("u__status"), s("developer")),
            (s("sclr0"), s("ROMANB")),
            (s("p__phonenumber"), s("43")),
            (s("a__id"), s("1")),
            (s("a__topic"), s("Getting things done!")),
            (s("c__id"), s("1")),
            (s("c__topic"), s("First!")),
        ]),
        array_of(vec![
            (s("u__id"), s("1")),
            (s("u__status"), s("developer")),
            (s("sclr0"), s("ROMANB")),
            (s("p__phonenumber"), s("42")),
            (s("a__id"), s("2")),
            (s("a__topic"), s("ZendCon")),
            (s("c__id"), Value::Null),
            (s("c__topic"), Value::Null),
        ]),
        array_of(vec![
            (s("u__id"), s("1")),
            (s("u__status"), s("developer")),
            (s("sclr0"), s("ROMANB")),
            (s("p__phonenumber"), s("43")),
            (s("a__id"), s("2")),
            (s("a__topic"), s("ZendCon")),
            (s("c__id"), Value::Null),
            (s("c__topic"), Value::Null),
        ]),
        array_of(vec![
            (s("u__id"), s("2")),
            (s("u__status"), s("developer")),
            (s("sclr0"), s("JWAGE")),
            (s("p__phonenumber"), s("91")),
            (s("a__id"), s("3")),
            (s("a__topic"), s("LINQ")),
            (s("c__id"), Value::Null),
            (s("c__topic"), Value::Null),
        ]),
        array_of(vec![
            (s("u__id"), s("2")),
            (s("u__status"), s("developer")),
            (s("sclr0"), s("JWAGE")),
            (s("p__phonenumber"), s("91")),
            (s("a__id"), s("4")),
            (s("a__topic"), s("PHP7")),
            (s("c__id"), Value::Null),
            (s("c__topic"), Value::Null),
        ]),
    ]);
    let result = php_hydrate_all(&em, HYDRATE_ARRAY, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_count(&Value::Int(2), &result.clone());
    assert_is_array(&result.clone());
    assert_is_array(&idx(&result.clone(), &Value::Int(0)));
    assert_is_array(&idx(&result.clone(), &Value::Int(1)));
    assert_count(
        &Value::Int(2),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(0)), &Value::Int(0)),
            &s("phonenumbers"),
        ),
    );
    assert_count(
        &Value::Int(2),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(0)), &Value::Int(0)),
            &s("articles"),
        ),
    );
    assert_count(
        &Value::Int(1),
        &idx(
            &idx(
                &idx(
                    &idx(&idx(&result.clone(), &Value::Int(0)), &Value::Int(0)),
                    &s("articles"),
                ),
                &Value::Int(0),
            ),
            &s("comments"),
        ),
    );
    assert_equals(
        &s("ROMANB"),
        &idx(&idx(&result.clone(), &Value::Int(0)), &s("nameUpper")),
    );
    assert_count(
        &Value::Int(1),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(1)), &Value::Int(0)),
            &s("phonenumbers"),
        ),
    );
    assert_count(
        &Value::Int(2),
        &idx(
            &idx(&idx(&result.clone(), &Value::Int(1)), &Value::Int(0)),
            &s("articles"),
        ),
    );
    assert_equals(
        &s("JWAGE"),
        &idx(&idx(&result.clone(), &Value::Int(1)), &s("nameUpper")),
    );
    assert_equals(
        &Value::Int(42),
        &idx(
            &idx(
                &idx(
                    &idx(&idx(&result.clone(), &Value::Int(0)), &Value::Int(0)),
                    &s("phonenumbers"),
                ),
                &Value::Int(0),
            ),
            &s("phonenumber"),
        ),
    );
    assert_equals(
        &Value::Int(43),
        &idx(
            &idx(
                &idx(
                    &idx(&idx(&result.clone(), &Value::Int(0)), &Value::Int(0)),
                    &s("phonenumbers"),
                ),
                &Value::Int(1),
            ),
            &s("phonenumber"),
        ),
    );
    assert_equals(
        &Value::Int(91),
        &idx(
            &idx(
                &idx(
                    &idx(&idx(&result.clone(), &Value::Int(1)), &Value::Int(0)),
                    &s("phonenumbers"),
                ),
                &Value::Int(0),
            ),
            &s("phonenumber"),
        ),
    );
    assert_equals(
        &s("Getting things done!"),
        &idx(
            &idx(
                &idx(
                    &idx(&idx(&result.clone(), &Value::Int(0)), &Value::Int(0)),
                    &s("articles"),
                ),
                &Value::Int(0),
            ),
            &s("topic"),
        ),
    );
    assert_equals(
        &s("ZendCon"),
        &idx(
            &idx(
                &idx(
                    &idx(&idx(&result.clone(), &Value::Int(0)), &Value::Int(0)),
                    &s("articles"),
                ),
                &Value::Int(1),
            ),
            &s("topic"),
        ),
    );
    assert_equals(
        &s("LINQ"),
        &idx(
            &idx(
                &idx(
                    &idx(&idx(&result.clone(), &Value::Int(1)), &Value::Int(0)),
                    &s("articles"),
                ),
                &Value::Int(0),
            ),
            &s("topic"),
        ),
    );
    assert_equals(
        &s("PHP7"),
        &idx(
            &idx(
                &idx(
                    &idx(&idx(&result.clone(), &Value::Int(1)), &Value::Int(0)),
                    &s("articles"),
                ),
                &Value::Int(1),
            ),
            &s("topic"),
        ),
    );
    assert_equals(
        &s("First!"),
        &idx(
            &idx(
                &idx(
                    &idx(
                        &idx(
                            &idx(&idx(&result.clone(), &Value::Int(0)), &Value::Int(0)),
                            &s("articles"),
                        ),
                        &Value::Int(0),
                    ),
                    &s("comments"),
                ),
                &Value::Int(0),
            ),
            &s("topic"),
        ),
    );
    assert_true(&php_isset(&[idx(
        &idx(
            &idx(
                &idx(&idx(&result.clone(), &Value::Int(0)), &Value::Int(0)),
                &s("articles"),
            ),
            &Value::Int(0),
        ),
        &s("comments"),
    )]));
    assert_is_array(&idx(
        &idx(
            &idx(
                &idx(&idx(&result.clone(), &Value::Int(0)), &Value::Int(0)),
                &s("articles"),
            ),
            &Value::Int(1),
        ),
        &s("comments"),
    ));
    assert_count(
        &Value::Int(0),
        &idx(
            &idx(
                &idx(
                    &idx(&idx(&result.clone(), &Value::Int(0)), &Value::Int(0)),
                    &s("articles"),
                ),
                &Value::Int(1),
            ),
            &s("comments"),
        ),
    );
    assert_is_array(&idx(
        &idx(
            &idx(
                &idx(&idx(&result.clone(), &Value::Int(1)), &Value::Int(0)),
                &s("articles"),
            ),
            &Value::Int(0),
        ),
        &s("comments"),
    ));
    assert_count(
        &Value::Int(0),
        &idx(
            &idx(
                &idx(
                    &idx(&idx(&result.clone(), &Value::Int(1)), &Value::Int(0)),
                    &s("articles"),
                ),
                &Value::Int(0),
            ),
            &s("comments"),
        ),
    );
    assert_is_array(&idx(
        &idx(
            &idx(
                &idx(&idx(&result.clone(), &Value::Int(1)), &Value::Int(0)),
                &s("articles"),
            ),
            &Value::Int(1),
        ),
        &s("comments"),
    ));
    assert_count(
        &Value::Int(0),
        &idx(
            &idx(
                &idx(
                    &idx(&idx(&result.clone(), &Value::Int(1)), &Value::Int(0)),
                    &s("articles"),
                ),
                &Value::Int(1),
            ),
            &s("comments"),
        ),
    );
}

#[tokio::test]
async fn testEntityQueryCustomResultSetOrder() {
    let em = entity_manager();
    let mut rsm = ResultSetMapping::new();
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[s("Doctrine\\Tests\\Models\\Forum\\ForumCategory"), s("c")],
    );
    rsm_call(
        &mut rsm,
        "addJoinedEntityResult",
        &[
            s("Doctrine\\Tests\\Models\\Forum\\ForumBoard"),
            s("b"),
            s("c"),
            s("boards"),
        ],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("c"), s("c__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("c"), s("c__position"), s("position")],
    );
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("c"), s("c__name"), s("name")],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("b"), s("b__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("b"), s("b__position"), s("position")],
    );
    //'b__category_id' => '1'
    //'b__category_id' => '2'
    //'b__category_id' => '1'
    //'b__category_id' => '1'
    let mut result_set = list_of(vec![
        array_of(vec![
            (s("c__id"), s("1")),
            (s("c__position"), s("0")),
            (s("c__name"), s("First")),
            (s("b__id"), s("1")),
            (s("b__position"), s("0")),
        ]),
        array_of(vec![
            (s("c__id"), s("2")),
            (s("c__position"), s("0")),
            (s("c__name"), s("Second")),
            (s("b__id"), s("2")),
            (s("b__position"), s("0")),
        ]),
        array_of(vec![
            (s("c__id"), s("1")),
            (s("c__position"), s("0")),
            (s("c__name"), s("First")),
            (s("b__id"), s("3")),
            (s("b__position"), s("1")),
        ]),
        array_of(vec![
            (s("c__id"), s("1")),
            (s("c__position"), s("0")),
            (s("c__name"), s("First")),
            (s("b__id"), s("4")),
            (s("b__position"), s("2")),
        ]),
    ]);
    let result = php_hydrate_all(&em, HYDRATE_ARRAY, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_count(&Value::Int(2), &result.clone());
    assert_is_array(&result.clone());
    assert_is_array(&idx(&result.clone(), &Value::Int(0)));
    assert_is_array(&idx(&result.clone(), &Value::Int(1)));
    assert_true(&php_isset(&[idx(
        &idx(&result.clone(), &Value::Int(0)),
        &s("boards"),
    )]));
    assert_count(
        &Value::Int(3),
        &idx(&idx(&result.clone(), &Value::Int(0)), &s("boards")),
    );
    assert_true(&php_isset(&[idx(
        &idx(&result.clone(), &Value::Int(1)),
        &s("boards"),
    )]));
    assert_count(
        &Value::Int(1),
        &idx(&idx(&result.clone(), &Value::Int(1)), &s("boards")),
    );
}

#[tokio::test]
async fn testChainedJoinWithScalars() {
    let em = entity_manager();
    for entity_key in [Value::Int(0), s("user"), s("scalars"), s("newObjects")] {
        let mut rsm = ResultSetMapping::new();
        rsm_call(
            &mut rsm,
            "addEntityResult",
            &[
                s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
                s("u"),
                elvis(entity_key.clone(), Value::Null),
            ],
        );
        rsm_call(&mut rsm, "addFieldResult", &[s("u"), s("u__id"), s("id")]);
        rsm_call(
            &mut rsm,
            "addFieldResult",
            &[s("u"), s("u__status"), s("status")],
        );
        rsm_call(
            &mut rsm,
            "addScalarResult",
            &[s("a__id"), s("id"), s("integer")],
        );
        rsm_call(
            &mut rsm,
            "addScalarResult",
            &[s("a__topic"), s("topic"), s("string")],
        );
        rsm_call(
            &mut rsm,
            "addScalarResult",
            &[s("c__id"), s("cid"), s("integer")],
        );
        rsm_call(
            &mut rsm,
            "addScalarResult",
            &[s("c__topic"), s("ctopic"), s("string")],
        );
        let mut result_set = list_of(vec![
            array_of(vec![
                (s("u__id"), s("1")),
                (s("u__status"), s("developer")),
                (s("a__id"), s("1")),
                (s("a__topic"), s("The First")),
                (s("c__id"), s("1")),
                (s("c__topic"), s("First Comment")),
            ]),
            array_of(vec![
                (s("u__id"), s("1")),
                (s("u__status"), s("developer")),
                (s("a__id"), s("1")),
                (s("a__topic"), s("The First")),
                (s("c__id"), s("2")),
                (s("c__topic"), s("Second Comment")),
            ]),
            array_of(vec![
                (s("u__id"), s("1")),
                (s("u__status"), s("developer")),
                (s("a__id"), s("42")),
                (s("a__topic"), s("The Answer")),
                (s("c__id"), Value::Null),
                (s("c__topic"), Value::Null),
            ]),
        ]);
        let result = php_hydrate_all(&em, HYDRATE_ARRAY, &result_set, &rsm, &Value::Null)
            .await
            .unwrap();
        assert_count(&Value::Int(3), &result.clone());
        assert_count(
            &Value::Int(2),
            &idx(&idx(&result.clone(), &Value::Int(0)), &entity_key.clone()),
        );
        assert_equals(
            &Value::Int(1),
            &idx(&idx(&result.clone(), &Value::Int(0)), &s("id")),
        );
        assert_equals(
            &s("The First"),
            &idx(&idx(&result.clone(), &Value::Int(0)), &s("topic")),
        );
        assert_equals(
            &Value::Int(1),
            &idx(&idx(&result.clone(), &Value::Int(0)), &s("cid")),
        );
        assert_equals(
            &s("First Comment"),
            &idx(&idx(&result.clone(), &Value::Int(0)), &s("ctopic")),
        );
        assert_count(
            &Value::Int(2),
            &idx(&idx(&result.clone(), &Value::Int(1)), &entity_key.clone()),
        );
        assert_equals(
            &Value::Int(1),
            &idx(&idx(&result.clone(), &Value::Int(1)), &s("id")),
        );
        assert_equals(
            &s("The First"),
            &idx(&idx(&result.clone(), &Value::Int(1)), &s("topic")),
        );
        assert_equals(
            &Value::Int(2),
            &idx(&idx(&result.clone(), &Value::Int(1)), &s("cid")),
        );
        assert_equals(
            &s("Second Comment"),
            &idx(&idx(&result.clone(), &Value::Int(1)), &s("ctopic")),
        );
        assert_count(
            &Value::Int(2),
            &idx(&idx(&result.clone(), &Value::Int(2)), &entity_key.clone()),
        );
        assert_equals(
            &Value::Int(42),
            &idx(&idx(&result.clone(), &Value::Int(2)), &s("id")),
        );
        assert_equals(
            &s("The Answer"),
            &idx(&idx(&result.clone(), &Value::Int(2)), &s("topic")),
        );
        assert_null(&idx(&idx(&result.clone(), &Value::Int(2)), &s("cid")));
        assert_null(&idx(&idx(&result.clone(), &Value::Int(2)), &s("ctopic")));
    }
}

#[tokio::test]
async fn testResultIteration() {
    let em = entity_manager();
    let mut rsm = ResultSetMapping::new();
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[s("Doctrine\\Tests\\Models\\CMS\\CmsUser"), s("u")],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("u"), s("u__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("u"), s("u__name"), s("name")],
    );
    let mut result_set = list_of(vec![
        array_of(vec![(s("u__id"), s("1")), (s("u__name"), s("romanb"))]),
        array_of(vec![(s("u__id"), s("2")), (s("u__name"), s("jwage"))]),
    ]);
    let iterator = php_to_iterable(&em, HYDRATE_ARRAY, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    let mut row_num = Value::Int(0);
    for row in iter(&iterator.clone()) {
        if truthy(&same(&row_num.clone(), &Value::Int(0))) {
            assert_equals(&Value::Int(1), &idx(&row.clone(), &s("id")));
            assert_equals(&s("romanb"), &idx(&row.clone(), &s("name")));
        } else if truthy(&same(&row_num.clone(), &Value::Int(1))) {
            assert_equals(&Value::Int(2), &idx(&row.clone(), &s("id")));
            assert_equals(&s("jwage"), &idx(&row.clone(), &s("name")));
        }
        row_num = inc(&row_num);
    }
}

#[tokio::test]
async fn testResultIterationWithAliasedUserEntity() {
    let em = entity_manager();
    let mut rsm = ResultSetMapping::new();
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[
            s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
            s("u"),
            s("user"),
        ],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("u"), s("u__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("u"), s("u__name"), s("name")],
    );
    let mut result_set = list_of(vec![
        array_of(vec![(s("u__id"), s("1")), (s("u__name"), s("romanb"))]),
        array_of(vec![(s("u__id"), s("2")), (s("u__name"), s("jwage"))]),
    ]);
    let iterator = php_to_iterable(&em, HYDRATE_ARRAY, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    let mut row_num = Value::Int(0);
    for row in iter(&iterator.clone()) {
        assert_array_has_key(&s("user"), &row.clone());
        if truthy(&same(&row_num.clone(), &Value::Int(0))) {
            assert_equals(
                &Value::Int(1),
                &idx(&idx(&row.clone(), &s("user")), &s("id")),
            );
            assert_equals(
                &s("romanb"),
                &idx(&idx(&row.clone(), &s("user")), &s("name")),
            );
        } else if truthy(&same(&row_num.clone(), &Value::Int(1))) {
            assert_equals(
                &Value::Int(2),
                &idx(&idx(&row.clone(), &s("user")), &s("id")),
            );
            assert_equals(
                &s("jwage"),
                &idx(&idx(&row.clone(), &s("user")), &s("name")),
            );
        }
        row_num = inc(&row_num);
    }
}

#[tokio::test]
async fn testSkipUnknownColumns() {
    let em = entity_manager();
    let mut rsm = ResultSetMapping::new();
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[s("Doctrine\\Tests\\Models\\CMS\\CmsUser"), s("u")],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("u"), s("u__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("u"), s("u__name"), s("name")],
    );
    let mut result_set = list_of(vec![array_of(vec![
        (s("u__id"), s("1")),
        (s("u__name"), s("romanb")),
        (s("foo"), s("bar")),
    ])]);
    let result = php_hydrate_all(&em, HYDRATE_ARRAY, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_count(&Value::Int(1), &result.clone());
    assert_array_has_key(&s("id"), &idx(&result.clone(), &Value::Int(0)));
    assert_array_has_key(&s("name"), &idx(&result.clone(), &Value::Int(0)));
    assert_array_not_has_key(&s("foo"), &idx(&result.clone(), &Value::Int(0)));
}

#[tokio::test]
async fn testMissingIdForRootEntity() {
    let em = entity_manager();
    for user_entity_key in [Value::Int(0), s("user"), s("scalars"), s("newObjects")] {
        let mut rsm = ResultSetMapping::new();
        rsm_call(
            &mut rsm,
            "addEntityResult",
            &[
                s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
                s("u"),
                elvis(user_entity_key.clone(), Value::Null),
            ],
        );
        rsm_call(&mut rsm, "addFieldResult", &[s("u"), s("u__id"), s("id")]);
        rsm_call(
            &mut rsm,
            "addFieldResult",
            &[s("u"), s("u__status"), s("status")],
        );
        rsm_call(
            &mut rsm,
            "addScalarResult",
            &[s("sclr0"), s("nameUpper"), s("string")],
        );
        let mut result_set = list_of(vec![
            array_of(vec![
                (s("u__id"), s("1")),
                (s("u__status"), s("developer")),
                (s("sclr0"), s("ROMANB")),
            ]),
            array_of(vec![
                (s("u__id"), Value::Null),
                (s("u__status"), Value::Null),
                (s("sclr0"), s("ROMANB")),
            ]),
            array_of(vec![
                (s("u__id"), s("2")),
                (s("u__status"), s("developer")),
                (s("sclr0"), s("JWAGE")),
            ]),
            array_of(vec![
                (s("u__id"), Value::Null),
                (s("u__status"), Value::Null),
                (s("sclr0"), s("JWAGE")),
            ]),
        ]);
        let result = php_hydrate_all(&em, HYDRATE_ARRAY, &result_set, &rsm, &Value::Null)
            .await
            .unwrap();
        assert_count(&Value::Int(4), &result.clone());
        assert_equals(
            &s("ROMANB"),
            &idx(&idx(&result.clone(), &Value::Int(0)), &s("nameUpper")),
        );
        assert_equals(
            &s("ROMANB"),
            &idx(&idx(&result.clone(), &Value::Int(1)), &s("nameUpper")),
        );
        assert_equals(
            &s("JWAGE"),
            &idx(&idx(&result.clone(), &Value::Int(2)), &s("nameUpper")),
        );
        assert_equals(
            &s("JWAGE"),
            &idx(&idx(&result.clone(), &Value::Int(3)), &s("nameUpper")),
        );
        assert_equals(
            &array_of(vec![
                (s("id"), Value::Int(1)),
                (s("status"), s("developer")),
            ]),
            &idx(
                &idx(&result.clone(), &Value::Int(0)),
                &user_entity_key.clone(),
            ),
        );
        assert_null(&idx(
            &idx(&result.clone(), &Value::Int(1)),
            &user_entity_key.clone(),
        ));
        assert_equals(
            &array_of(vec![
                (s("id"), Value::Int(2)),
                (s("status"), s("developer")),
            ]),
            &idx(
                &idx(&result.clone(), &Value::Int(2)),
                &user_entity_key.clone(),
            ),
        );
        assert_null(&idx(
            &idx(&result.clone(), &Value::Int(3)),
            &user_entity_key.clone(),
        ));
    }
}

#[tokio::test]
async fn testIndexByAndMixedResult() {
    let em = entity_manager();
    for user_entity_key in [Value::Int(0), s("user"), s("scalars"), s("newObjects")] {
        let mut rsm = ResultSetMapping::new();
        rsm_call(
            &mut rsm,
            "addEntityResult",
            &[
                s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
                s("u"),
                elvis(user_entity_key.clone(), Value::Null),
            ],
        );
        rsm_call(&mut rsm, "addFieldResult", &[s("u"), s("u__id"), s("id")]);
        rsm_call(
            &mut rsm,
            "addFieldResult",
            &[s("u"), s("u__status"), s("status")],
        );
        rsm_call(
            &mut rsm,
            "addScalarResult",
            &[s("sclr0"), s("nameUpper"), s("string")],
        );
        rsm_call(&mut rsm, "addIndexBy", &[s("u"), s("id")]);
        let mut result_set = list_of(vec![
            array_of(vec![
                (s("u__id"), s("1")),
                (s("u__status"), s("developer")),
                (s("sclr0"), s("ROMANB")),
            ]),
            array_of(vec![
                (s("u__id"), s("2")),
                (s("u__status"), s("developer")),
                (s("sclr0"), s("JWAGE")),
            ]),
        ]);
        let result = php_hydrate_all(&em, HYDRATE_ARRAY, &result_set, &rsm, &Value::Null)
            .await
            .unwrap();
        assert_count(&Value::Int(2), &result.clone());
        assert_array_has_key(&Value::Int(1), &result.clone());
        assert_equals(
            &Value::Int(1),
            &idx(
                &idx(
                    &idx(&result.clone(), &Value::Int(1)),
                    &user_entity_key.clone(),
                ),
                &s("id"),
            ),
        );
        assert_array_has_key(&Value::Int(2), &result.clone());
        assert_equals(
            &Value::Int(2),
            &idx(
                &idx(
                    &idx(&result.clone(), &Value::Int(2)),
                    &user_entity_key.clone(),
                ),
                &s("id"),
            ),
        );
    }
}
