//! Ticket/DDC1918Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the cms models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1918Test.php

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

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::tools::pagination::{OffsetPaginator, Page, Window};

use orm::*;

#[tokio::test]
async fn testLastPageCorrect() {
    let em = orm::set_up(&CMS_MODELS).await;

    let mut groups = Vec::new();
    for _ in 0..3 {
        let group = cms_group("test");
        em.persist(&group).unwrap();
        groups.push(Value::Object(group));
    }

    for i in 0..10 {
        let user = cms_user_with(&format!("user{i}"), &format!("user{i}"), "active");
        set(
            &user,
            "groups",
            Value::Array(PhpArray::list(groups.clone())),
        );
        em.persist(&user).unwrap();
    }

    em.flush().await.unwrap();

    let query =
        em.create_query("SELECT u, g FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u JOIN u.groups g");

    let paginator = OffsetPaginator::new(true, None);
    for (first, expected) in [(6, 3), (8, 2), (10, 0)] {
        let page = paginator
            .paginate_window((&query).into(), Window::new(first, 3).unwrap())
            .await
            .unwrap();
        assert_eq!(expected, page.count());
    }
}
