//! `PaginatorQuery`: the implementation shared by the paginators, as a
//! value the paginators hold (PHP's trait).
//!
//! Upstream: doctrine/orm src/Tools/Pagination/PaginatorQuery.php

use php_value::numeric::int_cast;
use php_value::{Key, PhpArray, Thrown, Value};

use crate::exception::from_dbal;
use crate::mapping::quote_strategy::sql_result_casing;
use crate::query::context::{
    DqlQuery, HINT_CUSTOM_OUTPUT_WALKER, HINT_CUSTOM_TREE_WALKERS, Hint, TreeWalkerClass,
};
use crate::query::exec::SqlStatements;
use crate::query::result_set_mapping::ResultSetMapping;
use crate::query::{Parser, Query};
use crate::query_builder::QueryBuilder;

use super::{
    CountOutputWalker, CountWalker, LimitSubqueryOutputWalker, LimitSubqueryWalker, RootTypeWalker,
    WhereInWalker,
};

/// `Query|QueryBuilder $query`.
#[derive(Clone, Copy)]
pub enum QueryOrBuilder<'q> {
    Query(&'q Query),
    QueryBuilder(&'q QueryBuilder),
}

impl<'q> From<&'q Query> for QueryOrBuilder<'q> {
    fn from(query: &'q Query) -> Self {
        Self::Query(query)
    }
}

impl<'q> From<&'q QueryBuilder> for QueryOrBuilder<'q> {
    fn from(query_builder: &'q QueryBuilder) -> Self {
        Self::QueryBuilder(query_builder)
    }
}

/// A query `resolveQuery()` gives: the query passed in, or the query
/// builder's new query.
pub enum ResolvedQuery<'q> {
    Borrowed(&'q Query),
    Owned(Box<Query>),
}

impl std::ops::Deref for ResolvedQuery<'_> {
    type Target = Query;

    fn deref(&self) -> &Query {
        match self {
            Self::Borrowed(query) => query,
            Self::Owned(query) => query,
        }
    }
}

/// `(int) array_sum(array_map('current', $rows))`.
pub(crate) fn sum_of_first_columns(rows: &Value) -> i64 {
    let Value::Array(rows) = rows else {
        return 0;
    };
    let mut sum = Value::Int(0);
    for row in rows.values() {
        let first = match row {
            Value::Array(row) => row.values().next().cloned().unwrap_or(Value::Bool(false)),
            other => other.clone(),
        };
        // array_sum() skips arrays and objects.
        if matches!(first, Value::Array(_) | Value::Object(_)) {
            continue;
        }
        if let Ok(added) = php_value::ops::add(&sum, &first) {
            sum = added;
        }
    }
    int_cast(&sum)
}

/// Provides the implementation shared by Paginator, OffsetPaginator
/// and CursorPaginator.
///
/// Every method takes the query it works on as an argument, so that the
/// stateless paginators can serve any query.
#[derive(Debug, Clone, Copy, Default)]
pub struct PaginatorQuery {
    /// Whether to force the use of an output walker. Null lets the paginator
    /// decide. The legacy Paginator exposes a setter for it, the
    /// stateless paginators take it as a constructor argument.
    pub use_output_walkers: Option<bool>,
}

impl PaginatorQuery {
    pub fn new(use_output_walkers: Option<bool>) -> Self {
        Self { use_output_walkers }
    }

    /// Determines whether to use an output walker for the query.
    pub fn use_output_walker(&self, query: &Query) -> bool {
        match self.use_output_walkers {
            None => !query.get_hint(HINT_CUSTOM_OUTPUT_WALKER).is_truthy(),
            Some(use_output_walkers) => use_output_walkers,
        }
    }

