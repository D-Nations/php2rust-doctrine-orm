//! Upstream: doctrine/collections src/Selectable.php

use php_value::Thrown;

use crate::{Collection, Criteria};

/// `Selectable`: a collection that can be filtered by a `Criteria`.
pub trait Selectable {
    /// `matching($criteria)`: the elements that match the criteria's
    /// expression, in its orderings, sliced by its first and max
    /// results. PHP declares a `ReadableCollection`; the package's
    /// implementations return a `Collection`, as here.
    fn matching(&mut self, criteria: &Criteria) -> Result<Box<dyn Collection>, Thrown>;
}
