//! OffsetPaginatorTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on a driver connection that records the parameters of each query
//! (`tests/common/recording_connection.rs`). Upstream stubs the hydrator:
//! here the real hydrators read the rows the connection gives (the
//! identifier row `[10]` for the identifier query, no rows otherwise),
//! which the assertions do not depend on. The connection sees the
//! parameters after DBAL expands the array parameter of the WHERE IN
//! query, so `[$paramInSubSelect, $paramInWhere, $returnedIds]` is
//! `[2, 1, 10]`. `executeQuery()`'s expected call counts are the counts of
//! recorded queries.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Tools/Pagination/OffsetPaginatorTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;
#[path = "common/recording_connection.rs"]
mod recording_connection;

use std::sync::Arc;

use php_value::{PhpArray, Thrown, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::query::Query;
use php2rust_doctrine_orm::tools::pagination::{OffsetPaginator, Page, Window, WindowPage};

use recording_connection::{SharedRecorder, recording_connection, row};

struct SetUp {
    em: Arc<EntityManager>,
    recorder: SharedRecorder,
}

fn set_up() -> SetUp {
    let (connection, recorder) = recording_connection();
    SetUp {
        em: orm::entity_manager(connection, None),
        recorder,
    }
}

fn params(entries: &[(&str, i64)]) -> PhpArray {
    let mut params = PhpArray::new();
    for (name, value) in entries {
        params.insert(*name, Value::Int(*value));
    }
    params
}

fn received(recorder: &SharedRecorder) -> Vec<Vec<i64>> {
    recorder
        .lock()
        .unwrap()
        .received_params
        .iter()
        .map(|params| {
            params
                .iter()
                .map(|v| match v {
                    Value::Int(i) => *i,
                    other => panic!("not an int: {other:?}"),
                })
                .collect()
        })
        .collect()
}

#[tokio::test]
async fn testExtraParametersAreStrippedWhenWalkerRemovingOriginalSelectElementsIsUsed() {
    let t = set_up();
    let param_in_where = 1;
    let param_in_sub_select = 2;
    let returned_ids = 10;

    // The identifier query gets the identifier, the others no rows.
    t.recorder.lock().unwrap().rows = vec![vec![row("id_0", returned_ids)]].into();

    let query = Query::new(&t.em);
    query.set_dql(
        "SELECT u,
                (
                    SELECT MAX(a.version)
                    FROM Doctrine\\Tests\\Models\\CMS\\CmsArticle a
                    WHERE a.user = u AND 1 = :paramInSubSelect
                ) AS HIDDEN max_version
            FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u
            WHERE u.id = :paramInWhere",
    );
    query.set_parameters_array(&params(&[
        ("paramInWhere", param_in_where),
        ("paramInSubSelect", param_in_sub_select),
    ]));

    OffsetPaginator::new(true, Some(false))
        .paginate_window((&query).into(), Window::new(0, 1).unwrap())
        .await
        .unwrap();

    assert_eq!(
        vec![
            vec![param_in_where],
            vec![param_in_sub_select, param_in_where, returned_ids],
            vec![param_in_where],
        ],
        received(&t.recorder)
    );
}

#[tokio::test]
async fn testPaginatingDoesNotCareAboutExtraParametersWithoutOutputWalkersWhenResultIsEmpty() {
    let t = set_up();

    let page = paginate_with_extra_parameters_without_output_walkers(&t)
        .await
        .unwrap();

    assert_eq!(0, page.count());
    // exactly(2) with(anything(), [])
    assert_eq!(vec![Vec::<i64>::new(); 2], received(&t.recorder));
}

#[tokio::test]
async fn testPaginatingDoesCareAboutExtraParametersWithoutOutputWalkersWhenResultIsNotEmpty() {
    let t = set_up();
    t.recorder.lock().unwrap().rows = vec![vec![row("id_0", 10)]].into();

    let e = paginate_with_extra_parameters_without_output_walkers(&t)
        .await
        .unwrap_err();

    assert_eq!("Doctrine\\ORM\\Query\\QueryException", e.class);
    assert_eq!(
        "Too many parameters: the query defines 1 parameters and you bound 2",
        orm::message(&e)
    );
    // exactly(1) with(anything(), [])
    assert_eq!(vec![Vec::<i64>::new()], received(&t.recorder));
}

async fn paginate_with_extra_parameters_without_output_walkers(
    t: &SetUp,
) -> Result<WindowPage, Thrown> {
    let query = Query::new(&t.em);
    query.set_dql("SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u");
    query.set_parameters_array(&params(&[("paramInWhere", 1)]));

    OffsetPaginator::new(true, Some(false))
        .paginate_window((&query).into(), Window::new(0, 1).unwrap())
        .await
}
