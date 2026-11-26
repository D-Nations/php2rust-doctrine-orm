//! Upstream: doctrine/orm tests/Tests/ORM/Hydration/ObjectHydratorTest.php
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
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::query::ResultSetMapping;

use php_test::*;

/// The entity manager, on a database with the schema of the models, so
/// that the queries of inverse one-to-one associations find no rows (as
/// the mocked connection of upstream).
async fn entity_manager() -> std::sync::Arc<php2rust_doctrine_orm::entity_manager::EntityManager> {
    orm::set_up(&[
        "Doctrine\\Tests\\Models\\CMS\\CmsUser",
        "Doctrine\\Tests\\Models\\CMS\\CmsAddress",
        "Doctrine\\Tests\\Models\\CMS\\CmsArticle",
        "Doctrine\\Tests\\Models\\CMS\\CmsComment",
        "Doctrine\\Tests\\Models\\CMS\\CmsGroup",
        "Doctrine\\Tests\\Models\\CMS\\CmsPhonenumber",
        "Doctrine\\Tests\\Models\\CMS\\CmsEmail",
        "Doctrine\\Tests\\Models\\CMS\\CmsTag",
        "Doctrine\\Tests\\Models\\ECommerce\\ECommerceProduct",
        "Doctrine\\Tests\\Models\\ECommerce\\ECommerceShipping",
        "Doctrine\\Tests\\Models\\ECommerce\\ECommerceFeature",
        "Doctrine\\Tests\\Models\\ECommerce\\ECommerceCategory",
    ])
    .await
}

#[tokio::test]
async fn testSimpleEntityQuery() {
    let em = entity_manager().await;
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
    let result = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_equals(&Value::Int(2), &php_count(std::slice::from_ref(&result)));
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
        &idx(&result.clone(), &Value::Int(0)),
    );
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
        &idx(&result.clone(), &Value::Int(1)),
    );
    assert_equals(
        &Value::Int(1),
        &prop(&idx(&result.clone(), &Value::Int(0)), "id"),
    );
    assert_equals(
        &s("romanb"),
        &prop(&idx(&result.clone(), &Value::Int(0)), "name"),
    );
    assert_equals(
        &Value::Int(2),
        &prop(&idx(&result.clone(), &Value::Int(1)), "id"),
    );
    assert_equals(
        &s("jwage"),
        &prop(&idx(&result.clone(), &Value::Int(1)), "name"),
    );
}

#[tokio::test]
async fn testSimpleEntityQueryWithAliasedUserEntity() {
    let em = entity_manager().await;
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
    let result = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_equals(&Value::Int(2), &php_count(std::slice::from_ref(&result)));
    assert_array_has_key(&s("user"), &idx(&result.clone(), &Value::Int(0)));
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
        &idx(&idx(&result.clone(), &Value::Int(0)), &s("user")),
    );
    assert_array_has_key(&s("user"), &idx(&result.clone(), &Value::Int(1)));
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
        &idx(&idx(&result.clone(), &Value::Int(1)), &s("user")),
    );
    assert_equals(
        &Value::Int(1),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(0)), &s("user")),
            "id",
        ),
    );
    assert_equals(
        &s("romanb"),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(0)), &s("user")),
            "name",
        ),
    );
    assert_equals(
        &Value::Int(2),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(1)), &s("user")),
            "id",
        ),
    );
    assert_equals(
        &s("jwage"),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(1)), &s("user")),
            "name",
        ),
    );
}

#[tokio::test]
async fn testSimpleMultipleRootEntityQuery() {
    let em = entity_manager().await;
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
    let result = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_equals(&Value::Int(4), &php_count(std::slice::from_ref(&result)));
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
        &idx(&result.clone(), &Value::Int(0)),
    );
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\CMS\\CmsArticle"),
        &idx(&result.clone(), &Value::Int(1)),
    );
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
        &idx(&result.clone(), &Value::Int(2)),
    );
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\CMS\\CmsArticle"),
        &idx(&result.clone(), &Value::Int(3)),
    );
    assert_equals(
        &Value::Int(1),
        &prop(&idx(&result.clone(), &Value::Int(0)), "id"),
    );
    assert_equals(
        &s("romanb"),
        &prop(&idx(&result.clone(), &Value::Int(0)), "name"),
    );
    assert_equals(
        &Value::Int(1),
        &prop(&idx(&result.clone(), &Value::Int(1)), "id"),
    );
    assert_equals(
        &s("Cool things."),
        &prop(&idx(&result.clone(), &Value::Int(1)), "topic"),
    );
    assert_equals(
        &Value::Int(2),
        &prop(&idx(&result.clone(), &Value::Int(2)), "id"),
    );
    assert_equals(
        &s("jwage"),
        &prop(&idx(&result.clone(), &Value::Int(2)), "name"),
    );
    assert_equals(
        &Value::Int(2),
        &prop(&idx(&result.clone(), &Value::Int(3)), "id"),
    );
    assert_equals(
        &s("Cool things II."),
        &prop(&idx(&result.clone(), &Value::Int(3)), "topic"),
    );
}

#[tokio::test]
async fn testSimpleMultipleRootEntityQueryWithAliasedUserEntity() {
    let em = entity_manager().await;
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
    let result = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_equals(&Value::Int(4), &php_count(std::slice::from_ref(&result)));
    assert_array_has_key(&s("user"), &idx(&result.clone(), &Value::Int(0)));
    assert_array_not_has_key(&Value::Int(0), &idx(&result.clone(), &Value::Int(0)));
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
        &idx(&idx(&result.clone(), &Value::Int(0)), &s("user")),
    );
    assert_equals(
        &Value::Int(1),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(0)), &s("user")),
            "id",
        ),
    );
    assert_equals(
        &s("romanb"),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(0)), &s("user")),
            "name",
        ),
    );
    assert_array_has_key(&Value::Int(0), &idx(&result.clone(), &Value::Int(1)));
    assert_array_not_has_key(&s("user"), &idx(&result.clone(), &Value::Int(1)));
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\CMS\\CmsArticle"),
        &idx(&idx(&result.clone(), &Value::Int(1)), &Value::Int(0)),
    );
    assert_equals(
        &Value::Int(1),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(1)), &Value::Int(0)),
            "id",
        ),
    );
    assert_equals(
        &s("Cool things."),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(1)), &Value::Int(0)),
            "topic",
        ),
    );
    assert_array_has_key(&s("user"), &idx(&result.clone(), &Value::Int(2)));
    assert_array_not_has_key(&Value::Int(0), &idx(&result.clone(), &Value::Int(2)));
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
        &idx(&idx(&result.clone(), &Value::Int(2)), &s("user")),
    );
    assert_equals(
        &Value::Int(2),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(2)), &s("user")),
            "id",
        ),
    );
    assert_equals(
        &s("jwage"),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(2)), &s("user")),
            "name",
        ),
    );
    assert_array_has_key(&Value::Int(0), &idx(&result.clone(), &Value::Int(3)));
    assert_array_not_has_key(&s("user"), &idx(&result.clone(), &Value::Int(3)));
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\CMS\\CmsArticle"),
        &idx(&idx(&result.clone(), &Value::Int(3)), &Value::Int(0)),
    );
    assert_equals(
        &Value::Int(2),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(3)), &Value::Int(0)),
            "id",
        ),
    );
    assert_equals(
        &s("Cool things II."),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(3)), &Value::Int(0)),
            "topic",
        ),
    );
}