    /// `resolveQuery($query)`.
    pub fn resolve_query(query: QueryOrBuilder<'_>) -> Result<ResolvedQuery<'_>, Thrown> {
        Ok(match query {
            QueryOrBuilder::QueryBuilder(query_builder) => {
                ResolvedQuery::Owned(Box::new(query_builder.get_query()?))
            }
            QueryOrBuilder::Query(query) => ResolvedQuery::Borrowed(query),
        })
    }

    /// `cloneQuery($query)`.
    pub fn clone_query(query: &Query) -> Query {
        let clone_query = query.clone_query();

        clone_query.set_parameters(query.get_parameters());
        clone_query.set_cacheable(false);

        for (name, value) in query.get_hints() {
            clone_query.set_hint(&name, value);
        }

        clone_query
    }

    /// `convertWhereInIdentifiersToDatabaseValues($query, $identifiers)`.
    pub async fn convert_where_in_identifiers_to_database_values(
        query: &Query,
        identifiers: Vec<Value>,
    ) -> Result<Vec<Value>, Thrown> {
        let type_query = Self::clone_query(query);
        type_query.set_hint(
            HINT_CUSTOM_OUTPUT_WALKER,
            Hint::OutputWalker(RootTypeWalker::CLASS),
        );

        let connection = query.get_entity_manager().get_connection();
        let type_ = match type_query.get_sql()? {
            SqlStatements::One(type_) => type_,
            SqlStatements::Many(_) => {
                return Err(Thrown::new("AssertionError", "assert(is_string($type))", 0));
            }
        };

        let mut values = Vec::with_capacity(identifiers.len());
        for id in identifiers {
            values.push(
                connection
                    .convert_to_database_value(&id, &type_)
                    .await
                    .map_err(|e| from_dbal(&e))?,
            );
        }
        Ok(values)
    }

    /// `getCountQuery($query)`.
    pub fn get_count_query(&self, query: &Query) -> Result<Query, Thrown> {
        // As opposed to using self::cloneQuery, the following code does not transfer
        // a potentially existing result set mapping (either set directly by the user,
        // or taken from the parser result from a previous invocation of Query::parse())
        // to the new query object. This is fine, since we are going to completely change the
        // select clause, so a previously existing result set mapping (RSM) is probably wrong anyway.
        // In the case of using output walkers, we are even creating a new RSM down below.
        // In the case of using a tree walker, we want to have a new RSM created by the parser.
        let count_query = Query::new(query.get_entity_manager());
        if let Some(dql) = query.get_dql() {
            count_query.set_dql(&dql);
        }
        count_query.set_parameters(query.get_parameters());
        count_query.set_cacheable(false);
        for (name, value) in query.get_hints() {
            count_query.set_hint(&name, value);
        }

        if !count_query.has_hint(CountWalker::HINT_DISTINCT) {
            count_query.set_hint(CountWalker::HINT_DISTINCT, Hint::Value(Value::Bool(true)));
        }

        if self.use_output_walker(&count_query) {
            let platform = count_query.get_entity_manager().get_platform(); // law of demeter win

            let mut rsm = ResultSetMapping::new();
            rsm.add_scalar_result(
                &sql_result_casing(platform, "dctrn_count"),
                "count",
                "string",
            );

            count_query.set_hint(
                HINT_CUSTOM_OUTPUT_WALKER,
                Hint::OutputWalker(CountOutputWalker::CLASS),
            );
            count_query.set_result_set_mapping(rsm)?;
        } else {
            Self::append_tree_walker(&count_query, CountWalker::CLASS)?;
            Self::unbind_unused_query_params(&count_query)?;
        }

        count_query.set_first_result(0).set_max_results(None);

        Ok(count_query)
    }

    /// Executes the query for the given offset window and returns the entities.
    ///
    /// Shared by Paginator and OffsetPaginator: when a to-many
    /// collection is fetch-joined, it uses the ID subquery + WHERE IN strategy to
    /// return the correct number of root entities despite duplicate rows.
    ///
    /// The result keys are preserved (e.g. for DQL ``INDEX BY``); callers that
    /// need a list should apply array_values() themselves.
    pub async fn get_result_for_offset(
        &self,
        query: &Query,
        offset: i64,
        length: Option<i64>,
        fetch_join_collection: bool,
    ) -> Result<PhpArray, Thrown> {
        let result = if fetch_join_collection && length.is_some() {
            let sub_query = Self::clone_query(query);

            if self.use_output_walker(&sub_query) {
                sub_query.set_hint(
                    HINT_CUSTOM_OUTPUT_WALKER,
                    Hint::OutputWalker(LimitSubqueryOutputWalker::CLASS),
                );
            } else {
                Self::append_tree_walker(&sub_query, LimitSubqueryWalker::CLASS)?;
                Self::unbind_unused_query_params(&sub_query)?;
            }

            sub_query.set_first_result(offset).set_max_results(length);

            let found_id_rows = sub_query.get_scalar_result().await?;

            // don't do this for an empty id array
            let ids: Vec<Value> = match &found_id_rows {
                Value::Array(rows) if !rows.is_empty() => rows
                    .values()
                    .map(|row| match row {
                        Value::Array(row) => {
                            row.values().next().cloned().unwrap_or(Value::Bool(false))
                        }
                        other => other.clone(),
                    })
                    .collect(),
                _ => return Ok(PhpArray::new()),
            };

            let where_in_query = Self::clone_query(query);

            Self::append_tree_walker(&where_in_query, WhereInWalker::CLASS)?;
            where_in_query.set_hint(
                WhereInWalker::HINT_PAGINATOR_HAS_IDS,
                Hint::Value(Value::Bool(true)),
            );
            where_in_query.set_first_result(0).set_max_results(None);
            where_in_query.set_cacheable(query.is_cacheable());

            let database_ids =
                Self::convert_where_in_identifiers_to_database_values(query, ids).await?;
            where_in_query.set_parameter(
                WhereInWalker::PAGINATOR_ID_ALIAS,
                Value::Array(PhpArray::list(database_ids)),
                None,
            );

            where_in_query
                .get_result(query.get_hydration_mode())
                .await?
        } else {
            let clone_query = Self::clone_query(query);
            clone_query
                .set_max_results(length)
                .set_first_result(offset)
                .set_cacheable(query.is_cacheable());
            clone_query.get_result(query.get_hydration_mode()).await?
        };

        match result {
            Value::Array(result) => Ok(result),
            _ => Err(Thrown::new(
                "AssertionError",
                "assert(is_array($result))",
                0,
            )),
        }
    }

    /// Appends a custom tree walker to the tree walkers hint.
    pub fn append_tree_walker(query: &Query, walker_class: TreeWalkerClass) -> Result<(), Thrown> {
        let mut hints = match query.get_hint(HINT_CUSTOM_TREE_WALKERS) {
            Hint::Value(Value::Bool(false)) => Vec::new(),
            Hint::TreeWalkers(hints) => hints,
            // `$hints[] = ...` on a scalar.
            _ => {
                return Err(Thrown::new(
                    "Error",
                    "Cannot use a scalar value as an array",
                    0,
                ));
            }
        };

        hints.push(walker_class);
        query.set_hint(HINT_CUSTOM_TREE_WALKERS, Hint::TreeWalkers(hints));
        Ok(())
    }

    /// `unbindUnusedQueryParams($query)`.
    pub fn unbind_unused_query_params(query: &Query) -> Result<(), Thrown> {
        let parser_result = Parser::new(query as &dyn DqlQuery)?.parse()?;
        let parameter_mappings = parser_result.borrow().get_parameter_mappings().clone();
        let mut parameters = query.get_parameters();

        parameters
            .retain(|parameter| parameter_mappings.contains_key(&Key::from(parameter.get_name())));

        query.set_parameters(parameters);
        Ok(())
    }
}
