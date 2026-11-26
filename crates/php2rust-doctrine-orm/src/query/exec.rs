//! `Doctrine\ORM\Query\Exec`: the SQL executors and finalizers, as data.
//! An executor holds the SQL statements the walker generated (and, for the
//! multi-table executors, the temporary table statements and parameter
//! lists). `execute()` runs them on a connection; a SELECT's rows are
//! fetched whole. The query cache profile is not ported.
//!
//! Upstream: doctrine/orm src/Query/Exec/AbstractSqlExecutor.php src/Query/Exec/FinalizedSelectExecutor.php src/Query/Exec/MultiTableDeleteExecutor.php src/Query/Exec/MultiTableUpdateExecutor.php src/Query/Exec/PreparedExecutorFinalizer.php src/Query/Exec/SingleSelectExecutor.php src/Query/Exec/SingleSelectSqlFinalizer.php src/Query/Exec/SingleTableDeleteUpdateExecutor.php src/Query/Exec/SqlFinalizer.php

use std::fmt;

use php_value::{PhpArray, Thrown, Value};
use php2rust_doctrine_dbal::LockMode;
use php2rust_doctrine_dbal::schema::TypeObject;

use crate::deprecations;
use crate::query::ast::{
    DeleteStatement, FromClause, IdentificationVariableDeclaration, Node, RangeVariableDeclaration,
    SelectStatement, UpdateStatement,
};
use crate::query::context::{DqlQuery, lock_mode_hint};
use crate::query::query_exception;
use crate::query::sql_walker::{SqlWalker, dbal_error};
use crate::query::utility;

/// `AbstractSqlExecutor::$sqlStatements` (`array|string`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SqlStatements {
    One(String),
    Many(Vec<String>),
}

/// What kind of executor, with the state a kind keeps besides its SQL
/// statements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutorKind {
    /// `FinalizedSelectExecutor`.
    FinalizedSelect,
    /// `SingleSelectExecutor`.
    SingleSelect,
    /// `SingleTableDeleteUpdateExecutor`.
    SingleTableDeleteUpdate,
    /// `MultiTableDeleteExecutor`.
    MultiTableDelete {
        create_temp_table_sql: String,
        drop_temp_table_sql: String,
        insert_sql: String,
    },
    /// `MultiTableUpdateExecutor`.
    MultiTableUpdate {
        create_temp_table_sql: String,
        drop_temp_table_sql: String,
        insert_sql: String,
        sql_parameters: Vec<Vec<String>>,
        num_parameters_in_update_clause: i64,
    },
}

/// An `AbstractSqlExecutor`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlExecutor {
    sql_statements: SqlStatements,
    kind: ExecutorKind,
}

impl SqlExecutor {
    /// An executor of the given kind.
    pub fn new(kind: ExecutorKind, sql_statements: SqlStatements) -> Self {
        Self {
            sql_statements,
            kind,
        }
    }

    /// `getSqlStatements()`.
    pub fn get_sql_statements(&self) -> &SqlStatements {
        &self.sql_statements
    }

    pub fn kind(&self) -> &ExecutorKind {
        &self.kind
    }