#[tokio::test]
async fn testSimpleMultipleRootEntityQueryWithAliasedArticleEntity() {
    let em = entity_manager().await;
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
    let result = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_equals(&Value::Int(4), &php_count(std::slice::from_ref(&result)));
    assert_array_has_key(&Value::Int(0), &idx(&result.clone(), &Value::Int(0)));
    assert_array_not_has_key(&s("article"), &idx(&result.clone(), &Value::Int(0)));
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
        &idx(&idx(&result.clone(), &Value::Int(0)), &Value::Int(0)),
    );
    assert_equals(
        &Value::Int(1),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(0)), &Value::Int(0)),
            "id",
        ),
    );
    assert_equals(
        &s("romanb"),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(0)), &Value::Int(0)),
            "name",
        ),
    );
    assert_array_has_key(&s("article"), &idx(&result.clone(), &Value::Int(1)));
    assert_array_not_has_key(&Value::Int(0), &idx(&result.clone(), &Value::Int(1)));
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\CMS\\CmsArticle"),
        &idx(&idx(&result.clone(), &Value::Int(1)), &s("article")),
    );
    assert_equals(
        &Value::Int(1),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(1)), &s("article")),
            "id",
        ),
    );
    assert_equals(
        &s("Cool things."),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(1)), &s("article")),
            "topic",
        ),
    );
    assert_array_has_key(&Value::Int(0), &idx(&result.clone(), &Value::Int(2)));
    assert_array_not_has_key(&s("article"), &idx(&result.clone(), &Value::Int(2)));
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
        &idx(&idx(&result.clone(), &Value::Int(2)), &Value::Int(0)),
    );
    assert_equals(
        &Value::Int(2),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(2)), &Value::Int(0)),
            "id",
        ),
    );
    assert_equals(
        &s("jwage"),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(2)), &Value::Int(0)),
            "name",
        ),
    );
    assert_array_has_key(&s("article"), &idx(&result.clone(), &Value::Int(3)));
    assert_array_not_has_key(&Value::Int(0), &idx(&result.clone(), &Value::Int(3)));
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\CMS\\CmsArticle"),
        &idx(&idx(&result.clone(), &Value::Int(3)), &s("article")),
    );
    assert_equals(
        &Value::Int(2),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(3)), &s("article")),
            "id",
        ),
    );
    assert_equals(
        &s("Cool things II."),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(3)), &s("article")),
            "topic",
        ),
    );
}

#[tokio::test]
async fn testSimpleMultipleRootEntityQueryWithAliasedEntities() {
    let em = entity_manager().await;
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
    let result = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_equals(&Value::Int(4), &php_count(std::slice::from_ref(&result)));
    assert_array_has_key(&s("user"), &idx(&result.clone(), &Value::Int(0)));
    assert_array_not_has_key(&s("article"), &idx(&result.clone(), &Value::Int(0)));
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
        &idx(&idx(&result.clone(), &Value::Int(0)), &s("user")),
    );
    assert_equals(
        &Value::Int(1),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(0)), &s("user")),
            "id",
        ),
    );
    assert_equals(
        &s("romanb"),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(0)), &s("user")),
            "name",
        ),
    );
    assert_array_has_key(&s("article"), &idx(&result.clone(), &Value::Int(1)));
    assert_array_not_has_key(&s("user"), &idx(&result.clone(), &Value::Int(1)));
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\CMS\\CmsArticle"),
        &idx(&idx(&result.clone(), &Value::Int(1)), &s("article")),
    );
    assert_equals(
        &Value::Int(1),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(1)), &s("article")),
            "id",
        ),
    );
    assert_equals(
        &s("Cool things."),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(1)), &s("article")),
            "topic",
        ),
    );
    assert_array_has_key(&s("user"), &idx(&result.clone(), &Value::Int(2)));
    assert_array_not_has_key(&s("article"), &idx(&result.clone(), &Value::Int(2)));
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
        &idx(&idx(&result.clone(), &Value::Int(2)), &s("user")),
    );
    assert_equals(
        &Value::Int(2),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(2)), &s("user")),
            "id",
        ),
    );
    assert_equals(
        &s("jwage"),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(2)), &s("user")),
            "name",
        ),
    );
    assert_array_has_key(&s("article"), &idx(&result.clone(), &Value::Int(3)));
    assert_array_not_has_key(&s("user"), &idx(&result.clone(), &Value::Int(3)));
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\CMS\\CmsArticle"),
        &idx(&idx(&result.clone(), &Value::Int(3)), &s("article")),
    );
    assert_equals(
        &Value::Int(2),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(3)), &s("article")),
            "id",
        ),
    );
    assert_equals(
        &s("Cool things II."),
        &prop(
            &idx(&idx(&result.clone(), &Value::Int(3)), &s("article")),
            "topic",
        ),
    );
}

#[tokio::test]
async fn testMixedQueryNormalJoin() {
    let em = entity_manager().await;
    for user_entity_key in [Value::Int(0), s("user")] {
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
        let result = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
            .await
            .unwrap();
        assert_equals(&Value::Int(2), &php_count(std::slice::from_ref(&result)));
        assert_is_array(&result.clone());
        assert_is_array(&idx(&result.clone(), &Value::Int(0)));
        assert_is_array(&idx(&result.clone(), &Value::Int(1)));
        assert_equals(
            &Value::Int(2),
            &idx(&idx(&result.clone(), &Value::Int(0)), &s("numPhones")),
        );
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
            &idx(
                &idx(&result.clone(), &Value::Int(0)),
                &user_entity_key.clone(),
            ),
        );
        assert_equals(
            &Value::Int(1),
            &idx(&idx(&result.clone(), &Value::Int(1)), &s("numPhones")),
        );
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
            &idx(
                &idx(&result.clone(), &Value::Int(1)),
                &user_entity_key.clone(),
            ),
        );
    }
}

#[tokio::test]
async fn testMixedQueryFetchJoin() {
    let em = entity_manager().await;
    for user_entity_key in [Value::Int(0), s("user")] {
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
            "addFieldResult",
            &[s("p"), s("p__phonenumber"), s("phonenumber")],
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
                (s("p__phonenumber"), s("42")),
                (s("sclr0"), s("ROMANB")),
            ]),
            array_of(vec![
                (s("u__id"), s("1")),
                (s("u__status"), s("developer")),
                (s("p__phonenumber"), s("43")),
                (s("sclr0"), s("ROMANB")),
            ]),
            array_of(vec![
                (s("u__id"), s("2")),
                (s("u__status"), s("developer")),
                (s("p__phonenumber"), s("91")),
                (s("sclr0"), s("JWAGE")),
            ]),
        ]);
        let result = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
            .await
            .unwrap();
        assert_equals(&Value::Int(2), &php_count(std::slice::from_ref(&result)));
        assert_is_array(&result.clone());
        assert_is_array(&idx(&result.clone(), &Value::Int(0)));
        assert_is_array(&idx(&result.clone(), &Value::Int(1)));
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
            &idx(
                &idx(&result.clone(), &Value::Int(0)),
                &user_entity_key.clone(),
            ),
        );
        assert_instance_of(
            &s("Doctrine\\ORM\\PersistentCollection"),
            &prop(
                &idx(
                    &idx(&result.clone(), &Value::Int(0)),
                    &user_entity_key.clone(),
                ),
                "phonenumbers",
            ),
        );
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsPhonenumber"),
            &idx(
                &prop(
                    &idx(
                        &idx(&result.clone(), &Value::Int(0)),
                        &user_entity_key.clone(),
                    ),
                    "phonenumbers",
                ),
                &Value::Int(0),
            ),
        );
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
            &idx(
                &idx(&result.clone(), &Value::Int(1)),
                &user_entity_key.clone(),
            ),
        );
        assert_instance_of(
            &s("Doctrine\\ORM\\PersistentCollection"),
            &prop(
                &idx(
                    &idx(&result.clone(), &Value::Int(1)),
                    &user_entity_key.clone(),
                ),
                "phonenumbers",
            ),
        );
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsPhonenumber"),
            &idx(
                &prop(
                    &idx(
                        &idx(&result.clone(), &Value::Int(0)),
                        &user_entity_key.clone(),
                    ),
                    "phonenumbers",
                ),
                &Value::Int(1),
            ),
        );
        assert_equals(
            &Value::Int(2),
            &php_count(&[prop(
                &idx(
                    &idx(&result.clone(), &Value::Int(0)),
                    &user_entity_key.clone(),
                ),
                "phonenumbers",
            )]),
        );
        assert_equals(
            &s("ROMANB"),
            &idx(&idx(&result.clone(), &Value::Int(0)), &s("nameUpper")),
        );
        assert_equals(
            &Value::Int(1),
            &php_count(&[prop(
                &idx(
                    &idx(&result.clone(), &Value::Int(1)),
                    &user_entity_key.clone(),
                ),
                "phonenumbers",
            )]),
        );
        assert_equals(
            &s("JWAGE"),
            &idx(&idx(&result.clone(), &Value::Int(1)), &s("nameUpper")),
        );
        assert_equals(
            &Value::Int(42),
            &prop(
                &idx(
                    &prop(
                        &idx(
                            &idx(&result.clone(), &Value::Int(0)),
                            &user_entity_key.clone(),
                        ),
                        "phonenumbers",
                    ),
                    &Value::Int(0),
                ),
                "phonenumber",
            ),
        );
        assert_equals(
            &Value::Int(43),
            &prop(
                &idx(
                    &prop(
                        &idx(
                            &idx(&result.clone(), &Value::Int(0)),
                            &user_entity_key.clone(),
                        ),
                        "phonenumbers",
                    ),
                    &Value::Int(1),
                ),
                "phonenumber",
            ),
        );
        assert_equals(
            &Value::Int(91),
            &prop(
                &idx(
                    &prop(
                        &idx(
                            &idx(&result.clone(), &Value::Int(1)),
                            &user_entity_key.clone(),
                        ),
                        "phonenumbers",
                    ),
                    &Value::Int(0),
                ),
                "phonenumber",
            ),
        );
    }
}

