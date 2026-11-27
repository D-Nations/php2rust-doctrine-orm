//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2943Test.php
//!
//! `enableSecondLevelCache()` and `useModelSet('cache')`: the second-level
//! cache fixture of `tests/common/slc_functional.rs`.

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
#[path = "common/query_log.rs"]
mod query_log;
#[path = "common/slc.rs"]
mod slc;
#[path = "common/slc_functional.rs"]
mod slc_functional;

use php2rust_doctrine_orm::cache::DEFAULT_QUERY_REGION_NAME;
use php2rust_doctrine_orm::tools::pagination::{OffsetPaginator, Page, Window, WindowPage};

use slc_functional::{COUNTRY, Fixture};

async fn load_fixtures(f: &Fixture) {
    for name in ["Brazil", "Canada", "Germany", "France"] {
        f.em.persist(&slc::country(name)).unwrap();
    }
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();
}

fn assert_paginator_query_put(
    f: &Fixture,
    page: &WindowPage,
    region_name: &str,
    count: i64,
    page_size: usize,
) {
    assert_eq!(count, page.get_total_count());
    assert_eq!(page_size, page.count());

    assert_eq!(0, f.logger.get_region_hit_count(DEFAULT_QUERY_REGION_NAME));
    assert_eq!(1, f.logger.get_region_put_count(DEFAULT_QUERY_REGION_NAME));
    assert_eq!(0, f.logger.get_region_hit_count(region_name));
    assert_eq!(count, f.logger.get_region_put_count(region_name));
}

fn assert_paginator_query_hit(
    f: &Fixture,
    page: &WindowPage,
    region_name: &str,
    count: i64,
    page_size: usize,
) {
    assert_eq!(count, page.get_total_count());
    assert_eq!(page_size, page.count());

    assert_eq!(1, f.logger.get_region_hit_count(DEFAULT_QUERY_REGION_NAME));
    assert_eq!(0, f.logger.get_region_put_count(DEFAULT_QUERY_REGION_NAME));
    assert_eq!(
        i64::try_from(page_size).unwrap(),
        f.logger.get_region_hit_count(region_name)
    );
    assert_eq!(0, f.logger.get_region_put_count(region_name));
}

async fn issue(fetch_join_collection: bool) {
    let f = slc_functional::set_up().await;
    load_fixtures(&f).await;

    let region =
        f.em.get_cache()
            .unwrap()
            .get_entity_cache_region(COUNTRY)
            .unwrap()
            .unwrap();
    let dql = "SELECT c FROM Doctrine\\Tests\\Models\\Cache\\Country c";
    let query = f.em.create_query(dql);
    query.set_cacheable(true);

    let paginator = OffsetPaginator::new(fetch_join_collection, None);
    let window = Window::new(0, 2).unwrap();

    let page = paginator
        .paginate_window((&query).into(), window)
        .await
        .unwrap();
    assert_paginator_query_put(&f, &page, &region.get_name(), 4, 2);

    f.em.clear().unwrap();
    f.logger.clear_stats();

    let page = paginator
        .paginate_window((&query).into(), window)
        .await
        .unwrap();
    assert_paginator_query_hit(&f, &page, &region.get_name(), 4, 2);
}

#[tokio::test]
async fn testIssue() {
    issue(true).await;
}

#[tokio::test]
async fn testIssueNonFetchJoin() {
    issue(false).await;
}