    /// The executor's class.
    pub fn class(&self) -> &'static str {
        match self.kind {
            ExecutorKind::FinalizedSelect => "Doctrine\\ORM\\Query\\Exec\\FinalizedSelectExecutor",
            ExecutorKind::SingleSelect => "Doctrine\\ORM\\Query\\Exec\\SingleSelectExecutor",
            ExecutorKind::SingleTableDeleteUpdate => {
                "Doctrine\\ORM\\Query\\Exec\\SingleTableDeleteUpdateExecutor"
            }
            ExecutorKind::MultiTableDelete { .. } => {
                "Doctrine\\ORM\\Query\\Exec\\MultiTableDeleteExecutor"
            }
            ExecutorKind::MultiTableUpdate { .. } => {
                "Doctrine\\ORM\\Query\\Exec\\MultiTableUpdateExecutor"
            }
        }
    }

    /// `new FinalizedSelectExecutor($sql)`.
    pub fn finalized_select(sql: String) -> Self {
        Self::new(ExecutorKind::FinalizedSelect, SqlStatements::One(sql))
    }

    /// `new SingleSelectExecutor($AST, $sqlWalker)`.
    pub fn single_select(
        ast: &SelectStatement,
        sql_walker: &mut dyn SqlWalker<'_>,
    ) -> Result<Self, Thrown> {
        deprecations::trigger("https://github.com/doctrine/orm/pull/11188/", || {
            "The Doctrine\\ORM\\Query\\Exec\\SingleSelectExecutor is no longer needed by the ORM and will be removed in 4.0".to_owned()
        });
        let sql = sql_walker.walk_select_statement(ast)?;
        Ok(Self::new(
            ExecutorKind::SingleSelect,
            SqlStatements::One(sql),
        ))
    }

    /// `new SingleTableDeleteUpdateExecutor($AST, $sqlWalker)` for an
    /// UPDATE.
    pub fn single_table_update(
        ast: &UpdateStatement,
        sql_walker: &mut dyn SqlWalker<'_>,
    ) -> Result<Self, Thrown> {
        let sql = sql_walker.walk_update_statement(ast)?;
        Ok(Self::new(
            ExecutorKind::SingleTableDeleteUpdate,
            SqlStatements::One(sql),
        ))
    }

    /// `new SingleTableDeleteUpdateExecutor($AST, $sqlWalker)` for a
    /// DELETE.
    pub fn single_table_delete(
        ast: &DeleteStatement,
        sql_walker: &mut dyn SqlWalker<'_>,
    ) -> Result<Self, Thrown> {
        let sql = sql_walker.walk_delete_statement(ast)?;
        Ok(Self::new(
            ExecutorKind::SingleTableDeleteUpdate,
            SqlStatements::One(sql),
        ))
    }

    /// `new MultiTableDeleteExecutor($AST, $sqlWalker)`.
    pub fn multi_table_delete(
        ast: &DeleteStatement,
        sql_walker: &mut dyn SqlWalker<'_>,
    ) -> Result<Self, Thrown> {
        let em = sql_walker.get_entity_manager();
        let primary_class = em.get_class_metadata(&ast.delete_clause.abstract_schema_name)?;
        let primary_dql_alias = ast.delete_clause.alias_identification_variable.clone();
        let root_class = em.get_class_metadata(&primary_class.read().root_entity_name)?;
        let temp = TempTable::new(sql_walker, &root_class)?;

        let primary_table = primary_class.read().get_table_name()?;
        sql_walker.set_sql_table_alias(&primary_table, "t0", &primary_dql_alias);

        let mut insert_sql = temp.insert_prefix();
        let primary_name = primary_class.read().name.clone();
        insert_sql.push_str(&walk_primary_from(
            sql_walker,
            &primary_name,
            &primary_dql_alias,
        )?);
        if let Some(where_clause) = &ast.where_clause {
            insert_sql.push_str(&sql_walker.walk_where_clause(Some(where_clause))?);
        }

        let id_subselect = format!("SELECT {} FROM {}", temp.id_column_list, temp.table);
        let em = sql_walker.get_entity_manager();
        let mut statements = Vec::new();
        for class_name in class_names(&primary_class).iter().rev() {
            let class = em.get_class_metadata(class_name)?;
            let table_name = em
                .get_quote_strategy()
                .get_table_name(&class.read(), em.get_database_platform());
            statements.push(format!(
                "DELETE FROM {table_name} WHERE ({}) IN ({id_subselect})",
                temp.id_column_list
            ));
        }

        let (create_temp_table_sql, drop_temp_table_sql) = temp.create_and_drop(sql_walker)?;
        Ok(Self::new(
            ExecutorKind::MultiTableDelete {
                create_temp_table_sql,
                drop_temp_table_sql,
                insert_sql,
            },
            SqlStatements::Many(statements),
        ))
    }

    /// `new MultiTableUpdateExecutor($AST, $sqlWalker)`.
    pub fn multi_table_update(
        ast: &UpdateStatement,
        sql_walker: &mut dyn SqlWalker<'_>,
    ) -> Result<Self, Thrown> {
        let update_clause = &ast.update_clause;
        let em = sql_walker.get_entity_manager();
        let primary_class = em.get_class_metadata(&update_clause.abstract_schema_name)?;
        let root_class = em.get_class_metadata(&primary_class.read().root_entity_name)?;
        let temp = TempTable::new(sql_walker, &root_class)?;

        let primary_table = primary_class.read().get_table_name()?;
        sql_walker.set_sql_table_alias(
            &primary_table,
            "t0",
            &update_clause.alias_identification_variable,
        );

        let mut insert_sql = temp.insert_prefix();
        let primary_name = primary_class.read().name.clone();
        insert_sql.push_str(&walk_primary_from(
            sql_walker,
            &primary_name,
            &update_clause.alias_identification_variable,
        )?);

        let id_subselect = format!("SELECT {} FROM {}", temp.id_column_list, temp.table);
        let mut statements = Vec::new();
        let mut all_sql_parameters = Vec::new();
        let mut num_parameters_in_update_clause = 0;
        for class_name in class_names(&primary_class).iter().rev() {
            let mut affected = false;
            let em = sql_walker.get_entity_manager();
            let class = em.get_class_metadata(class_name)?;
            let mut update_sql = format!(
                "UPDATE {} SET ",
                em.get_quote_strategy()
                    .get_table_name(&class.read(), em.get_database_platform())
            );
            let mut sql_parameters = Vec::new();
            for update_item in &update_clause.update_items {
                let field = update_item
                    .path_expression
                    .borrow()
                    .field
                    .clone()
                    .unwrap_or_default();
                let own = {
                    let info = class.read();
                    info.field_mappings
                        .get(&field)
                        .is_some_and(|m| m.inherited.is_none())
                        || info
                            .association_mappings
                            .get(&field)
                            .is_some_and(|m| m.inherited.is_none())
                };
                if own {
                    if affected {
                        update_sql.push_str(", ");
                    } else {
                        affected = true;
                    }
                    update_sql.push_str(&sql_walker.walk_update_item(update_item)?);
                    if let Some(Node::InputParameter(parameter)) = &update_item.new_value {
                        sql_parameters.push(parameter.name.clone());
                        num_parameters_in_update_clause += 1;
                    }
                }
            }
            if affected {
                all_sql_parameters.push(sql_parameters);
                statements.push(format!(
                    "{update_sql} WHERE ({}) IN ({id_subselect})",
                    temp.id_column_list
                ));
            }
        }

        if let Some(where_clause) = &ast.where_clause {
            insert_sql.push_str(&sql_walker.walk_where_clause(Some(where_clause))?);
        }

        let (create_temp_table_sql, drop_temp_table_sql) = temp.create_and_drop(sql_walker)?;
        Ok(Self::new(
            ExecutorKind::MultiTableUpdate {
                create_temp_table_sql,
                drop_temp_table_sql,
                insert_sql,
                sql_parameters: all_sql_parameters,
                num_parameters_in_update_clause,
            },
            SqlStatements::Many(statements),
        ))
    }
}

