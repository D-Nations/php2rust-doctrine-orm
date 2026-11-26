//! `OffsetPaginator` and `PaginatorInterface`.
//!
//! Upstream: doctrine/orm src/Tools/Pagination/OffsetPaginator.php src/Tools/Pagination/PaginatorInterface.php

use php_value::{Thrown, Value};

use super::paginator_query::{PaginatorQuery, QueryOrBuilder, sum_of_first_columns};
use super::{Window, WindowPage};

/// Paginates a DQL query for a given pagination position.
///
/// Implementations hold no query and no position: both are passed to
/// paginate(), so a paginator is stateless and can be reused — and shared
/// as a service — across queries and pages.
///
/// The position type depends on the pagination strategy, so it is `mixed`
/// (a `Value`) here.
#[allow(async_fn_in_trait, reason = "the paginators run on one task")]
pub trait PaginatorInterface {
    /// The page `paginate()` gives.
    type Page;

    /// Executes the query for the given position and returns an immutable page.
    async fn paginate(
        &self,
        query: QueryOrBuilder<'_>,
        position: &Value,
    ) -> Result<Self::Page, Thrown>;
}

/// Offset-based paginator, designed to be symmetric with CursorPaginator.
///
/// The query and the pagination position are both passed to paginate(),
/// which returns an immutable, iterable WindowPage rather than the
/// paginator itself. The paginator therefore holds no state beyond its
/// configuration: a single instance can be shared as a service and reused for
/// any query and any page. This avoids the implicit offset handling and the
/// stateful API of the legacy Paginator, which this class is intended to
/// replace.
#[derive(Debug, Clone, Copy)]
pub struct OffsetPaginator {
    fetch_join_collection: bool,
    /// `use PaginatorQuery`.
    shared: PaginatorQuery,
}

impl Default for OffsetPaginator {
    fn default() -> Self {
        Self::new(true, None)
    }
}

impl OffsetPaginator {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Tools\\Pagination\\OffsetPaginator";

    /// `new OffsetPaginator($fetchJoinCollection, $useOutputWalkers)`
    /// (`true` and `null` by default).
    pub fn new(fetch_join_collection: bool, use_output_walkers: Option<bool>) -> Self {
        Self {
            fetch_join_collection,
            shared: PaginatorQuery::new(use_output_walkers),
        }
    }

    /// Returns whether the query joins a collection.
    pub fn get_fetch_join_collection(&self) -> bool {
        self.fetch_join_collection
    }

    /// The `PaginatorQuery` methods (private upstream).
    pub fn paginator_query(&self) -> &PaginatorQuery {
        &self.shared
    }

    /// `getCountQuery($query)` (private upstream).
    pub fn get_count_query(
        &self,
        query: &crate::query::Query,
    ) -> Result<crate::query::Query, Thrown> {
        self.shared.get_count_query(query)
    }

    /// `paginate($query, $window)` with a [`Window`].
    pub async fn paginate_window(
        &self,
        query: QueryOrBuilder<'_>,
        position: Window,
    ) -> Result<WindowPage, Thrown> {
        self.paginate(query, &Value::from(position)).await
    }
}

impl PaginatorInterface for OffsetPaginator {
    type Page = WindowPage;

    /// Executes the query for the given window and returns an immutable page
    /// (an `InvalidArgumentException` if the position is not a Window).
    async fn paginate(
        &self,
        query: QueryOrBuilder<'_>,
        position: &Value,
    ) -> Result<WindowPage, Thrown> {
        let Some(position) = position
            .as_object()
            .and_then(|object| object.downcast_ref::<Window>())
            .copied()
        else {
            return Err(Thrown::new(
                "InvalidArgumentException",
                format!(
                    "Expected an instance of {}, got {}.",
                    Window::CLASS,
                    position.debug_type()
                ),
                0,
            ));
        };

        let query = PaginatorQuery::resolve_query(query)?;

        let items: Vec<Value> = self
            .shared
            .get_result_for_offset(
                &query,
                position.get_first_result(),
                Some(position.get_max_results()),
                self.fetch_join_collection,
            )
            .await?
            .values()
            .cloned()
            .collect();

        let total_count = match self
            .shared
            .get_count_query(&query)?
            .get_scalar_result()
            .await
        {
            Ok(rows) => sum_of_first_columns(&rows),
            Err(e) if e.class == "Doctrine\\ORM\\NoResultException" => 0,
            Err(e) => return Err(e),
        };

        Ok(WindowPage::new(items, total_count, position))
    }
}