#[tokio::test]
async fn testMixedQueryFetchJoinCustomIndex() {
    let em = entity_manager().await;
    for user_entity_key in [Value::Int(0), s("user")] {
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
        let result = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
            .await
            .unwrap();
        assert_equals(&Value::Int(2), &php_count(std::slice::from_ref(&result)));
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
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
            &idx(
                &idx(&result.clone(), &Value::Int(1)),
                &user_entity_key.clone(),
            ),
        );
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
            &idx(
                &idx(&result.clone(), &Value::Int(2)),
                &user_entity_key.clone(),
            ),
        );
        assert_instance_of(
            &s("Doctrine\\ORM\\PersistentCollection"),
            &prop(
                &idx(
                    &idx(&result.clone(), &Value::Int(1)),
                    &user_entity_key.clone(),
                ),
                "phonenumbers",
            ),
        );
        assert_equals(
            &Value::Int(2),
            &php_count(&[prop(
                &idx(
                    &idx(&result.clone(), &Value::Int(1)),
                    &user_entity_key.clone(),
                ),
                "phonenumbers",
            )]),
        );
        assert_equals(
            &Value::Int(1),
            &php_count(&[prop(
                &idx(
                    &idx(&result.clone(), &Value::Int(2)),
                    &user_entity_key.clone(),
                ),
                "phonenumbers",
            )]),
        );
        assert_true(&php_isset(&[idx(
            &prop(
                &idx(
                    &idx(&result.clone(), &Value::Int(1)),
                    &user_entity_key.clone(),
                ),
                "phonenumbers",
            ),
            &s("42"),
        )]));
        assert_true(&php_isset(&[idx(
            &prop(
                &idx(
                    &idx(&result.clone(), &Value::Int(1)),
                    &user_entity_key.clone(),
                ),
                "phonenumbers",
            ),
            &s("43"),
        )]));
        assert_true(&php_isset(&[idx(
            &prop(
                &idx(
                    &idx(&result.clone(), &Value::Int(2)),
                    &user_entity_key.clone(),
                ),
                "phonenumbers",
            ),
            &s("91"),
        )]));
    }
}

#[tokio::test]
async fn testMixedQueryMultipleFetchJoin() {
    let em = entity_manager().await;
    for user_entity_key in [Value::Int(0), s("user")] {
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
        let result = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
            .await
            .unwrap();
        assert_equals(&Value::Int(2), &php_count(std::slice::from_ref(&result)));
        assert_is_array(&result.clone());
        assert_is_array(&idx(&result.clone(), &Value::Int(0)));
        assert_is_array(&idx(&result.clone(), &Value::Int(1)));
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
            &idx(
                &idx(&result.clone(), &Value::Int(0)),
                &user_entity_key.clone(),
            ),
        );
        assert_instance_of(
            &s("Doctrine\\ORM\\PersistentCollection"),
            &prop(
                &idx(
                    &idx(&result.clone(), &Value::Int(0)),
                    &user_entity_key.clone(),
                ),
                "phonenumbers",
            ),
        );
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsPhonenumber"),
            &idx(
                &prop(
                    &idx(
                        &idx(&result.clone(), &Value::Int(0)),
                        &user_entity_key.clone(),
                    ),
                    "phonenumbers",
                ),
                &Value::Int(0),
            ),
        );
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsPhonenumber"),
            &idx(
                &prop(
                    &idx(
                        &idx(&result.clone(), &Value::Int(0)),
                        &user_entity_key.clone(),
                    ),
                    "phonenumbers",
                ),
                &Value::Int(1),
            ),
        );
        assert_instance_of(
            &s("Doctrine\\ORM\\PersistentCollection"),
            &prop(
                &idx(
                    &idx(&result.clone(), &Value::Int(0)),
                    &user_entity_key.clone(),
                ),
                "articles",
            ),
        );
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsArticle"),
            &idx(
                &prop(
                    &idx(
                        &idx(&result.clone(), &Value::Int(0)),
                        &user_entity_key.clone(),
                    ),
                    "articles",
                ),
                &Value::Int(0),
            ),
        );
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsArticle"),
            &idx(
                &prop(
                    &idx(
                        &idx(&result.clone(), &Value::Int(0)),
                        &user_entity_key.clone(),
                    ),
                    "articles",
                ),
                &Value::Int(1),
            ),
        );
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
            &idx(
                &idx(&result.clone(), &Value::Int(1)),
                &user_entity_key.clone(),
            ),
        );
        assert_instance_of(
            &s("Doctrine\\ORM\\PersistentCollection"),
            &prop(
                &idx(
                    &idx(&result.clone(), &Value::Int(1)),
                    &user_entity_key.clone(),
                ),
                "phonenumbers",
            ),
        );
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsPhonenumber"),
            &idx(
                &prop(
                    &idx(
                        &idx(&result.clone(), &Value::Int(1)),
                        &user_entity_key.clone(),
                    ),
                    "phonenumbers",
                ),
                &Value::Int(0),
            ),
        );
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsArticle"),
            &idx(
                &prop(
                    &idx(
                        &idx(&result.clone(), &Value::Int(1)),
                        &user_entity_key.clone(),
                    ),
                    "articles",
                ),
                &Value::Int(0),
            ),
        );
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsArticle"),
            &idx(
                &prop(
                    &idx(
                        &idx(&result.clone(), &Value::Int(1)),
                        &user_entity_key.clone(),
                    ),
                    "articles",
                ),
                &Value::Int(1),
            ),
        );
    }
}