impl SqlExecutor {
    /// Executes all sql statements.
    #[allow(clippy::too_many_lines, reason = "the execute() of each executor")]
    pub async fn execute(
        &self,
        conn: &php2rust_doctrine_dbal::Connection,
        params: &PhpArray,
        types: &php2rust_doctrine_dbal::ParameterTypes,
    ) -> Result<crate::abstract_query::Executed, Thrown> {
        use crate::abstract_query::Executed;
        let no_types = php2rust_doctrine_dbal::ParameterTypes::new();
        let empty = PhpArray::new();
        let one = |statements: &SqlStatements| match statements {
            SqlStatements::One(sql) => sql.clone(),
            SqlStatements::Many(sql) => sql.join("; "),
        };
        match &self.kind {
            ExecutorKind::FinalizedSelect | ExecutorKind::SingleSelect => {
                let rows = conn
                    .fetch_all_associative(&one(&self.sql_statements), params, types)
                    .await
                    .map_err(dbal_error)?;
                Ok(Executed::Rows(rows))
            }
            ExecutorKind::SingleTableDeleteUpdate => {
                let count = conn
                    .execute_statement(&one(&self.sql_statements), params, types)
                    .await
                    .map_err(dbal_error)?;
                Ok(Executed::Count(count))
            }
            ExecutorKind::MultiTableDelete {
                create_temp_table_sql,
                drop_temp_table_sql,
                insert_sql,
            } => {
                // Create temporary id table
                conn.execute_statement(create_temp_table_sql, &empty, &no_types)
                    .await
                    .map_err(dbal_error)?;
                let outcome = async {
                    // Insert identifiers
                    let num_deleted = conn
                        .execute_statement(insert_sql, params, types)
                        .await
                        .map_err(dbal_error)?;
                    // Execute DELETE statements
                    for sql in statements(&self.sql_statements) {
                        conn.execute_statement(sql, &empty, &no_types)
                            .await
                            .map_err(dbal_error)?;
                    }
                    Ok::<i64, Thrown>(num_deleted)
                }
                .await;
                // Drop temporary table (also on a failure, to avoid
                // possible collisions)
                let dropped = conn
                    .execute_statement(drop_temp_table_sql, &empty, &no_types)
                    .await
                    .map_err(dbal_error);
                let num_deleted = outcome?;
                dropped?;
                Ok(Executed::Count(num_deleted))
            }
            ExecutorKind::MultiTableUpdate {
                create_temp_table_sql,
                drop_temp_table_sql,
                insert_sql,
                sql_parameters,
                num_parameters_in_update_clause,
            } => {
                // Create temporary id table
                conn.execute_statement(create_temp_table_sql, &empty, &no_types)
                    .await
                    .map_err(dbal_error)?;
                let skip = usize::try_from(*num_parameters_in_update_clause).unwrap_or(0);
                let outcome = async {
                    // Insert identifiers. Parameters from the update
                    // clause are cut off.
                    let mut insert_params = PhpArray::new();
                    let mut insert_types = php2rust_doctrine_dbal::ParameterTypes::new();
                    for (i, value) in params.values().skip(skip).enumerate() {
                        let _ = insert_params.push(value.clone());
                        if let Some(ty) = types.get(&php_value::Key::Int(
                            i64::try_from(i + skip).unwrap_or(i64::MAX),
                        )) {
                            insert_types.insert(
                                php_value::Key::Int(i64::try_from(i).unwrap_or(i64::MAX)),
                                ty.clone(),
                            );
                        }
                    }
                    let num_updated = conn
                        .execute_statement(insert_sql, &insert_params, &insert_types)
                        .await
                        .map_err(dbal_error)?;

                    // Execute UPDATE statements
                    let param_list: Vec<&Value> = params.values().collect();
                    for (key, statement) in statements(&self.sql_statements).iter().enumerate() {
                        let mut param_values = PhpArray::new();
                        let mut param_types = php2rust_doctrine_dbal::ParameterTypes::new();
                        if let Some(names) = sql_parameters.get(key) {
                            for (parameter_key, _) in names.iter().enumerate() {
                                let value = param_list
                                    .get(parameter_key)
                                    .map_or(Value::Null, |v| (*v).clone());
                                let ty = types
                                    .get(&php_value::Key::Int(
                                        i64::try_from(parameter_key).unwrap_or(i64::MAX),
                                    ))
                                    .cloned()
                                    .unwrap_or_else(|| {
                                        crate::query::ParameterTypeInferer::infer_type(&value)
                                    });
                                let index = php_value::Key::Int(
                                    i64::try_from(param_values.len()).unwrap_or(i64::MAX),
                                );
                                param_types.insert(index, ty);
                                let _ = param_values.push(value);
                            }
                        }
                        conn.execute_statement(statement, &param_values, &param_types)
                            .await
                            .map_err(dbal_error)?;
                    }
                    Ok::<i64, Thrown>(num_updated)
                }
                .await;
                // Drop temporary table
                let dropped = conn
                    .execute_statement(drop_temp_table_sql, &empty, &no_types)
                    .await
                    .map_err(dbal_error);
                let num_updated = outcome?;
                dropped?;
                Ok(Executed::Count(num_updated))
            }
        }
    }
}

