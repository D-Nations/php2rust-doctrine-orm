//! Upstream: doctrine/collections src/Criteria.php

use php_value::Thrown;

use crate::deprecations;
use crate::expr::{CompositeExpression, Expression};
use crate::{ExpressionBuilder, Order, SortDirection};

/// An ordering direction as `orderBy()` takes it: `Order` or
/// `\SortDirection`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Order(Order),
    SortDirection(SortDirection),
}

impl From<Order> for Direction {
    fn from(order: Order) -> Self {
        Self::Order(order)
    }
}

impl From<SortDirection> for Direction {
    fn from(direction: SortDirection) -> Self {
        Self::SortDirection(direction)
    }
}

impl Direction {
    /// The direction as a `\SortDirection` (`getOrderings()`).
    pub fn sort_direction(self) -> SortDirection {
        match self {
            Self::Order(Order::Ascending) => SortDirection::Ascending,
            Self::Order(Order::Descending) => SortDirection::Descending,
            Self::SortDirection(direction) => direction,
        }
    }

    /// The direction as an `Order` (the deprecated `orderings()`).
    pub fn order(self) -> Order {
        match self {
            Self::SortDirection(SortDirection::Ascending) => Order::Ascending,
            Self::SortDirection(SortDirection::Descending) => Order::Descending,
            Self::Order(order) => order,
        }
    }
}

const CLASS: &str = "Doctrine\\Common\\Collections\\Criteria";

/// `Criteria`: a filter (an expression), orderings and a slice, for
/// `Selectable::matching()`.
///
/// Orderings are kept as PHP keeps the array `orderBy()` receives: field
/// names in order, a repeated name replacing the earlier direction in
/// its place. (A numeric field name such as `'0'` is an int key in PHP,
/// which `matching()` then rejects; here it stays a string.)
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Criteria {
    expression: Option<Expression>,
    orderings: Vec<(String, Direction)>,
    first_result: Option<i64>,
    max_results: Option<i64>,
}

impl Criteria {
    /// `Criteria::create()`.
    pub fn create() -> Self {
        Self::new(None, None, 0, None)
    }

    /// `Criteria::create($accessRawFieldValues)`: the argument is
    /// deprecated (a deprecation is triggered) and a no-op.
    pub fn create_legacy(access_raw_field_values: bool) -> Self {
        let _ = access_raw_field_values;
        deprecations::trigger_access_raw_field_values(&format!("{CLASS}::create"));
        Self::create()
    }

    /// `Criteria::expr()`: the expression builder.
    pub fn expr() -> ExpressionBuilder {
        ExpressionBuilder
    }

    /// `new Criteria($expression, $orderings, $firstResult, $maxResults)`.
    pub fn new(
        expression: Option<Expression>,
        orderings: Option<Vec<(String, Direction)>>,
        first_result: i64,
        max_results: Option<i64>,
    ) -> Self {
        let mut criteria = Self {
            expression,
            ..Self::default()
        };
        criteria.set_first_result(first_result);
        criteria.set_max_results(max_results);
        if let Some(orderings) = orderings {
            criteria.order_by(orderings);
        }
        criteria
    }

    /// `new Criteria(..., $accessRawFieldValues)`: the fifth argument is
    /// deprecated (a deprecation is triggered) and a no-op.
    pub fn new_legacy(
        expression: Option<Expression>,
        orderings: Option<Vec<(String, Direction)>>,
        first_result: i64,
        max_results: Option<i64>,
        access_raw_field_values: bool,
    ) -> Self {
        let _ = access_raw_field_values;
        deprecations::trigger_access_raw_field_values(&format!("{CLASS}::__construct"));
        Self::new(expression, orderings, first_result, max_results)
    }

    /// `where($expression)`.
    pub fn r#where(&mut self, expression: impl Into<Expression>) -> &mut Self {
        self.expression = Some(expression.into());
        self
    }

    /// `andWhere($expression)`: the expression, or `AND` of the current one
    /// and it (a `RuntimeException` where either is a `Value`).
    pub fn and_where(&mut self, expression: impl Into<Expression>) -> Result<&mut Self, Thrown> {
        self.compose(CompositeExpression::TYPE_AND, expression.into())
    }

    /// `orWhere($expression)`: the expression, or `OR` of the current one
    /// and it (a `RuntimeException` where either is a `Value`).
    pub fn or_where(&mut self, expression: impl Into<Expression>) -> Result<&mut Self, Thrown> {
        self.compose(CompositeExpression::TYPE_OR, expression.into())
    }

    fn compose(&mut self, type_: &str, expression: Expression) -> Result<&mut Self, Thrown> {
        let Some(current) = self.expression.take() else {
            return Ok(self.r#where(expression));
        };
        match CompositeExpression::new(type_, [current.clone(), expression]) {
            Ok(composite) => {
                self.expression = Some(Expression::Composite(composite));
                Ok(self)
            }
            Err(error) => {
                self.expression = Some(current);
                Err(error)
            }
        }
    }

    /// `getWhereExpression()`.
    pub fn get_where_expression(&self) -> Option<&Expression> {
        self.expression.as_ref()
    }

    /// `getOrderings()`: the orderings as `\SortDirection`s.
    pub fn get_orderings(&self) -> Vec<(String, SortDirection)> {
        self.orderings
            .iter()
            .map(|(field, direction)| (field.clone(), direction.sort_direction()))
            .collect()
    }

    /// `orderings()`: the orderings as `Order`s.
    #[deprecated(note = "Use get_orderings() instead (doctrine/collections 3.1).")]
    pub fn orderings(&self) -> Vec<(String, Order)> {
        self.orderings
            .iter()
            .map(|(field, direction)| (field.clone(), direction.order()))
            .collect()
    }

    /// `orderBy($orderings)`: replaces the orderings.
    pub fn order_by<F, D>(&mut self, orderings: impl IntoIterator<Item = (F, D)>) -> &mut Self
    where
        F: Into<String>,
        D: Into<Direction>,
    {
        let mut kept: Vec<(String, Direction)> = Vec::new();
        for (field, direction) in orderings {
            let (field, direction) = (field.into(), direction.into());
            match kept.iter_mut().find(|(known, _)| *known == field) {
                Some(entry) => entry.1 = direction,
                None => kept.push((field, direction)),
            }
        }
        self.orderings = kept;
        self
    }

    /// `getFirstResult()`.
    pub fn get_first_result(&self) -> Option<i64> {
        self.first_result
    }

    /// `setFirstResult($firstResult)`.
    pub fn set_first_result(&mut self, first_result: i64) -> &mut Self {
        self.first_result = Some(first_result);
        self
    }

    /// `getMaxResults()`.
    pub fn get_max_results(&self) -> Option<i64> {
        self.max_results
    }

    /// `setMaxResults($maxResults)`.
    pub fn set_max_results(&mut self, max_results: Option<i64>) -> &mut Self {
        self.max_results = max_results;
        self
    }
}
