//! `Expr\OrderBy`.
//!
//! Upstream: doctrine/orm src/Query/Expr/OrderBy.php

use std::sync::{Mutex, PoisonError};

use php_value::{PhpArray, PhpString, Thrown, Value};

use super::php::expression_object;
use crate::query::SortDirection;
use crate::query::deprecations;

/// The deprecation of an `$order` that is not a `SortDirection`.
pub const ORDER_DEPRECATION: &str = "https://github.com/doctrine/orm/issues/11313";

/// `SortDirection|string|null $order`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Order {
    Direction(SortDirection),
    Name(String),
    Null,
}

impl Order {
    /// `get_debug_type($order)`.
    fn debug_type(&self) -> &'static str {
        match self {
            Self::Direction(_) => SortDirection::CLASS,
            Self::Name(_) => "string",
            Self::Null => "null",
        }
    }
}

impl From<SortDirection> for Order {
    fn from(direction: SortDirection) -> Self {
        Self::Direction(direction)
    }
}

impl From<&str> for Order {
    fn from(name: &str) -> Self {
        Self::Name(name.to_owned())
    }
}

impl From<String> for Order {
    fn from(name: String) -> Self {
        Self::Name(name)
    }
}

impl<T: Into<Order>> From<Option<T>> for Order {
    fn from(order: Option<T>) -> Self {
        order.map_or(Self::Null, Into::into)
    }
}

/// Expression class for building DQL Order By parts.
#[derive(Debug)]
pub struct OrderBy {
    parts: Mutex<Vec<String>>,
}

impl Clone for OrderBy {
    fn clone(&self) -> Self {
        Self {
            parts: Mutex::new(self.get_parts()),
        }
    }
}

fn trigger_deprecation(order: &Order, method: &str) {
    deprecations::trigger(ORDER_DEPRECATION, || {
        format!(
            "Passing {} as $order to {method} is deprecated, use an instance of SortDirection instead.",
            order.debug_type()
        )
    });
}

impl OrderBy {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Query\\Expr\\OrderBy";

    const PRE_SEPARATOR: &'static str = "";
    const SEPARATOR: &'static str = ", ";
    const POST_SEPARATOR: &'static str = "";

    /// `new OrderBy($sort, $order)`; upstream's default order is
    /// `SortDirection::Ascending`.
    pub fn new(sort: Option<&str>, order: impl Into<Order>) -> Self {
        let order = order.into();
        if !matches!(order, Order::Direction(_)) {
            trigger_deprecation(&order, "Doctrine\\ORM\\Query\\Expr\\OrderBy::__construct");
        }
        let order_by = Self {
            parts: Mutex::new(Vec::new()),
        };
        if let Some(sort) = sort.filter(|s| super::php::string_is_truthy(s)) {
            order_by.add(sort, Some(order));
        }
        order_by
    }

    /// `add($sort, $order)`; `None` is a call without `$order`.
    pub fn add(&self, sort: &str, order: Option<Order>) {
        if let Some(order) = order.as_ref().filter(|o| !matches!(o, Order::Direction(_))) {
            trigger_deprecation(order, "Doctrine\\ORM\\Query\\Expr\\OrderBy::add");
        }
        let order = match order {
            Some(Order::Direction(SortDirection::Descending)) => "DESC".to_owned(),
            Some(Order::Name(name)) if super::php::string_is_truthy(&name) => name,
            Some(Order::Direction(SortDirection::Ascending) | Order::Name(_) | Order::Null)
            | None => "ASC".to_owned(),
        };
        self.parts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(format!("{sort} {order}"));
    }

    /// `count()`.
    pub fn count(&self) -> usize {
        self.parts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }

    /// `getParts()`.
    pub fn get_parts(&self) -> Vec<String> {
        self.parts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn properties_in_order(&self) -> Vec<(&'static str, Value)> {
        vec![
            ("preSeparator", Value::from(Self::PRE_SEPARATOR)),
            ("separator", Value::from(Self::SEPARATOR)),
            ("postSeparator", Value::from(Self::POST_SEPARATOR)),
            ("allowedClasses", Value::Array(PhpArray::new())),
            (
                "parts",
                Value::Array(PhpArray::list(
                    self.get_parts().into_iter().map(Value::from),
                )),
            ),
        ]
    }

    /// `__toString()`.
    pub fn try_to_string(&self) -> Result<PhpString, Thrown> {
        Ok(PhpString::from(format!(
            "{}{}{}",
            Self::PRE_SEPARATOR,
            self.get_parts().join(Self::SEPARATOR),
            Self::POST_SEPARATOR
        )))
    }
}

expression_object!(OrderBy);