/// The statements of an executor, as a list.
fn statements(sql: &SqlStatements) -> Vec<&String> {
    match sql {
        SqlStatements::One(sql) => vec![sql],
        SqlStatements::Many(sql) => sql.iter().collect(),
    }
}

/// `[...$primaryClass->parentClasses, $primaryClass->name, ...$primaryClass->subClasses]`.
fn class_names(primary_class: &crate::mapping::ClassMetadata) -> Vec<String> {
    let info = primary_class.read();
    let mut names = info.parent_classes.clone();
    names.push(info.name.clone());
    names.extend(info.sub_classes.iter().cloned());
    names
}

/// `$sqlWalker->walkFromClause(new FromClause([new IdentificationVariableDeclaration(new RangeVariableDeclaration(...), null, [])]))`.
fn walk_primary_from(
    sql_walker: &mut dyn SqlWalker<'_>,
    class_name: &str,
    dql_alias: &str,
) -> Result<String, Thrown> {
    let from_clause = FromClause {
        identification_variable_declarations: vec![IdentificationVariableDeclaration {
            range_variable_declaration: Some(RangeVariableDeclaration::new(class_name, dql_alias)),
            index_by: None,
            joins: Vec::new(),
        }],
    };
    sql_walker.walk_from_clause(&from_clause)
}

