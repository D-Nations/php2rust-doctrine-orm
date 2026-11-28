//! `CursorPaginator`: cursor-based pagination of a DQL query.
//!
//! The page's closures hold a clone of the paginated query (PHP's
//! closures hold the query itself). There is no query cache, so
//! `expireQueryCache()` has nothing to expire.
//!
//! Upstream: doctrine/orm src/Tools/Pagination/CursorPaginator.php

use std::rc::Rc;

use php_value::{PhpArray, Thrown, Value};
use php2rust_doctrine_persistence::mapping::AbstractClassMetadataFactory;

use crate::exception::from_dbal;
use crate::query::Query;
use crate::query::context::{HINT_CUSTOM_OUTPUT_WALKER, HYDRATE_OBJECT, Hint};
use crate::utility::get_type_of_field;

use super::cursor_page::LocalBoxFuture;
use super::cursor_walker::{CursorOrderByExpression, CursorOrderByItem, order_by_items_hint};
use super::paginator_query::{PaginatorQuery, QueryOrBuilder, sum_of_first_columns};
use super::{
    Cursor, CursorPage, CursorWalker, InvalidCursor, LimitSubqueryOutputWalker,
    LimitSubqueryWalker, PaginatorInterface, WhereInWalker,
};

/// The cursor paginator handles cursor-based pagination for DQL queries.
///
/// The query and the cursor are both passed to paginate(), which returns
/// an immutable CursorPage rather than the paginator itself. The paginator
/// therefore holds no state beyond its configuration: a single instance can be
/// shared as a service and reused for any query and any page. This keeps the
/// result free of temporal coupling and symmetric with OffsetPaginator.
///
/// Unlike the offset-based Window, which carries its own page size, a
/// Cursor is a pure position: the page size is therefore paginator
/// configuration and never comes from user input.
#[derive(Debug, Clone, Copy)]
pub struct CursorPaginator {
    limit: i64,
    query_produces_duplicates: bool,
    /// `use PaginatorQuery`.
    shared: PaginatorQuery,
}

/// `array_slice($array, 0, $length)`'s length.
fn slice_length(len: usize, length: i64) -> usize {
    match usize::try_from(length) {
        Ok(length) => length.min(len),
        Err(_) => len.saturating_sub(usize::try_from(length.unsigned_abs()).unwrap_or(usize::MAX)),
    }
}

fn type_error(message: String) -> Thrown {
    Thrown::new("TypeError", message, 0)
}

/// `current($row)`.
fn current(row: &Value) -> Value {
    match row {
        Value::Array(row) => row.values().next().cloned().unwrap_or(Value::Bool(false)),
        other => other.clone(),
    }
}

impl CursorPaginator {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Tools\\Pagination\\CursorPaginator";

    /// `new CursorPaginator($limit, $queryProducesDuplicates, $useOutputWalkers)`
    /// (`true` and `null` by default).
    ///
    /// `$limit`: the maximum number of results per page.
    /// `$queryProducesDuplicates`: whether the query joins a collection.
    /// `$useOutputWalkers`: whether to force the use of an output walker;
    /// null lets the paginator decide.
    pub fn new(
        limit: i64,
        query_produces_duplicates: bool,
        use_output_walkers: Option<bool>,
    ) -> Self {
        Self {
            limit,
            query_produces_duplicates,
            shared: PaginatorQuery::new(use_output_walkers),
        }
    }

    /// Returns the maximum number of results per page.
    pub fn get_limit(&self) -> i64 {
        self.limit
    }

    /// Returns whether the query joins a collection.
    pub fn get_query_produces_duplicates(&self) -> bool {
        self.query_produces_duplicates
    }

    /// `resolveCursor($position)`: an `InvalidCursor` if `$position` is not a
    /// Cursor instance, a string, or null.
    fn resolve_cursor(position: &Value) -> Result<Option<Cursor>, InvalidCursor> {
        if let Some(cursor) = position
            .as_object()
            .and_then(|object| object.downcast_ref::<Cursor>())
        {
            return Ok(Some(cursor.clone()));
        }

        match position {
            Value::Null => Ok(None),
            Value::String(s) if s.as_bytes().is_empty() => Ok(None),
            Value::String(_) => Cursor::from_encoded_string(position).map(Some),
            _ => Err(InvalidCursor::new(b"(non-string value)", None)),
        }
    }

