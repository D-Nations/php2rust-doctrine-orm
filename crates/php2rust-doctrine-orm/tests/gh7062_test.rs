//! Ticket/GH7062Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH7062Test.php

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

use std::sync::Arc;

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;
use ticket::*;

const SEASON_ID: &str = "season_18";
const TEAM_ID: &str = "team_A";

/// `new $short($id)`.
fn with_id(short: &str, id: &str) -> Object {
    let entity = new(short);
    set(&entity, "id", s(id));
    entity
}

/// `new GH7062Ranking($season, $teams)`.
fn ranking(season: &Object, teams: &[&Object]) -> Object {
    let ranking = new("GH7062Ranking");
    set_obj(&ranking, "season", season);
    set(
        &ranking,
        "positions",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    for team in teams {
        // new GH7062RankingPosition($this, $team)
        let position = new("GH7062RankingPosition");
        set_obj(&position, "ranking", &ranking);
        set_obj(&position, "team", team);
        set(&position, "points", Value::Int(0));
        coll::add(&get(&ranking, "positions"), Value::Object(position)).unwrap();
    }
    ranking
}

async fn positions(ranking: &Object) -> Vec<Object> {
    coll::get_values(&get(ranking, "positions"))
        .await
        .unwrap()
        .iter()
        .map(obj)
        .collect()
}

#[tokio::test]
async fn testEntityWithAssociationKeyIdentityCanBeUpdated() {
    let em = ticket::set_up(&[
        "GH7062Team",
        "GH7062Season",
        "GH7062Ranking",
        "GH7062RankingPosition",
    ])
    .await;
    create_initial_ranking_with_related_entities(&em).await;
    modify_ranking(&em).await;
    verify_ranking(&em).await;
}

async fn create_initial_ranking_with_related_entities(em: &Arc<EntityManager>) {
    let team = with_id("GH7062Team", TEAM_ID);
    let season = with_id("GH7062Season", SEASON_ID);

    set_obj(&season, "ranking", &ranking(&season, &[&team]));

    em.persist(&team).unwrap();
    em.persist(&season).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    for position in positions(&obj(&get(&season, "ranking"))).await {
        assert!(Value::Int(0).identical(&get(&position, "points")));
    }
}

async fn modify_ranking(em: &Arc<EntityManager>) {
    let ranking = find(em, "GH7062Ranking", s(SEASON_ID)).await.unwrap();

    for position in positions(&ranking).await {
        let Value::Int(points) = get(&position, "points") else {
            panic!("an int");
        };
        set(&position, "points", Value::Int(points + 3));
    }

    em.flush().await.unwrap();
    em.clear().unwrap();
}

async fn verify_ranking(em: &Arc<EntityManager>) {
    let season = find(em, "GH7062Season", s(SEASON_ID)).await.unwrap();
    assert_instance_of("GH7062Season", &Value::Object(season.clone()));

    let ranking = get(&season, "ranking");
    assert_instance_of("GH7062Ranking", &ranking);

    for position in positions(&obj(&ranking)).await {
        assert!(Value::Int(3).identical(&get(&position, "points")));
    }
}
