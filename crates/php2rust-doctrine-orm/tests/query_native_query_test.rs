//! Upstream: doctrine/orm tests/Tests/ORM/Query/NativeQueryTest.php
//!
//! The test entity manager runs on the `UnitOfWorkTest` driver stub,
//! whose statements give no rows (as upstream's mocked connection). The
//! unit of work is not a mock: the parameter of a specified type is
//! bound as given, and the query finds nothing.

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
#[path = "common/stub_connection.rs"]
mod stub_connection;

use php_value::datetime::{DateTime, DateTimeImmutable, TimeZone};
use php2rust_doctrine_dbal::WrapperParameterType;
use php2rust_doctrine_orm::query::ResultSetMapping;
use php2rust_doctrine_orm::query::context::HYDRATE_OBJECT;

#[tokio::test]
async fn testValuesAreNotBeingResolvedForSpecifiedParameterTypes() {
    let em = orm::entity_manager(
        stub_connection::stub_connection(stub_connection::Failures::default()),
        None,
    );
    let rsm = ResultSetMapping::new();

    let query = em
        .create_native_query(
            "SELECT d.* FROM date_time_model d WHERE d.datetime = :value",
            rsm,
        )
        .unwrap();

    query.set_parameter(
        "value",
        DateTime::object(DateTimeImmutable::now(TimeZone::Utc)),
        Some(WrapperParameterType::Name("datetime".into())),
    );

    let result = query.get_result(HYDRATE_OBJECT).await.unwrap();
    assert!(!result.is_truthy());
}
