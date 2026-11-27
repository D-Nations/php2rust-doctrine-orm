//! Port of tests/Tests/ORM/Hydration/CustomHydratorTest.php. The
//! `CustomHydrator` class is an `AbstractHydrator` implementation
//! registered under its class name.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Hydration/CustomHydratorTest.php

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

use std::sync::Arc;

use php_value::{PhpArray, Thrown, Value};
use php2rust_doctrine_orm::internal::hydration::{
    AbstractHydrator, HydrationContext, register_hydrator_class,
};

const CUSTOM_HYDRATOR: &str = "Doctrine\\Tests\\ORM\\Hydration\\CustomHydrator";

/// `CustomHydrator`.
struct CustomHydrator;

impl AbstractHydrator for CustomHydrator {
    fn class(&self) -> &'static str {
        CUSTOM_HYDRATOR
    }

    fn hydrate_all_data(
        &mut self,
        context: &mut HydrationContext<'_, '_>,
    ) -> Result<Value, Thrown> {
        let rows = context
            .statement()
            .fetch_all_associative()
            .map_err(|e| Thrown::new(e.class(), e.message().clone(), 0))?;
        Ok(Value::Array(PhpArray::list(
            rows.into_iter().map(Value::Array),
        )))
    }
}

#[test]
fn testCustomHydrator() {
    register_hydrator_class(CUSTOM_HYDRATOR, Arc::new(|_em| Box::new(CustomHydrator)));

    // $em->getConfiguration()->addCustomHydrationMode(...): the port's
    // configuration is set before the entity manager is created.
    let em = orm::entity_manager_with(orm::connection(), |config| {
        config.add_custom_hydration_mode("CustomHydrator", CUSTOM_HYDRATOR);
    });

    let hydrator = em.new_hydrator("CustomHydrator").unwrap();
    assert_eq!(CUSTOM_HYDRATOR, hydrator.class());
    assert!(
        em.get_configuration()
            .get_custom_hydration_mode("does not exist")
            .is_none()
    );
}