/// The temporary id table of the multi-table executors.
struct TempTable {
    table: String,
    id_column_names: Vec<String>,
    id_column_list: String,
    root_class: std::sync::Arc<crate::mapping::ClassMetadata>,
}

impl TempTable {
    fn new(
        sql_walker: &mut dyn SqlWalker<'_>,
        root_class: &std::sync::Arc<crate::mapping::ClassMetadata>,
    ) -> Result<Self, Thrown> {
        let platform = sql_walker.get_entity_manager().get_database_platform();
        let table =
            platform.get_temporary_table_name(&root_class.read().get_temporary_id_table_name()?);
        let id_column_names = root_class.read().get_identifier_column_names();
        let id_column_list = id_column_names.join(", ");
        Ok(Self {
            table,
            id_column_names,
            id_column_list,
            root_class: root_class.clone(),
        })
    }

    fn insert_prefix(&self) -> String {
        format!(
            "INSERT INTO {} ({}) SELECT t0.{}",
            self.table,
            self.id_column_list,
            self.id_column_names.join(", t0.")
        )
    }

    fn create_and_drop(
        &self,
        sql_walker: &mut dyn SqlWalker<'_>,
    ) -> Result<(String, String), Thrown> {
        let em = sql_walker.get_entity_manager();
        let platform = em.get_database_platform();
        let mut column_definitions = Vec::new();
        for id_column_name in &self.id_column_names {
            let type_name = utility::get_type_of_column(id_column_name, &self.root_class, em)?;
            let type_ = php2rust_doctrine_dbal::types::get_type(&type_name)
                .map_err(|e| dbal_error(e.into()))?;
            let mut column = PhpArray::new();
            column.insert("name", Value::from(id_column_name.as_str()));
            column.insert("notnull", Value::Bool(true));
            column.insert("type", TypeObject::value(type_));
            column_definitions.push(column);
        }
        let create = format!(
            "{} {} ({}, PRIMARY KEY({}))",
            platform.get_create_temporary_table_snippet_sql(),
            self.table,
            platform
                .get_column_declaration_list_sql(&column_definitions)
                .map_err(dbal_error)?,
            self.id_column_names.join(",")
        );
        let drop = platform.get_drop_temporary_table_sql(&self.table);
        Ok((create, drop))
    }
}