#[tokio::test]
async fn testMixedQueryMultipleDeepMixedFetchJoin() {
    let em = entity_manager().await;
    for user_entity_key in [Value::Int(0), s("user")] {
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
        let result = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
            .await
            .unwrap();
        assert_equals(&Value::Int(2), &php_count(std::slice::from_ref(&result)));
        assert_is_array(&result.clone());
        assert_is_array(&idx(&result.clone(), &Value::Int(0)));
        assert_is_array(&idx(&result.clone(), &Value::Int(1)));
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
            &idx(
                &idx(&result.clone(), &Value::Int(0)),
                &user_entity_key.clone(),
            ),
        );
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
            &idx(
                &idx(&result.clone(), &Value::Int(1)),
                &user_entity_key.clone(),
            ),
        );
        assert_instance_of(
            &s("Doctrine\\ORM\\PersistentCollection"),
            &prop(
                &idx(
                    &idx(&result.clone(), &Value::Int(0)),
                    &user_entity_key.clone(),
                ),
                "phonenumbers",
            ),
        );
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsPhonenumber"),
            &idx(
                &prop(
                    &idx(
                        &idx(&result.clone(), &Value::Int(0)),
                        &user_entity_key.clone(),
                    ),
                    "phonenumbers",
                ),
                &Value::Int(0),
            ),
        );
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsPhonenumber"),
            &idx(
                &prop(
                    &idx(
                        &idx(&result.clone(), &Value::Int(0)),
                        &user_entity_key.clone(),
                    ),
                    "phonenumbers",
                ),
                &Value::Int(1),
            ),
        );
        assert_instance_of(
            &s("Doctrine\\ORM\\PersistentCollection"),
            &prop(
                &idx(
                    &idx(&result.clone(), &Value::Int(1)),
                    &user_entity_key.clone(),
                ),
                "phonenumbers",
            ),
        );
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsPhonenumber"),
            &idx(
                &prop(
                    &idx(
                        &idx(&result.clone(), &Value::Int(1)),
                        &user_entity_key.clone(),
                    ),
                    "phonenumbers",
                ),
                &Value::Int(0),
            ),
        );
        assert_instance_of(
            &s("Doctrine\\ORM\\PersistentCollection"),
            &prop(
                &idx(
                    &idx(&result.clone(), &Value::Int(0)),
                    &user_entity_key.clone(),
                ),
                "articles",
            ),
        );
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsArticle"),
            &idx(
                &prop(
                    &idx(
                        &idx(&result.clone(), &Value::Int(0)),
                        &user_entity_key.clone(),
                    ),
                    "articles",
                ),
                &Value::Int(0),
            ),
        );
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsArticle"),
            &idx(
                &prop(
                    &idx(
                        &idx(&result.clone(), &Value::Int(0)),
                        &user_entity_key.clone(),
                    ),
                    "articles",
                ),
                &Value::Int(1),
            ),
        );
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsArticle"),
            &idx(
                &prop(
                    &idx(
                        &idx(&result.clone(), &Value::Int(1)),
                        &user_entity_key.clone(),
                    ),
                    "articles",
                ),
                &Value::Int(0),
            ),
        );
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsArticle"),
            &idx(
                &prop(
                    &idx(
                        &idx(&result.clone(), &Value::Int(1)),
                        &user_entity_key.clone(),
                    ),
                    "articles",
                ),
                &Value::Int(1),
            ),
        );
        assert_instance_of(
            &s("Doctrine\\ORM\\PersistentCollection"),
            &prop(
                &idx(
                    &prop(
                        &idx(
                            &idx(&result.clone(), &Value::Int(0)),
                            &user_entity_key.clone(),
                        ),
                        "articles",
                    ),
                    &Value::Int(0),
                ),
                "comments",
            ),
        );
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsComment"),
            &idx(
                &prop(
                    &idx(
                        &prop(
                            &idx(
                                &idx(&result.clone(), &Value::Int(0)),
                                &user_entity_key.clone(),
                            ),
                            "articles",
                        ),
                        &Value::Int(0),
                    ),
                    "comments",
                ),
                &Value::Int(0),
            ),
        );
        assert_instance_of(
            &s("Doctrine\\ORM\\PersistentCollection"),
            &prop(
                &idx(
                    &prop(
                        &idx(
                            &idx(&result.clone(), &Value::Int(0)),
                            &user_entity_key.clone(),
                        ),
                        "articles",
                    ),
                    &Value::Int(1),
                ),
                "comments",
            ),
        );
        assert_equals(
            &Value::Int(0),
            &php_count(&[prop(
                &idx(
                    &prop(
                        &idx(
                            &idx(&result.clone(), &Value::Int(0)),
                            &user_entity_key.clone(),
                        ),
                        "articles",
                    ),
                    &Value::Int(1),
                ),
                "comments",
            )]),
        );
        assert_instance_of(
            &s("Doctrine\\ORM\\PersistentCollection"),
            &prop(
                &idx(
                    &prop(
                        &idx(
                            &idx(&result.clone(), &Value::Int(1)),
                            &user_entity_key.clone(),
                        ),
                        "articles",
                    ),
                    &Value::Int(0),
                ),
                "comments",
            ),
        );
        assert_equals(
            &Value::Int(0),
            &php_count(&[prop(
                &idx(
                    &prop(
                        &idx(
                            &idx(&result.clone(), &Value::Int(1)),
                            &user_entity_key.clone(),
                        ),
                        "articles",
                    ),
                    &Value::Int(0),
                ),
                "comments",
            )]),
        );
        assert_instance_of(
            &s("Doctrine\\ORM\\PersistentCollection"),
            &prop(
                &idx(
                    &prop(
                        &idx(
                            &idx(&result.clone(), &Value::Int(1)),
                            &user_entity_key.clone(),
                        ),
                        "articles",
                    ),
                    &Value::Int(1),
                ),
                "comments",
            ),
        );
        assert_equals(
            &Value::Int(0),
            &php_count(&[prop(
                &idx(
                    &prop(
                        &idx(
                            &idx(&result.clone(), &Value::Int(1)),
                            &user_entity_key.clone(),
                        ),
                        "articles",
                    ),
                    &Value::Int(1),
                ),
                "comments",
            )]),
        );
    }
}

#[tokio::test]
async fn testEntityQueryCustomResultSetOrder() {
    let em = entity_manager().await;
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
    let result = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_equals(&Value::Int(2), &php_count(std::slice::from_ref(&result)));
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\Forum\\ForumCategory"),
        &idx(&result.clone(), &Value::Int(0)),
    );
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\Forum\\ForumCategory"),
        &idx(&result.clone(), &Value::Int(1)),
    );
    assert_true(&not_same(
        &idx(&result.clone(), &Value::Int(0)),
        &idx(&result.clone(), &Value::Int(1)),
    ));
    assert_equals(
        &Value::Int(1),
        &call(&idx(&result.clone(), &Value::Int(0)), "getId", &[]),
    );
    assert_equals(
        &Value::Int(2),
        &call(&idx(&result.clone(), &Value::Int(1)), "getId", &[]),
    );
    assert_true(&php_property_exists(&[
        idx(&result.clone(), &Value::Int(0)),
        s("boards"),
    ]));
    assert_equals(
        &Value::Int(3),
        &php_count(&[prop(&idx(&result.clone(), &Value::Int(0)), "boards")]),
    );
    assert_true(&php_property_exists(&[
        idx(&result.clone(), &Value::Int(1)),
        s("boards"),
    ]));
    assert_equals(
        &Value::Int(1),
        &php_count(&[prop(&idx(&result.clone(), &Value::Int(1)), "boards")]),
    );
}

#[tokio::test]
async fn testSkipUnknownColumns() {
    let em = entity_manager().await;
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
    let result = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_equals(&Value::Int(1), &php_count(std::slice::from_ref(&result)));
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
        &idx(&result.clone(), &Value::Int(0)),
    );
}

#[tokio::test]
async fn testScalarQueryWithoutResultVariables() {
    let em = entity_manager().await;
    for user_entity_key in [Value::Int(0), s("user")] {
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
            "addScalarResult",
            &[s("sclr0"), s("id"), s("integer")],
        );
        rsm_call(
            &mut rsm,
            "addScalarResult",
            &[s("sclr1"), s("name"), s("string")],
        );
        let mut result_set = list_of(vec![
            array_of(vec![(s("sclr0"), s("1")), (s("sclr1"), s("romanb"))]),
            array_of(vec![(s("sclr0"), s("2")), (s("sclr1"), s("jwage"))]),
        ]);
        let result = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
            .await
            .unwrap();
        assert_equals(&Value::Int(2), &php_count(std::slice::from_ref(&result)));
        assert_is_array(&idx(&result.clone(), &Value::Int(0)));
        assert_is_array(&idx(&result.clone(), &Value::Int(1)));
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
}

