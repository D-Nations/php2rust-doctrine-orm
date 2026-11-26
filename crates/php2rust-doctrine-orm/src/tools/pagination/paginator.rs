//! `Paginator`: the legacy, stateful paginator over a [`Query`].
//!
//! Upstream: doctrine/orm src/Tools/Pagination/Paginator.php

use std::sync::{Mutex, PoisonError};

use php_value::{PhpArray, Thrown};

use crate::query::Query;
use crate::query_builder::QueryBuilder;

use super::paginator_query::{PaginatorQuery, sum_of_first_columns};

/// The paginator can handle various complex scenarios with DQL.
///
/// @deprecated This class is no longer needed by the ORM and will be removed in 4.0.
///             Use OffsetPaginator instead.
#[derive(Debug)]
pub struct Paginator {
    query: Query,
    count: Mutex<Option<i64>>,
    fetch_join_collection: bool,
    /// `use PaginatorQuery`.
    shared: PaginatorQuery,
}

impl Paginator {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Tools\\Pagination\\Paginator";

    /// @deprecated Use PaginatorInterface::HINT_ENABLE_DISTINCT instead.
    pub const HINT_ENABLE_DISTINCT: &'static str = super::HINT_ENABLE_DISTINCT;

    /// `new Paginator($query, $fetchJoinCollection)` of a `Query`
    /// (`$fetchJoinCollection` is `true` by default).
    pub fn new(query: Query, fetch_join_collection: bool) -> Self {
        Self {
            query,
            count: Mutex::new(None),
            fetch_join_collection,
            shared: PaginatorQuery::default(),
        }
    }

    /// `new Paginator($queryBuilder, $fetchJoinCollection)`:
    /// `resolveQuery()` takes `$queryBuilder->getQuery()`.
    pub fn from_query_builder(
        query_builder: &QueryBuilder,
        fetch_join_collection: bool,
    ) -> Result<Self, Thrown> {
        Ok(Self::new(query_builder.get_query()?, fetch_join_collection))
    }

    /// Returns the query.
    pub fn get_query(&self) -> &Query {
        &self.query
    }

    /// Returns whether the query joins a collection.
    pub fn get_fetch_join_collection(&self) -> bool {
        self.fetch_join_collection
    }

    /// Returns whether the paginator will use an output walker.
    pub fn get_use_output_walkers(&self) -> Option<bool> {
        self.shared.use_output_walkers
    }

    /// Sets whether the paginator will use an output walker.
    pub fn set_use_output_walkers(&mut self, use_output_walkers: Option<bool>) -> &mut Self {
        self.shared.use_output_walkers = use_output_walkers;
        self
    }

    /// The `PaginatorQuery` methods (private upstream).
    pub fn paginator_query(&self) -> &PaginatorQuery {
        &self.shared
    }

    /// `getCountQuery($query)` (private upstream).
    pub fn get_count_query(&self, query: &Query) -> Result<Query, Thrown> {
        self.shared.get_count_query(query)
    }

    /// `count()`.
    pub async fn count(&self) -> Result<i64, Thrown> {
        let known = *self.count.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(count) = known {
            return Ok(count);
        }
        let count = match self
            .shared
            .get_count_query(&self.query)?
            .get_scalar_result()
            .await
        {
            Ok(rows) => sum_of_first_columns(&rows),
            Err(e) if e.class == "Doctrine\\ORM\\NoResultException" => 0,
            Err(e) => return Err(e),
        };
        *self.count.lock().unwrap_or_else(PoisonError::into_inner) = Some(count);
        Ok(count)
    }

    /// `getIterator()`: the result, with its keys.
    pub async fn get_iterator(&self) -> Result<PhpArray, Thrown> {
        self.shared
            .get_result_for_offset(
                &self.query,
                self.query.get_first_result(),
                self.query.get_max_results(),
                self.fetch_join_collection,
            )
            .await
    }
}
