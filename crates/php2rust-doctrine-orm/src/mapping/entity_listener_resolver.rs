//! `EntityListenerResolver` and `DefaultEntityListenerResolver`: the
//! entity listener instances by class name. `new $className()` is the
//! listener object with its properties' defaults ([`EntityObject`]).
//!
//! Upstream: doctrine/orm src/Mapping/EntityListenerResolver.php src/Mapping/DefaultEntityListenerResolver.php

use std::fmt;
use std::sync::{Mutex, PoisonError};

use php_value::{Object, Thrown};

use crate::object::EntityObject;

/// A resolver is used to instantiate an entity listener.
pub trait EntityListenerResolver: Send + Sync + fmt::Debug {
    /// Clear all instances from the set, or a specific instance when given
    /// its identifier.
    ///
    /// `class_name` may be any arbitrary string. Name kept for BC only.
    fn clear(&self, class_name: Option<&str>);

    /// Returns a entity listener instance for the given identifier.
    ///
    /// `class_name` may be any arbitrary string. Name kept for BC only.
    fn resolve(&self, class_name: &str) -> Result<Object, Thrown>;

    /// Register a entity listener instance.
    fn register(&self, object: Object);
}

/// The default `EntityListenerResolver`.
#[derive(Debug, Default)]
pub struct DefaultEntityListenerResolver {
    /// Map to store entity listener instances.
    instances: Mutex<Vec<(String, Object)>>,
}

impl DefaultEntityListenerResolver {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Mapping\\DefaultEntityListenerResolver";

    pub fn new() -> Self {
        Self::default()
    }
}

impl EntityListenerResolver for DefaultEntityListenerResolver {
    fn clear(&self, class_name: Option<&str>) {
        let mut instances = self
            .instances
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let Some(class_name) = class_name else {
            instances.clear();

            return;
        };

        let class_name = class_name.trim_matches('\\');
        instances.retain(|(c, _)| c != class_name);
    }

    fn register(&self, object: Object) {
        let mut instances = self
            .instances
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let class = object.class().to_owned();
        match instances.iter_mut().find(|(c, _)| *c == class) {
            Some(entry) => entry.1 = object,
            None => instances.push((class, object)),
        }
    }

    fn resolve(&self, class_name: &str) -> Result<Object, Thrown> {
        let class_name = class_name.trim_matches('\\');
        let mut instances = self
            .instances
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some((_, instance)) = instances.iter().find(|(c, _)| c == class_name) {
            return Ok(instance.clone());
        }
        let instance = EntityObject::object(class_name)
            .ok_or_else(|| Thrown::new("Error", format!("Class \"{class_name}\" not found"), 0))?;
        instances.push((class_name.to_owned(), instance.clone()));
        Ok(instance)
    }
}
