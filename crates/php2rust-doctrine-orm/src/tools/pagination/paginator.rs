//! `Paginator` and the `PaginatorQuery` trait it uses: the queries the
//! paginator derives from the query it pages (the count query, the clones
//! with the walker hints). `count()` and `getIterator()` run those
//! queries and wait for the port of `Query.php`; the paginator works on
//! the [`StandInQuery`] until then.
//!
//! Upstream: doctrine/orm src/Tools/Pagination/Paginator.php src/Tools/Pagination/PaginatorQuery.php

use php_value::{Key, Thrown, Value};

use crate::mapping::quote_strategy::sql_result_casing;
use crate::query::context::{
    DqlQuery, HINT_CUSTOM_OUTPUT_WALKER, HINT_CUSTOM_TREE_WALKERS, Hint, TreeWalkerClass,
};
use crate::query::result_set_mapping::ResultSetMapping;
use crate::query::{Parser, StandInQuery};
use crate::query_builder::QueryBuilder;

use super::{CountOutputWalker, CountWalker};

/// The paginator can handle various complex scenarios with DQL.
///
/// @deprecated This class is no longer needed by the ORM and will be removed in 4.0.
///             Use OffsetPaginator instead.
#[derive(Debug)]
pub struct Paginator {
    query: StandInQuery,
    fetch_join_collection: bool,
    /// Whether to force the use of an output walker. Null lets the paginator
    /// decide. The legacy Paginator exposes a setter for it, the
    /// stateless paginators take it as a constructor argument.
    use_output_walkers: Option<bool>,
}

impl Paginator {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Tools\\Pagination\\Paginator";

    /// @deprecated Use PaginatorInterface::HINT_ENABLE_DISTINCT instead.
    pub const HINT_ENABLE_DISTINCT: &'static str = super::HINT_ENABLE_DISTINCT;

    /// `new Paginator($query, $fetchJoinCollection)` of a `Query`
    /// (`$fetchJoinCollection` is `true` by default).
    pub fn new(query: StandInQuery, fetch_join_collection: bool) -> Self {
        Self {
            query,
            fetch_join_collection,
            use_output_walkers: None,
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
    pub fn get_query(&self) -> &StandInQuery {
        &self.query
    }

    /// Returns whether the query joins a collection.
    pub fn get_fetch_join_collection(&self) -> bool {
        self.fetch_join_collection
    }

    /// Returns whether the paginator will use an output walker.
    pub fn get_use_output_walkers(&self) -> Option<bool> {
        self.use_output_walkers
    }

    /// Sets whether the paginator will use an output walker.
    pub fn set_use_output_walkers(&mut self, use_output_walkers: Option<bool>) -> &mut Self {
        self.use_output_walkers = use_output_walkers;
        self
    }

    /// Determines whether to use an output walker for the query.
    pub fn use_output_walker(&self, query: &StandInQuery) -> bool {
        match self.use_output_walkers {
            None => !query.get_hint(HINT_CUSTOM_OUTPUT_WALKER).is_truthy(),
            Some(use_output_walkers) => use_output_walkers,
        }
    }

    /// `cloneQuery($query)`.
    pub fn clone_query(query: &StandInQuery) -> StandInQuery {
        let clone_query = query.clone_query();

        clone_query.set_parameters(query.get_parameters());
        clone_query.set_cacheable(false);

        for (name, value) in query.get_hints() {
            clone_query.set_hint(&name, value);
        }

        clone_query
    }

    /// `getCountQuery($query)`.
    pub fn get_count_query(&self, query: &StandInQuery) -> Result<StandInQuery, Thrown> {
        // As opposed to using self::cloneQuery, the following code does not transfer
        // a potentially existing result set mapping (either set directly by the user,
        // or taken from the parser result from a previous invocation of Query::parse())
        // to the new query object. This is fine, since we are going to completely change the
        // select clause, so a previously existing result set mapping (RSM) is probably wrong anyway.
        // In the case of using output walkers, we are even creating a new RSM down below.
        // In the case of using a tree walker, we want to have a new RSM created by the parser.
        let count_query = StandInQuery::new(query.entity_manager(), &query.get_dql());
        count_query.set_parameters(query.get_parameters());
        count_query.set_cacheable(false);
        for (name, value) in query.get_hints() {
            count_query.set_hint(&name, value);
        }

        if !count_query.has_hint(CountWalker::HINT_DISTINCT) {
            count_query.set_hint(CountWalker::HINT_DISTINCT, Hint::Value(Value::Bool(true)));
        }

        if self.use_output_walker(&count_query) {
            let platform = count_query.get_entity_manager().get_database_platform(); // law of demeter win

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
            count_query.set_result_set_mapping(rsm);
        } else {
            Self::append_tree_walker(&count_query, CountWalker::CLASS)?;
            Self::unbind_unused_query_params(&count_query)?;
        }

        count_query.set_first_result(Some(0)).set_max_results(None);

        Ok(count_query)
    }

    /// Appends a custom tree walker to the tree walkers hint.
    pub fn append_tree_walker(
        query: &StandInQuery,
        walker_class: TreeWalkerClass,
    ) -> Result<(), Thrown> {
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
    pub fn unbind_unused_query_params(query: &StandInQuery) -> Result<(), Thrown> {
        let parser_result = Parser::new(query)?.parse()?;
        let parameter_mappings = parser_result.borrow().get_parameter_mappings().clone();
        let mut parameters = query.get_parameters();

        parameters
            .retain(|parameter| parameter_mappings.contains_key(&Key::from(parameter.get_name())));

        query.set_parameters(parameters);
        Ok(())
    }
}