#[tokio::test]
async fn testCreatesProxyForLazyLoadingWithForeignKeys() {
    let em = entity_manager().await;
    let mut rsm = ResultSetMapping::new();
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[
            s("Doctrine\\Tests\\Models\\ECommerce\\ECommerceProduct"),
            s("p"),
        ],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("p"), s("p__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("p"), s("p__name"), s("name")],
    );
    rsm_call(
        &mut rsm,
        "addMetaResult",
        &[
            s("p"),
            s("p__shipping_id"),
            s("shipping_id"),
            Value::Bool(false),
            s("integer"),
        ],
    );
    // Faked result set
    let mut result_set = list_of(vec![array_of(vec![
        (s("p__id"), s("1")),
        (s("p__name"), s("Doctrine Book")),
        (s("p__shipping_id"), Value::Int(42)),
    ])]);
    // configuring lazy loading
    em.get_class_metadata("Doctrine\\Tests\\Models\\ECommerce\\ECommerceProduct")
        .unwrap()
        .write()
        .association_mappings
        .get_mut("shipping")
        .unwrap()
        .fetch = Some(2);
    let result = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_equals(&Value::Int(1), &php_count(std::slice::from_ref(&result)));
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\ECommerce\\ECommerceProduct"),
        &idx(&result.clone(), &Value::Int(0)),
    );
    let product = idx(&result, &Value::Int(0));
    // The spy on getProxy(): a proxy of ECommerceShipping with ['id' => 42].
    let shipping = prop(&product, "shipping");
    assert_same(
        &s("Doctrine\\Tests\\Models\\ECommerce\\ECommerceShipping"),
        &s(match &shipping {
            Value::Object(o) => o.class(),
            _ => "",
        }),
    );
    assert!(em.is_uninitialized_object(&shipping));
    assert_equals(
        &Value::Int(42),
        &em.get_unit_of_work()
            .get_entity_identifier(&orm::obj(&shipping))
            .unwrap()
            .get(&"id".into())
            .cloned()
            .unwrap(),
    );
}

#[tokio::test]
async fn testCreatesProxyForLazyLoadingWithForeignKeysWithAliasedProductEntity() {
    let em = entity_manager().await;
    let mut rsm = ResultSetMapping::new();
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[
            s("Doctrine\\Tests\\Models\\ECommerce\\ECommerceProduct"),
            s("p"),
            s("product"),
        ],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("p"), s("p__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("p"), s("p__name"), s("name")],
    );
    rsm_call(
        &mut rsm,
        "addMetaResult",
        &[
            s("p"),
            s("p__shipping_id"),
            s("shipping_id"),
            Value::Bool(false),
            s("integer"),
        ],
    );
    // Faked result set
    let mut result_set = list_of(vec![array_of(vec![
        (s("p__id"), s("1")),
        (s("p__name"), s("Doctrine Book")),
        (s("p__shipping_id"), Value::Int(42)),
    ])]);
    // configuring lazy loading
    em.get_class_metadata("Doctrine\\Tests\\Models\\ECommerce\\ECommerceProduct")
        .unwrap()
        .write()
        .association_mappings
        .get_mut("shipping")
        .unwrap()
        .fetch = Some(2);
    let result = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_equals(&Value::Int(1), &php_count(std::slice::from_ref(&result)));
    assert_is_array(&idx(&result.clone(), &Value::Int(0)));
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\ECommerce\\ECommerceProduct"),
        &idx(&idx(&result.clone(), &Value::Int(0)), &s("product")),
    );
    let product = idx(&idx(&result, &Value::Int(0)), &s("product"));
    // The spy on getProxy(): a proxy of ECommerceShipping with ['id' => 42].
    let shipping = prop(&product, "shipping");
    assert_same(
        &s("Doctrine\\Tests\\Models\\ECommerce\\ECommerceShipping"),
        &s(match &shipping {
            Value::Object(o) => o.class(),
            _ => "",
        }),
    );
    assert!(em.is_uninitialized_object(&shipping));
    assert_equals(
        &Value::Int(42),
        &em.get_unit_of_work()
            .get_entity_identifier(&orm::obj(&shipping))
            .unwrap()
            .get(&"id".into())
            .cloned()
            .unwrap(),
    );
}

#[tokio::test]
async fn testChainedJoinWithEmptyCollections() {
    let em = entity_manager().await;
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
            (s("a__id"), Value::Null),
            (s("a__topic"), Value::Null),
            (s("c__id"), Value::Null),
            (s("c__topic"), Value::Null),
        ]),
        array_of(vec![
            (s("u__id"), s("2")),
            (s("u__status"), s("developer")),
            (s("a__id"), Value::Null),
            (s("a__topic"), Value::Null),
            (s("c__id"), Value::Null),
            (s("c__topic"), Value::Null),
        ]),
    ]);
    let result = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_equals(&Value::Int(2), &php_count(std::slice::from_ref(&result)));
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
        &idx(&result.clone(), &Value::Int(0)),
    );
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
        &idx(&result.clone(), &Value::Int(1)),
    );
    assert_equals(
        &Value::Int(0),
        &call(
            &prop(&idx(&result.clone(), &Value::Int(0)), "articles"),
            "count",
            &[],
        ),
    );
    assert_equals(
        &Value::Int(0),
        &call(
            &prop(&idx(&result.clone(), &Value::Int(1)), "articles"),
            "count",
            &[],
        ),
    );
}

#[tokio::test]
async fn testChainedJoinWithEmptyCollectionsWithAliasedUserEntity() {
    let em = entity_manager().await;
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
            (s("a__id"), Value::Null),
            (s("a__topic"), Value::Null),
            (s("c__id"), Value::Null),
            (s("c__topic"), Value::Null),
        ]),
        array_of(vec![
            (s("u__id"), s("2")),
            (s("u__status"), s("developer")),
            (s("a__id"), Value::Null),
            (s("a__topic"), Value::Null),
            (s("c__id"), Value::Null),
            (s("c__topic"), Value::Null),
        ]),
    ]);
    let result = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_equals(&Value::Int(2), &php_count(std::slice::from_ref(&result)));
    assert_is_array(&idx(&result.clone(), &Value::Int(0)));
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
        &idx(&idx(&result.clone(), &Value::Int(0)), &s("user")),
    );
    assert_is_array(&idx(&result.clone(), &Value::Int(1)));
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
        &idx(&idx(&result.clone(), &Value::Int(1)), &s("user")),
    );
    assert_equals(
        &Value::Int(0),
        &call(
            &prop(
                &idx(&idx(&result.clone(), &Value::Int(0)), &s("user")),
                "articles",
            ),
            "count",
            &[],
        ),
    );
    assert_equals(
        &Value::Int(0),
        &call(
            &prop(
                &idx(&idx(&result.clone(), &Value::Int(1)), &s("user")),
                "articles",
            ),
            "count",
            &[],
        ),
    );
}