    /// `getParametersForItem($query, $item, $orderByItems)`.
    async fn get_parameters_for_item(
        query: &Query,
        item: &Value,
        order_by_items: &[CursorOrderByItem],
    ) -> Result<PhpArray, Thrown> {
        let em = query.get_entity_manager();
        let connection = em.get_connection();
        let Value::Object(object) = item else {
            return Err(type_error(format!(
                "Cannot use \"::class\" on {}",
                item.debug_type()
            )));
        };
        let metadata = if em.get_metadata_factory().has_metadata_for(object.class()) {
            Some(em.get_class_metadata(object.class())?)
        } else {
            None
        };

        let mut result = PhpArray::new();

        for order_by_item in order_by_items {
            let CursorOrderByExpression::PathExpression { field, .. } = &order_by_item.expression
            else {
                continue;
            };

            let field_name = field.as_deref().unwrap_or_default();
            let order_metadata = order_by_item.metadata.clone().or_else(|| metadata.clone());
            let mut value = match &metadata {
                Some(metadata) => metadata.read().get_field_value(object, field_name)?,
                None => Value::Null,
            };
            if value.is_null() {
                value = object
                    .property(field_name.as_bytes())
                    .unwrap_or(Value::Null);
            }

            let target_entity = order_metadata.as_ref().and_then(|order_metadata| {
                order_metadata
                    .read()
                    .association_mappings
                    .get(field_name)
                    .map(|assoc| assoc.target_entity.clone())
            });
            if !value.is_null()
                && let Some(target_entity) = target_entity
            {
                let target_metadata = em.get_class_metadata(&target_entity)?;
                let target_metadata = target_metadata.read();
                let Value::Object(target) = &value else {
                    return Err(type_error(format!(
                        "Doctrine\\ORM\\Mapping\\ClassMetadata::getIdentifierValues(): Argument #1 ($entity) must be of type object, {} given",
                        value.debug_type()
                    )));
                };
                let identifier_values = target_metadata.get_identifier_values(target)?;
                let identifier_field = target_metadata.get_single_identifier_field_name()?;
                value = identifier_values
                    .get(&identifier_field.as_str().into())
                    .cloned()
                    .unwrap_or(Value::Null);
            }

            let Some(order_metadata) = order_metadata else {
                return Err(type_error(
                    "Doctrine\\ORM\\Utility\\PersisterHelper::getTypeOfField(): Argument #2 ($class) must be of type Doctrine\\ORM\\Mapping\\ClassMetadata, null given"
                        .to_owned(),
                ));
            };
            let field_types = get_type_of_field(field_name, &order_metadata.read(), em)?;
            let Some(type_) = field_types.into_iter().next() else {
                return Err(type_error(
                    "Doctrine\\DBAL\\Connection::convertToDatabaseValue(): Argument #2 ($type) must be of type string, null given"
                        .to_owned(),
                ));
            };

            result.insert(
                order_by_item.param_key.as_str(),
                connection
                    .convert_to_database_value(&value, &type_)
                    .await
                    .map_err(|e| from_dbal(&e))?,
            );
        }

        Ok(result)
    }
}

impl PaginatorInterface for CursorPaginator {
    type Page = CursorPage;