/// `Doctrine\ORM\Query\Exec\SqlFinalizer`: creates the executor once the
/// query's first/max results and lock mode are known.
pub trait SqlFinalizer: fmt::Debug {
    /// The finalizer's class.
    fn class(&self) -> &str;

    /// `createExecutor($query)`.
    fn create_executor(&self, query: &dyn DqlQuery) -> Result<SqlExecutor, Thrown>;

    /// The finalizer as a `SingleSelectSqlFinalizer`.
    fn as_single_select(&self) -> Option<&SingleSelectSqlFinalizer> {
        None
    }
}

/// `SingleSelectSqlFinalizer`: a SELECT statement, to which the query's
/// limit, offset and lock mode are added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SingleSelectSqlFinalizer {
    sql: String,
}

impl SingleSelectSqlFinalizer {
    pub fn new(sql: String) -> Self {
        Self { sql }
    }

    /// The SQL before finalization.
    pub fn sql(&self) -> &str {
        &self.sql
    }

    /// This method exists temporarily to support old `SqlWalker` interfaces.
    pub fn finalize_sql(&self, query: &dyn DqlQuery) -> Result<String, Thrown> {
        let platform = query.get_entity_manager().get_database_platform();
        let mut sql = platform
            .modify_limit_query(&self.sql, query.get_max_results(), query.get_first_result())
            .map_err(|e| dbal_error(e.into()))?;
        let Some(lock_mode) = lock_mode_hint(query) else {
            return Err(query_exception::invalid_lock_mode());
        };
        if lock_mode == LockMode::PessimisticRead {
            sql.push(' ');
            sql.push_str(utility::get_read_lock_sql(platform));
        } else if lock_mode == LockMode::PessimisticWrite {
            sql.push(' ');
            sql.push_str(utility::get_write_lock_sql(platform));
        }
        Ok(sql)
    }
}

impl SqlFinalizer for SingleSelectSqlFinalizer {
    fn class(&self) -> &'static str {
        "Doctrine\\ORM\\Query\\Exec\\SingleSelectSqlFinalizer"
    }

    fn create_executor(&self, query: &dyn DqlQuery) -> Result<SqlExecutor, Thrown> {
        Ok(SqlExecutor::finalized_select(self.finalize_sql(query)?))
    }

    fn as_single_select(&self) -> Option<&SingleSelectSqlFinalizer> {
        Some(self)
    }
}

/// `PreparedExecutorFinalizer`: `PreparedExecutorFinalizer` is a wrapper
/// for the SQL finalization phase that does nothing - it is constructed
/// with the sql executor already.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedExecutorFinalizer {
    executor: SqlExecutor,
}

impl PreparedExecutorFinalizer {
    pub fn new(executor: SqlExecutor) -> Self {
        Self { executor }
    }
}

impl SqlFinalizer for PreparedExecutorFinalizer {
    fn class(&self) -> &'static str {
        "Doctrine\\ORM\\Query\\Exec\\PreparedExecutorFinalizer"
    }

    fn create_executor(&self, _query: &dyn DqlQuery) -> Result<SqlExecutor, Thrown> {
        Ok(self.executor.clone())
    }
}