#[tokio::test]
async fn testResultIteration() {
    let em = entity_manager().await;
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
    let mut iterable_result = php_to_iterable(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    let mut row_num = Value::Int(0);
    for row in iter(&iterable_result.clone()) {
        assert_instance_of(&s("Doctrine\\Tests\\Models\\CMS\\CmsUser"), &row.clone());
        if truthy(&same(&row_num.clone(), &Value::Int(0))) {
            assert_equals(&Value::Int(1), &prop(&row.clone(), "id"));
            assert_equals(&s("romanb"), &prop(&row.clone(), "name"));
        } else if truthy(&same(&row_num.clone(), &Value::Int(1))) {
            assert_equals(&Value::Int(2), &prop(&row.clone(), "id"));
            assert_equals(&s("jwage"), &prop(&row.clone(), "name"));
        }
        row_num = inc(&row_num);
    }
    assert_same(&Value::Int(2), &row_num.clone());
    iterable_result = php_to_iterable(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    row_num = Value::Int(0);
    for user in iter(&iterable_result.clone()) {
        assert_instance_of(&s("Doctrine\\Tests\\Models\\CMS\\CmsUser"), &user.clone());
        if truthy(&same(&row_num.clone(), &Value::Int(0))) {
            assert_equals(&Value::Int(1), &prop(&user.clone(), "id"));
            assert_equals(&s("romanb"), &prop(&user.clone(), "name"));
        }
        if truthy(&same(&row_num.clone(), &Value::Int(1))) {
            assert_equals(&Value::Int(2), &prop(&user.clone(), "id"));
            assert_equals(&s("jwage"), &prop(&user.clone(), "name"));
        }
        row_num = inc(&row_num);
    }
    assert_same(&Value::Int(2), &row_num.clone());
}

#[tokio::test]
async fn testResultIterationWithAliasedUserEntity() {
    let em = entity_manager().await;
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
    let mut row_num = Value::Int(0);
    let mut iterable_result = php_to_iterable(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    for row in iter(&iterable_result.clone()) {
        assert_count(&Value::Int(1), &row.clone());
        assert_array_has_key(&s("user"), &row.clone());
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
            &idx(&row.clone(), &s("user")),
        );
        if truthy(&same(&row_num.clone(), &Value::Int(0))) {
            assert_equals(&Value::Int(1), &prop(&idx(&row.clone(), &s("user")), "id"));
            assert_equals(&s("romanb"), &prop(&idx(&row.clone(), &s("user")), "name"));
        } else if truthy(&same(&row_num.clone(), &Value::Int(1))) {
            assert_equals(&Value::Int(2), &prop(&idx(&row.clone(), &s("user")), "id"));
            assert_equals(&s("jwage"), &prop(&idx(&row.clone(), &s("user")), "name"));
        }
        row_num = inc(&row_num);
    }
    assert_same(&Value::Int(2), &row_num.clone());
    row_num = Value::Int(0);
    let mut iterable_result = php_to_iterable(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    for row in iter(&iterable_result.clone()) {
        assert_count(&Value::Int(1), &row.clone());
        assert_array_has_key(&s("user"), &row.clone());
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
            &idx(&row.clone(), &s("user")),
        );
        if truthy(&same(&row_num.clone(), &Value::Int(0))) {
            assert_equals(&Value::Int(1), &prop(&idx(&row.clone(), &s("user")), "id"));
            assert_equals(&s("romanb"), &prop(&idx(&row.clone(), &s("user")), "name"));
        }
        if truthy(&same(&row_num.clone(), &Value::Int(1))) {
            assert_equals(&Value::Int(2), &prop(&idx(&row.clone(), &s("user")), "id"));
            assert_equals(&s("jwage"), &prop(&idx(&row.clone(), &s("user")), "name"));
        }
        row_num = inc(&row_num);
    }
    assert_same(&Value::Int(2), &row_num.clone());
}

#[tokio::test]
async fn testManyToManyHydration() {
    let em = entity_manager().await;
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
    rsm_call(
        &mut rsm,
        "addJoinedEntityResult",
        &[
            s("Doctrine\\Tests\\Models\\CMS\\CmsGroup"),
            s("g"),
            s("u"),
            s("groups"),
        ],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("g"), s("g__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("g"), s("g__name"), s("name")],
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
        "addFieldResult",
        &[s("p"), s("p__phonenumber"), s("phonenumber")],
    );
    let mut result_set = list_of(vec![
        array_of(vec![
            (s("u__id"), s("1")),
            (s("u__name"), s("romanb")),
            (s("g__id"), s("3")),
            (s("g__name"), s("TestGroupB")),
            (s("p__phonenumber"), Value::Int(1111)),
        ]),
        array_of(vec![
            (s("u__id"), s("1")),
            (s("u__name"), s("romanb")),
            (s("g__id"), s("5")),
            (s("g__name"), s("TestGroupD")),
            (s("p__phonenumber"), Value::Int(1111)),
        ]),
        array_of(vec![
            (s("u__id"), s("1")),
            (s("u__name"), s("romanb")),
            (s("g__id"), s("3")),
            (s("g__name"), s("TestGroupB")),
            (s("p__phonenumber"), Value::Int(2222)),
        ]),
        array_of(vec![
            (s("u__id"), s("1")),
            (s("u__name"), s("romanb")),
            (s("g__id"), s("5")),
            (s("g__name"), s("TestGroupD")),
            (s("p__phonenumber"), Value::Int(2222)),
        ]),
        array_of(vec![
            (s("u__id"), s("2")),
            (s("u__name"), s("jwage")),
            (s("g__id"), s("2")),
            (s("g__name"), s("TestGroupA")),
            (s("p__phonenumber"), Value::Int(3333)),
        ]),
        array_of(vec![
            (s("u__id"), s("2")),
            (s("u__name"), s("jwage")),
            (s("g__id"), s("3")),
            (s("g__name"), s("TestGroupB")),
            (s("p__phonenumber"), Value::Int(3333)),
        ]),
        array_of(vec![
            (s("u__id"), s("2")),
            (s("u__name"), s("jwage")),
            (s("g__id"), s("4")),
            (s("g__name"), s("TestGroupC")),
            (s("p__phonenumber"), Value::Int(3333)),
        ]),
        array_of(vec![
            (s("u__id"), s("2")),
            (s("u__name"), s("jwage")),
            (s("g__id"), s("5")),
            (s("g__name"), s("TestGroupD")),
            (s("p__phonenumber"), Value::Int(3333)),
        ]),
        array_of(vec![
            (s("u__id"), s("2")),
            (s("u__name"), s("jwage")),
            (s("g__id"), s("2")),
            (s("g__name"), s("TestGroupA")),
            (s("p__phonenumber"), Value::Int(4444)),
        ]),
        array_of(vec![
            (s("u__id"), s("2")),
            (s("u__name"), s("jwage")),
            (s("g__id"), s("3")),
            (s("g__name"), s("TestGroupB")),
            (s("p__phonenumber"), Value::Int(4444)),
        ]),
        array_of(vec![
            (s("u__id"), s("2")),
            (s("u__name"), s("jwage")),
            (s("g__id"), s("4")),
            (s("g__name"), s("TestGroupC")),
            (s("p__phonenumber"), Value::Int(4444)),
        ]),
        array_of(vec![
            (s("u__id"), s("2")),
            (s("u__name"), s("jwage")),
            (s("g__id"), s("5")),
            (s("g__name"), s("TestGroupD")),
            (s("p__phonenumber"), Value::Int(4444)),
        ]),
    ]);
    let result = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_equals(&Value::Int(2), &php_count(std::slice::from_ref(&result)));
    assert_contains_only(&s("Doctrine\\Tests\\Models\\CMS\\CmsUser"), &result.clone());
    assert_equals(
        &Value::Int(2),
        &php_count(&[prop(&idx(&result.clone(), &Value::Int(0)), "groups")]),
    );
    assert_equals(
        &Value::Int(2),
        &php_count(&[prop(&idx(&result.clone(), &Value::Int(0)), "phonenumbers")]),
    );
    assert_equals(
        &Value::Int(4),
        &php_count(&[prop(&idx(&result.clone(), &Value::Int(1)), "groups")]),
    );
    assert_equals(
        &Value::Int(2),
        &php_count(&[prop(&idx(&result.clone(), &Value::Int(1)), "phonenumbers")]),
    );
}

#[tokio::test]
async fn testManyToManyHydrationWithAliasedUserEntity() {
    let em = entity_manager().await;
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
    rsm_call(
        &mut rsm,
        "addJoinedEntityResult",
        &[
            s("Doctrine\\Tests\\Models\\CMS\\CmsGroup"),
            s("g"),
            s("u"),
            s("groups"),
        ],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("g"), s("g__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("g"), s("g__name"), s("name")],
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
        "addFieldResult",
        &[s("p"), s("p__phonenumber"), s("phonenumber")],
    );
    let mut result_set = list_of(vec![
        array_of(vec![
            (s("u__id"), s("1")),
            (s("u__name"), s("romanb")),
            (s("g__id"), s("3")),
            (s("g__name"), s("TestGroupB")),
            (s("p__phonenumber"), Value::Int(1111)),
        ]),
        array_of(vec![
            (s("u__id"), s("1")),
            (s("u__name"), s("romanb")),
            (s("g__id"), s("5")),
            (s("g__name"), s("TestGroupD")),
            (s("p__phonenumber"), Value::Int(1111)),
        ]),
        array_of(vec![
            (s("u__id"), s("1")),
            (s("u__name"), s("romanb")),
            (s("g__id"), s("3")),
            (s("g__name"), s("TestGroupB")),
            (s("p__phonenumber"), Value::Int(2222)),
        ]),
        array_of(vec![
            (s("u__id"), s("1")),
            (s("u__name"), s("romanb")),
            (s("g__id"), s("5")),
            (s("g__name"), s("TestGroupD")),
            (s("p__phonenumber"), Value::Int(2222)),
        ]),
        array_of(vec![
            (s("u__id"), s("2")),
            (s("u__name"), s("jwage")),
            (s("g__id"), s("2")),
            (s("g__name"), s("TestGroupA")),
            (s("p__phonenumber"), Value::Int(3333)),
        ]),
        array_of(vec![
            (s("u__id"), s("2")),
            (s("u__name"), s("jwage")),
            (s("g__id"), s("3")),
            (s("g__name"), s("TestGroupB")),
            (s("p__phonenumber"), Value::Int(3333)),
        ]),
        array_of(vec![
            (s("u__id"), s("2")),
            (s("u__name"), s("jwage")),
            (s("g__id"), s("4")),
            (s("g__name"), s("TestGroupC")),
            (s("p__phonenumber"), Value::Int(3333)),
        ]),
        array_of(vec![
            (s("u__id"), s("2")),
            (s("u__name"), s("jwage")),
            (s("g__id"), s("5")),
            (s("g__name"), s("TestGroupD")),
            (s("p__phonenumber"), Value::Int(3333)),
        ]),
        array_of(vec![
            (s("u__id"), s("2")),
            (s("u__name"), s("jwage")),
            (s("g__id"), s("2")),
            (s("g__name"), s("TestGroupA")),
            (s("p__phonenumber"), Value::Int(4444)),
        ]),
        array_of(vec![
            (s("u__id"), s("2")),
            (s("u__name"), s("jwage")),
            (s("g__id"), s("3")),
            (s("g__name"), s("TestGroupB")),
            (s("p__phonenumber"), Value::Int(4444)),
        ]),
        array_of(vec![
            (s("u__id"), s("2")),
            (s("u__name"), s("jwage")),
            (s("g__id"), s("4")),
            (s("g__name"), s("TestGroupC")),
            (s("p__phonenumber"), Value::Int(4444)),
        ]),
        array_of(vec![
            (s("u__id"), s("2")),
            (s("u__name"), s("jwage")),
            (s("g__id"), s("5")),
            (s("g__name"), s("TestGroupD")),
            (s("p__phonenumber"), Value::Int(4444)),
        ]),
    ]);
    let result = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_equals(&Value::Int(2), &php_count(std::slice::from_ref(&result)));
    assert_is_array(&idx(&result.clone(), &Value::Int(0)));
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
        &idx(&idx(&result.clone(), &Value::Int(0)), &s("user")),
    );
    assert_is_array(&idx(&result.clone(), &Value::Int(1)));
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
        &idx(&idx(&result.clone(), &Value::Int(1)), &s("user")),
    );
    assert_equals(
        &Value::Int(2),
        &php_count(&[prop(
            &idx(&idx(&result.clone(), &Value::Int(0)), &s("user")),
            "groups",
        )]),
    );
    assert_equals(
        &Value::Int(2),
        &php_count(&[prop(
            &idx(&idx(&result.clone(), &Value::Int(0)), &s("user")),
            "phonenumbers",
        )]),
    );
    assert_equals(
        &Value::Int(4),
        &php_count(&[prop(
            &idx(&idx(&result.clone(), &Value::Int(1)), &s("user")),
            "groups",
        )]),
    );
    assert_equals(
        &Value::Int(2),
        &php_count(&[prop(
            &idx(&idx(&result.clone(), &Value::Int(1)), &s("user")),
            "phonenumbers",
        )]),
    );
}

#[tokio::test]
async fn testMissingIdForRootEntity() {
    let em = entity_manager().await;
    for user_entity_key in [Value::Int(0), s("user")] {
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
        let result = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
            .await
            .unwrap();
        assert_equals(&Value::Int(4), &php_count(std::slice::from_ref(&result)));
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
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
            &idx(
                &idx(&result.clone(), &Value::Int(0)),
                &user_entity_key.clone(),
            ),
        );
        assert_null(&idx(
            &idx(&result.clone(), &Value::Int(1)),
            &user_entity_key.clone(),
        ));
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsUser"),
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
async fn testMissingIdForCollectionValuedChildEntity() {
    let em = entity_manager().await;
    for user_entity_key in [Value::Int(0), s("user")] {
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
                (s("p__phonenumber"), Value::Null),
            ]),
            array_of(vec![
                (s("u__id"), s("2")),
                (s("u__status"), s("developer")),
                (s("sclr0"), s("JWAGE")),
                (s("p__phonenumber"), s("91")),
            ]),
            array_of(vec![
                (s("u__id"), s("2")),
                (s("u__status"), s("developer")),
                (s("sclr0"), s("JWAGE")),
                (s("p__phonenumber"), Value::Null),
            ]),
        ]);
        let result = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
            .await
            .unwrap();
        assert_equals(&Value::Int(2), &php_count(std::slice::from_ref(&result)));
        assert_equals(
            &Value::Int(1),
            &call(
                &prop(
                    &idx(
                        &idx(&result.clone(), &Value::Int(0)),
                        &user_entity_key.clone(),
                    ),
                    "phonenumbers",
                ),
                "count",
                &[],
            ),
        );
        assert_equals(
            &Value::Int(1),
            &call(
                &prop(
                    &idx(
                        &idx(&result.clone(), &Value::Int(1)),
                        &user_entity_key.clone(),
                    ),
                    "phonenumbers",
                ),
                "count",
                &[],
            ),
        );
    }
}