    /// Executes the query for the given cursor and returns an immutable page.
    ///
    /// Fetches $limit + 1 rows to detect whether a further page exists, then
    /// trims the extra row.
    ///
    /// `$position`: a cursor instance, an encoded cursor string, or
    /// null/empty string for the first page. Any other type (e.g. an
    /// array) throws InvalidCursor.
    #[allow(clippy::too_many_lines, reason = "paginate(), in its order")]
    async fn paginate(
        &self,
        query: QueryOrBuilder<'_>,
        position: &Value,
    ) -> Result<CursorPage, Thrown> {
        let query = PaginatorQuery::resolve_query(query)?;
        let cursor = Self::resolve_cursor(position)?;

        let should_reverse = cursor.as_ref().is_some_and(Cursor::is_previous);

        let sub_query = PaginatorQuery::clone_query(&query);

        PaginatorQuery::append_tree_walker(&sub_query, CursorWalker::CLASS)?;
        sub_query.set_hint(
            CursorWalker::HINT_QUERY_PRODUCES_DUPLICATES,
            Hint::Value(Value::Bool(self.query_produces_duplicates)),
        );

        if self.query_produces_duplicates {
            if self.shared.use_output_walker(&sub_query) {
                sub_query.set_hint(
                    HINT_CUSTOM_OUTPUT_WALKER,
                    Hint::OutputWalker(LimitSubqueryOutputWalker::CLASS),
                );
            } else {
                PaginatorQuery::append_tree_walker(&sub_query, LimitSubqueryWalker::CLASS)?;
            }
        }

        sub_query.set_hint(
            CursorWalker::HINT_CURSOR_REVERSE,
            Hint::Value(Value::Bool(should_reverse)),
        );
        sub_query.set_hint(
            CursorWalker::HINT_CURSOR_PARAMETERS,
            Hint::Value(Value::Array(
                cursor
                    .as_ref()
                    .map_or_default(|cursor| cursor.get_parameters().clone()),
            )),
        );
        sub_query
            .set_first_result(0)
            .set_max_results(Some(self.limit.saturating_add(1)));

        let (items, has_more, order_by_items) = if self.query_produces_duplicates {
            let found_id_rows: Vec<Value> = match sub_query.get_scalar_result().await? {
                Value::Array(rows) => rows.values().cloned().collect(),
                _ => Vec::new(),
            };
            let order_by_items =
                order_by_items_hint(&sub_query.get_hint(CursorWalker::HINT_CURSOR_ORDER_BY_ITEMS));
            let has_more = i64::try_from(found_id_rows.len()).unwrap_or(i64::MAX) > self.limit;
            let length = slice_length(found_id_rows.len(), self.limit);
            let found_id_rows = &found_id_rows[..length];

            let items = if found_id_rows.is_empty() {
                Vec::new()
            } else {
                let where_in_query = PaginatorQuery::clone_query(&query);
                let ids: Vec<Value> = found_id_rows.iter().map(current).collect();

                PaginatorQuery::append_tree_walker(&where_in_query, WhereInWalker::CLASS)?;
                where_in_query.set_hint(
                    WhereInWalker::HINT_PAGINATOR_HAS_IDS,
                    Hint::Value(Value::Bool(true)),
                );
                where_in_query.set_first_result(0).set_max_results(None);
                where_in_query.set_cacheable(query.is_cacheable());

                let database_ids =
                    PaginatorQuery::convert_where_in_identifiers_to_database_values(&query, ids)
                        .await?;
                where_in_query.set_parameter(
                    WhereInWalker::PAGINATOR_ID_ALIAS,
                    Value::Array(PhpArray::list(database_ids)),
                    None,
                );

                match where_in_query.get_result(HYDRATE_OBJECT).await? {
                    Value::Array(result) => result.values().cloned().collect(),
                    _ => Vec::new(),
                }
            };
            (items, has_more, order_by_items)
        } else {
            let result: Vec<Value> = match sub_query.get_result(HYDRATE_OBJECT).await? {
                Value::Array(result) => result.values().cloned().collect(),
                _ => Vec::new(),
            };
            let has_more = i64::try_from(result.len()).unwrap_or(i64::MAX) > self.limit;
            let length = slice_length(result.len(), self.limit);
            let mut result: Vec<Value> = result.into_iter().take(length).collect();
            let order_by_items =
                order_by_items_hint(&sub_query.get_hint(CursorWalker::HINT_CURSOR_ORDER_BY_ITEMS));

            if cursor.as_ref().is_some_and(Cursor::is_previous) {
                result.reverse();
            }

            (result, has_more, order_by_items)
        };

        let shared = self.shared;
        let page_query = Rc::new(query.clone_query());
        let count_query = Rc::clone(&page_query);
        let order_by_items = Rc::new(order_by_items);

        Ok(CursorPage::new(
            items,
            has_more,
            cursor,
            Box::new(move || -> LocalBoxFuture<i64> {
                let query = Rc::clone(&count_query);
                Box::pin(async move {
                    let rows = shared.get_count_query(&query)?.get_scalar_result().await?;
                    Ok(sum_of_first_columns(&rows))
                })
            }),
            Box::new(
                move |item: Value, is_next: bool| -> LocalBoxFuture<Cursor> {
                    let query = Rc::clone(&page_query);
                    let order_by_items = Rc::clone(&order_by_items);
                    Box::pin(async move {
                        Ok(Cursor::new(
                            Self::get_parameters_for_item(&query, &item, &order_by_items).await?,
                            is_next,
                        ))
                    })
                },
            ),
        ))
    }
}
