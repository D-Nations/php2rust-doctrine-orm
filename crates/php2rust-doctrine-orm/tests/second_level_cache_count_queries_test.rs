//! Upstream: doctrine/orm tests/Tests/ORM/Functional/SecondLevelCacheCountQueriesTest.php
//!
//! A test runs the rows of `cacheUsageProvider()` in turn, each on a new
//! fixture. The `FileLockRegion` directory of a `READ_WRITE` row is
//! under the system temporary directory, named after the class, the test
//! and the row (PHP's tests run one at a time and share one directory).

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

use std::path::PathBuf;

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::id::IdGenerator;
use php2rust_doctrine_orm::mapping::class_metadata::{
    CACHE_USAGE_NONSTRICT_READ_WRITE, CACHE_USAGE_READ_WRITE,
};

use slc_functional::{COUNTRY, Fixture, id, set_up};

/// `cacheUsageProvider()`.
const CACHE_USAGES: [i64; 3] = [0, CACHE_USAGE_NONSTRICT_READ_WRITE, CACHE_USAGE_READ_WRITE];

/// A fixture and its `$tmpDir`, removed on drop (`tearDown()`).
struct Test {
    f: Fixture,
    tmp_dir: Option<PathBuf>,
}

impl Drop for Test {
    fn drop(&mut self) {
        if let Some(tmp_dir) = &self.tmp_dir {
            let _ = std::fs::remove_dir_all(tmp_dir);
        }
    }
}

/// `setUp()` and `setupCountryModel($cacheUsage)`.
async fn set_up_country_model(test: &str, cache_usage: i64) -> Test {
    let f = set_up().await;
    let metadata = f.em.get_class_metadata(COUNTRY).unwrap();

    if cache_usage == 0 {
        metadata.write().cache = None;
        return Test { f, tmp_dir: None };
    }

    let mut tmp_dir = None;
    if cache_usage == CACHE_USAGE_READ_WRITE {
        let dir = std::env::temp_dir().join(format!(
            "SecondLevelCacheCountQueriesTest-{test}-{}",
            std::process::id()
        ));
        f.factory
            .set_file_lock_region_directory(&dir.to_string_lossy());
        tmp_dir = Some(dir);
    }

    let mut cache = PhpArray::new();
    cache.insert("usage", Value::Int(cache_usage));
    metadata.write().enable_cache(cache);
    Test { f, tmp_dir }
}

impl Test {
    async fn load_fixtures_countries_without_post_insert_identifier(&mut self) {
        let metadata = self.f.em.get_class_metadata(COUNTRY).unwrap();
        metadata.write().set_id_generator(IdGenerator::Assigned);

        let c1 = slc::country("Brazil");
        orm::set(&c1, "id", Value::Int(10));
        let c2 = slc::country("Germany");
        orm::set(&c2, "id", Value::Int(20));

        self.f.countries.push(c1.clone());
        self.f.countries.push(c2.clone());

        self.f.em.persist(&c1).unwrap();
        self.f.em.persist(&c2).unwrap();
        self.f.em.flush().await.unwrap();
    }

    /// `assertQueryCountByType($type, $expectedCount)`.
    #[track_caller]
    fn assert_query_count_by_type(&self, r#type: &str, expected_count: usize) {
        let queries: Vec<String> = self
            .f
            .log
            .queries()
            .into_iter()
            .filter(|sql| sql.starts_with(r#type))
            .collect();
        assert_eq!(expected_count, queries.len(), "{queries:#?}");
    }
}

#[tokio::test]
async fn testInsertWithPostInsertIdentifier() {
    for cache_usage in CACHE_USAGES {
        let mut t = set_up_country_model("insert-post", cache_usage).await;

        t.assert_query_count_by_type("INSERT", 0);

        t.f.load_fixtures_countries().await;

        assert_eq!(2, t.f.countries.len());
        t.assert_query_count_by_type("INSERT", 2);
    }
}

#[tokio::test]
async fn testInsertWithoutPostInsertIdentifier() {
    for cache_usage in CACHE_USAGES {
        let mut t = set_up_country_model("insert", cache_usage).await;

        t.assert_query_count_by_type("INSERT", 0);

        t.load_fixtures_countries_without_post_insert_identifier()
            .await;

        assert_eq!(2, t.f.countries.len());
        t.assert_query_count_by_type("INSERT", 2);
    }
}

#[tokio::test]
async fn testDelete() {
    for cache_usage in CACHE_USAGES {
        let mut t = set_up_country_model("delete", cache_usage).await;
        t.f.load_fixtures_countries().await;

        let c1 = t.f.find(COUNTRY, id(&t.f.countries[0])).await.unwrap();
        let c2 = t.f.find(COUNTRY, id(&t.f.countries[1])).await.unwrap();

        t.f.em.remove(&c1).await.unwrap();
        t.f.em.remove(&c2).await.unwrap();
        t.f.em.flush().await.unwrap();

        t.assert_query_count_by_type("DELETE", 2);
    }
}

#[tokio::test]
async fn testUpdate() {
    for cache_usage in CACHE_USAGES {
        let mut t = set_up_country_model("update", cache_usage).await;
        t.f.load_fixtures_countries().await;

        let c1 = t.f.find(COUNTRY, id(&t.f.countries[0])).await.unwrap();
        let c2 = t.f.find(COUNTRY, id(&t.f.countries[1])).await.unwrap();

        orm::set(&c1, "name", Value::string("Czech Republic"));
        orm::set(&c2, "name", Value::string("Hungary"));

        t.f.em.persist(&c1).unwrap();
        t.f.em.persist(&c2).unwrap();
        t.f.em.flush().await.unwrap();

        t.assert_query_count_by_type("UPDATE", 2);
    }
}