#[tokio::test]
async fn testMissingIdForSingleValuedChildEntity() {
    let em = entity_manager().await;
    for user_entity_key in [Value::Int(0), s("user")] {
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
                s("Doctrine\\Tests\\Models\\CMS\\CmsAddress"),
                s("a"),
                s("u"),
                s("address"),
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
        rsm_call(&mut rsm, "addFieldResult", &[s("a"), s("a__id"), s("id")]);
        rsm_call(
            &mut rsm,
            "addFieldResult",
            &[s("a"), s("a__city"), s("city")],
        );
        rsm_call(
            &mut rsm,
            "addMetaResult",
            &[
                s("a"),
                s("user_id"),
                s("user_id"),
                Value::Bool(false),
                s("string"),
            ],
        );
        let mut result_set = list_of(vec![
            array_of(vec![
                (s("u__id"), s("1")),
                (s("u__status"), s("developer")),
                (s("sclr0"), s("ROMANB")),
                (s("a__id"), Value::Int(1)),
                (s("a__city"), s("Berlin")),
            ]),
            array_of(vec![
                (s("u__id"), s("2")),
                (s("u__status"), s("developer")),
                (s("sclr0"), s("BENJAMIN")),
                (s("a__id"), Value::Null),
                (s("a__city"), Value::Null),
            ]),
        ]);
        let result = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
            .await
            .unwrap();
        assert_equals(&Value::Int(2), &php_count(std::slice::from_ref(&result)));
        assert_instance_of(
            &s("Doctrine\\Tests\\Models\\CMS\\CmsAddress"),
            &prop(
                &idx(
                    &idx(&result.clone(), &Value::Int(0)),
                    &user_entity_key.clone(),
                ),
                "address",
            ),
        );
        assert_null(&prop(
            &idx(
                &idx(&result.clone(), &Value::Int(1)),
                &user_entity_key.clone(),
            ),
            "address",
        ));
    }
}

#[tokio::test]
async fn testIndexByAndMixedResult() {
    let em = entity_manager().await;
    for user_entity_key in [Value::Int(0), s("user")] {
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
        let result = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
            .await
            .unwrap();
        assert_equals(&Value::Int(2), &php_count(std::slice::from_ref(&result)));
        assert_array_has_key(&Value::Int(1), &result.clone());
        assert_equals(
            &Value::Int(1),
            &prop(
                &idx(
                    &idx(&result.clone(), &Value::Int(1)),
                    &user_entity_key.clone(),
                ),
                "id",
            ),
        );
        assert_array_has_key(&Value::Int(2), &result.clone());
        assert_equals(
            &Value::Int(2),
            &prop(
                &idx(
                    &idx(&result.clone(), &Value::Int(2)),
                    &user_entity_key.clone(),
                ),
                "id",
            ),
        );
    }
}

#[tokio::test]
async fn testIndexByScalarsOnly() {
    let em = entity_manager().await;
    for user_entity_key in [Value::Int(0), s("user")] {
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
            "addScalarResult",
            &[s("sclr0"), s("nameUpper"), s("string")],
        );
        rsm_call(&mut rsm, "addIndexByScalar", &[s("sclr0")]);
        let mut result_set = list_of(vec![
            array_of(vec![(s("sclr0"), s("ROMANB"))]),
            array_of(vec![(s("sclr0"), s("JWAGE"))]),
        ]);
        let result = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
            .await
            .unwrap();
        assert_equals(
            &array_of(vec![
                (s("ROMANB"), array_of(vec![(s("nameUpper"), s("ROMANB"))])),
                (s("JWAGE"), array_of(vec![(s("nameUpper"), s("JWAGE"))])),
            ]),
            &result.clone(),
        );
    }
}

#[tokio::test]
async fn testMissingMetaMappingException() {
    let em = entity_manager().await;
    let mut rsm = ResultSetMapping::new();
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[
            s("Doctrine\\Tests\\Models\\Company\\CompanyFixContract"),
            s("c"),
        ],
    );
    rsm_call(
        &mut rsm,
        "addJoinedEntityResult",
        &[
            s("Doctrine\\Tests\\Models\\Company\\CompanyEmployee"),
            s("e"),
            s("c"),
            s("salesPerson"),
        ],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("c"), s("c__id"), s("id")]);
    rsm_call(&mut rsm, "setDiscriminatorColumn", &[s("c"), s("c_discr")]);
    let mut result_set = list_of(vec![array_of(vec![
        (s("c__id"), s("1")),
        (s("c_discr"), s("fix")),
    ])]);
    let e = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap_err();
    assert_eq!(
        s(&e.class),
        s("Doctrine\\ORM\\Internal\\Hydration\\HydrationException")
    );
    assert_eq!(
        s(&orm::message(&e)),
        s(
            "The meta mapping for the discriminator column \"c_discr\" is missing for \"Doctrine\\Tests\\Models\\Company\\CompanyFixContract\" using the DQL alias \"c\"."
        )
    );
}

#[tokio::test]
async fn testMissingDiscriminatorColumnException() {
    let em = entity_manager().await;
    let mut rsm = ResultSetMapping::new();
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[
            s("Doctrine\\Tests\\Models\\Company\\CompanyFixContract"),
            s("c"),
        ],
    );
    rsm_call(
        &mut rsm,
        "addJoinedEntityResult",
        &[
            s("Doctrine\\Tests\\Models\\Company\\CompanyEmployee"),
            s("e"),
            s("c"),
            s("salesPerson"),
        ],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("c"), s("c__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addMetaResult",
        &[
            s("c"),
            s("c_discr"),
            s("discr"),
            Value::Bool(false),
            s("string"),
        ],
    );
    rsm_call(&mut rsm, "setDiscriminatorColumn", &[s("c"), s("c_discr")]);
    rsm_call(&mut rsm, "addFieldResult", &[s("e"), s("e__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("e"), s("e__name"), s("name")],
    );
    rsm_call(
        &mut rsm,
        "addMetaResult",
        &[
            s("e "),
            s("e_discr"),
            s("discr"),
            Value::Bool(false),
            s("string"),
        ],
    );
    rsm_call(&mut rsm, "setDiscriminatorColumn", &[s("e"), s("e_discr")]);
    let mut result_set = list_of(vec![array_of(vec![
        (s("c__id"), s("1")),
        (s("c_discr"), s("fix")),
        (s("e__id"), s("1")),
        (s("e__name"), s("Fabio B. Silva")),
    ])]);
    let e = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap_err();
    assert_eq!(
        s(&e.class),
        s("Doctrine\\ORM\\Internal\\Hydration\\HydrationException")
    );
    assert_eq!(
        s(&orm::message(&e)),
        s(
            "The discriminator column \"discr\" is missing for \"Doctrine\\Tests\\Models\\Company\\CompanyEmployee\" using the DQL alias \"e\"."
        )
    );
}

#[tokio::test]
async fn testInvalidDiscriminatorValueException() {
    let em = entity_manager().await;
    let mut rsm = ResultSetMapping::new();
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[s("Doctrine\\Tests\\Models\\Company\\CompanyPerson"), s("p")],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("p"), s("p__id"), s("id")]);
    rsm_call(
        &mut rsm,
        "addFieldResult",
        &[s("p"), s("p__name"), s("name")],
    );
    rsm_call(
        &mut rsm,
        "addMetaResult",
        &[
            s("p"),
            s("discr"),
            s("discr"),
            Value::Bool(false),
            s("string"),
        ],
    );
    rsm_call(&mut rsm, "setDiscriminatorColumn", &[s("p"), s("discr")]);
    let mut result_set = list_of(vec![array_of(vec![
        (s("p__id"), s("1")),
        (s("p__name"), s("Fabio B. Silva")),
        (s("discr"), s("subworker")),
    ])]);
    let e = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap_err();
    assert_eq!(
        s(&e.class),
        s("Doctrine\\ORM\\Internal\\Hydration\\HydrationException")
    );
    assert_eq!(
        s(&orm::message(&e)),
        s(
            "The discriminator value \"subworker\" is invalid. It must be one of \"person\", \"manager\", \"employee\"."
        )
    );
}

#[tokio::test]
async fn testFetchJoinCollectionValuedAssociationWithDefaultArrayValue() {
    let em = entity_manager().await;
    let mut rsm = ResultSetMapping::new();
    rsm_call(
        &mut rsm,
        "addEntityResult",
        &[
            s("Doctrine\\Tests\\Models\\Hydration\\EntityWithArrayDefaultArrayValueM2M"),
            s("e1"),
            Value::Null,
        ],
    );
    rsm_call(
        &mut rsm,
        "addJoinedEntityResult",
        &[
            s("Doctrine\\Tests\\Models\\Hydration\\SimpleEntity"),
            s("e2"),
            s("e1"),
            s("collection"),
        ],
    );
    rsm_call(&mut rsm, "addFieldResult", &[s("e1"), s("a1__id"), s("id")]);
    rsm_call(&mut rsm, "addFieldResult", &[s("e2"), s("e2__id"), s("id")]);
    let mut result_set = list_of(vec![array_of(vec![
        (s("a1__id"), s("1")),
        (s("e2__id"), s("1")),
    ])]);
    let result = php_hydrate_all(&em, HYDRATE_OBJECT, &result_set, &rsm, &Value::Null)
        .await
        .unwrap();
    assert_count(&Value::Int(1), &result.clone());
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\Hydration\\EntityWithArrayDefaultArrayValueM2M"),
        &idx(&result.clone(), &Value::Int(0)),
    );
    assert_instance_of(
        &s("Doctrine\\ORM\\PersistentCollection"),
        &prop(&idx(&result.clone(), &Value::Int(0)), "collection"),
    );
    assert_count(
        &Value::Int(1),
        &prop(&idx(&result.clone(), &Value::Int(0)), "collection"),
    );
    assert_instance_of(
        &s("Doctrine\\Tests\\Models\\Hydration\\SimpleEntity"),
        &idx(
            &prop(&idx(&result.clone(), &Value::Int(0)), "collection"),
            &Value::Int(0),
        ),
    );
}
