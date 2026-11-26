//! The joined subclass persister maps a single entity instance to several
//! tables in the database as it is defined by the <tt>Class Table
//! Inheritance</tt> strategy.
//!
//! `BasicEntityPersister` with `PersisterKind::JoinedSubclass` runs these
//! overrides. Lock modes and filters are not ported, as in the parent: the
//! lock SQL is empty and there is no filter SQL.
//!
//! @see <https://martinfowler.com/eaaCatalog/classTableInheritance.html>
//!
//! Upstream: doctrine/orm src/Persisters/Entity/JoinedSubclassPersister.php

use std::fmt::Write as _;
use std::sync::Arc;

use php_value::{Key, Object, PhpArray, Thrown, Value};
use php2rust_doctrine_dbal::platforms::AbstractPlatform;
use php2rust_doctrine_dbal::{LockMode, types};

use crate::entity_manager::EntityManager;
use crate::exception;
use crate::id::IdGenerator;
use crate::mapping::ClassMetadata;
use crate::mapping::quote_strategy::get_sql_result_casing;
use crate::utility::{ParamType, flatten_identifier, get_type_of_column, key_text, to_text};

use super::basic_entity_persister::{BasicEntityPersister, PersisterCriteria, dbal, types_of};

impl BasicEntityPersister {
    /// This function finds the `ClassMetadata` instance in an inheritance
    /// hierarchy that is responsible for enabling versioning.
    fn get_versioned_class_metadata(
        &self,
        em: &EntityManager,
    ) -> Result<Arc<ClassMetadata>, Thrown> {
        let inherited = {
            let info = self.class.read();
            info.version_field.as_ref().and_then(|version_field| {
                info.field_mappings
                    .get(version_field)
                    .and_then(|m| m.inherited.clone())
            })
        };
        match inherited {
            Some(defining_class_name) => em.get_class_metadata(&defining_class_name),
            None => Ok(self.class.clone()),
        }
    }

    /// Gets the name of the table that owns the column the given field is
    /// mapped to.
    pub(super) fn joined_get_owning_table(
        &self,
        em: &EntityManager,
        field_name: &str,
    ) -> Result<String, Thrown> {
        if let Some(table) = self.st().owning_table_map.get(field_name) {
            return Ok(table.clone());
        }
        let inherited = {
            let info = self.class.read();
            match info.association_mappings.get(field_name) {
                Some(assoc) if assoc.inherited.is_some() => assoc.inherited.clone(),
                _ => info
                    .field_mappings
                    .get(field_name)
                    .and_then(|m| m.inherited.clone()),
            }
        };
        let cm = match inherited {
            Some(class_name) => em.get_class_metadata(&class_name)?,
            None => self.class.clone(),
        };
        let table_name = cm.read().get_table_name()?;
        let quoted_table_name = self
            .quote_strategy
            .get_table_name(&cm.read(), em.get_platform());
        let mut st = self.st();
        st.owning_table_map
            .insert(field_name.to_owned(), table_name.clone());
        st.quoted_table_map
            .insert(table_name.clone(), quoted_table_name);
        Ok(table_name)
    }

    /// `JoinedSubclassPersister::executeInserts()`.
    #[allow(clippy::too_many_lines, reason = "upstream's method, in its order")]
    pub(super) async fn joined_execute_inserts(&self) -> Result<(), Thrown> {
        if self.st().queued_inserts.is_empty() {
            return Ok(());
        }
        let em = self.em()?;
        let uow = em.get_unit_of_work();
        let (is_post_insert_id, class_name, root_name, parent_classes, requires_fetch) = {
            let info = self.class.read();
            (
                info.id_generator
                    .as_ref()
                    .is_some_and(IdGenerator::is_post_insert_generator),
                info.name.clone(),
                info.root_entity_name.clone(),
                info.parent_classes.clone(),
                info.requires_fetch_after_change,
            )
        };
        let root_class = if class_name == root_name {
            self.class.clone()
        } else {
            em.get_class_metadata(&root_name)?
        };

        // Prepare statement for the root table
        let root_persister = uow.get_entity_persister(&root_class.read().name.clone())?;
        let root_table_name = root_class.read().get_table_name()?;
        let root_table_stmt = root_persister.get_insert_sql(&em)?;

        // Prepare statements for sub tables.
        let mut sub_table_stmts: Vec<(String, String)> = Vec::new();
        if class_name != root_name {
            sub_table_stmts.push((
                self.class.read().get_table_name()?,
                self.get_insert_sql(&em)?,
            ));
        }
        for parent_class_name in &parent_classes {
            let parent_class = em.get_class_metadata(parent_class_name)?;
            let parent_table_name = parent_class.read().get_table_name()?;
            if *parent_class_name != root_name {
                let parent_persister = uow.get_entity_persister(parent_class_name)?;
                sub_table_stmts.push((parent_table_name, parent_persister.get_insert_sql(&em)?));
            }
        }

        let conn = em.get_connection();
        let string_type: ParamType = php2rust_doctrine_dbal::ParameterType::String.into();

        // Execute all inserts. For each entity:
        // 1) Insert on root table
        // 2) Insert on sub tables
        let queued: Vec<(usize, Object)> = self.st().queued_inserts.clone();
        for (key, entity) in queued {
            let insert_data = self.prepare_insert_data(&em, &entity)?;
            let table_data = |table: &str| match insert_data.get(&Key::from(table)) {
                Some(Value::Array(data)) => data.clone(),
                _ => PhpArray::new(),
            };

            // Execute insert on root table
            let column_types = self.st().column_types.clone();
            let type_of = |column: &str| {
                column_types
                    .get(column)
                    .map_or_else(|| string_type.clone(), |t| t.as_str().into())
            };
            let mut params = Vec::new();
            let mut param_types = Vec::new();
            for (column_name, value) in table_data(&root_table_name).iter() {
                params.push(value.clone());
                param_types.push(type_of(&key_text(column_name)));
            }
            conn.execute_statement(
                &root_table_stmt,
                &PhpArray::list(params),
                &types_of(param_types),
            )
            .await
            .map_err(|e| dbal(&e))?;

            let id = if is_post_insert_id {
                // IdentityGenerator::generateId()
                let generated_id = conn.last_insert_id().await.map_err(|e| dbal(&e))?;
                let id_field = self
                    .class
                    .read()
                    .identifier
                    .first()
                    .cloned()
                    .unwrap_or_default();
                uow.assign_post_insert_id(&entity, &generated_id)?;
                let mut id = PhpArray::new();
                id.insert(id_field.as_str(), generated_id);
                id
            } else {
                uow.get_entity_identifier(&entity)?
            };

            // Execute inserts on subtables.
            // The order doesn't matter because all child tables link to the
            // root table via FK.
            for (table_name, stmt) in &sub_table_stmts {
                let data = table_data(table_name);
                let mut params = Vec::new();
                let mut param_types = Vec::new();
                for (id_name, id_val) in id.iter() {
                    params.push(id_val.clone());
                    param_types.push(
                        column_types
                            .get(&key_text(id_name))
                            .map_or_else(|| ParamType::from("string"), |t| t.as_str().into()),
                    );
                }
                for (column_name, value) in data.iter() {
                    if id.get(column_name).is_none() {
                        params.push(value.clone());
                        param_types.push(type_of(&key_text(column_name)));
                    }
                }
                conn.execute_statement(stmt, &PhpArray::list(params), &types_of(param_types))
                    .await
                    .map_err(|e| dbal(&e))?;
            }

            if requires_fetch {
                self.assign_default_version_and_upsertable_values(&em, &entity, &id)
                    .await?;
            }

            // Unset this queued insert, so that the prepareUpdateData()
            // method (called via prepareInsertData() method) knows right
            // away (for the next entity already) that the current entity
            // has been written to the database and no extra updates need to
            // be scheduled to refer to it.
            self.st().queued_inserts.retain(|(k, _)| *k != key);
        }
        Ok(())
    }

    /// `JoinedSubclassPersister::update()`.
    pub(super) async fn joined_update(&self, entity: &Object) -> Result<(), Thrown> {
        let em = self.em()?;
        let update_data = self.prepare_update_data(&em, entity, false)?;
        if update_data.is_empty() {
            return Ok(());
        }

        let (is_versioned, requires_fetch) = {
            let info = self.class.read();
            (info.is_versioned, info.requires_fetch_after_change)
        };
        let versioned_class = self.get_versioned_class_metadata(&em)?;
        let versioned_table = versioned_class.read().get_table_name()?;

        for (table_name, data) in update_data.iter() {
            let Value::Array(data) = data else {
                continue;
            };
            let table_name = key_text(table_name);
            let table_name = self
                .st()
                .quoted_table_map
                .get(&table_name)
                .cloned()
                .unwrap_or(table_name);
            let versioned = is_versioned && versioned_table == table_name;
            self.update_table(&em, entity, &table_name, data, versioned)
                .await?;
        }

        if requires_fetch {
            // Make sure the table with the version column is updated even
            // if no columns on that table were affected.
            if is_versioned
                && update_data
                    .get(&Key::from(versioned_table.as_str()))
                    .is_none()
            {
                let table_name = self
                    .quote_strategy
                    .get_table_name(&versioned_class.read(), em.get_platform());
                self.update_table(&em, entity, &table_name, &PhpArray::new(), true)
                    .await?;
            }

            let identifiers = em.get_unit_of_work().get_entity_identifier(entity)?;
            self.assign_default_version_and_upsertable_values(&em, entity, &identifiers)
                .await?;
        }
        Ok(())
    }

    /// `JoinedSubclassPersister::delete()`.
    pub(super) async fn joined_delete(&self, entity: &Object) -> Result<bool, Thrown> {
        let em = self.em()?;
        let platform = em.get_platform();
        let identifier = em.get_unit_of_work().get_entity_identifier(entity)?;
        let id_columns = self.class.read().get_identifier_column_names();
        let mut id = PhpArray::new();
        for (column, value) in id_columns.iter().zip(identifier.values()) {
            id.insert(column.as_str(), value.clone());
        }
        let types = self.get_class_identifiers_types(&em, &self.class)?;

        self.delete_join_table_records(&em, &identifier, &types)
            .await?;

        // Delete the row from the root table. Cascades do the rest.
        let root = self.class.read().root_entity_name.clone();
        let root_class = em.get_class_metadata(&root)?;
        let root_table = self
            .quote_strategy
            .get_table_name(&root_class.read(), platform);
        let root_types = self.get_class_identifiers_types(&em, &root_class)?;

        let deleted = em
            .get_connection()
            .delete(&root_table, &id, &types_of(root_types))
            .await
            .map_err(|e| dbal(&e))?;
        Ok(deleted != 0)
    }

    /// `JoinedSubclassPersister::getSelectSQL()`.
    #[allow(
        clippy::too_many_arguments,
        reason = "getSelectSQL(), as PHP declares it"
    )]
    pub(super) fn joined_get_select_sql(
        &self,
        em: &Arc<EntityManager>,
        criteria: PersisterCriteria<'_>,
        assoc: Option<&crate::mapping::AssociationMapping>,
        lock_mode: Option<LockMode>,
        limit: Option<i64>,
        offset: Option<i64>,
        order_by: Option<&PhpArray>,
    ) -> Result<String, Thrown> {
        self.switch_persister_context(offset, limit);

        let platform = em.get_platform();
        let base_table_alias = self.get_sql_table_alias(&self.class.read().name.clone(), "");
        let mut join_sql = self.get_join_sql(em, &base_table_alias)?;

        if let Some(assoc) = assoc
            && assoc.is_many_to_many()
        {
            join_sql.push_str(&self.get_select_many_to_many_join_sql(em, assoc)?);
        }

        let condition_sql = match criteria {
            PersisterCriteria::Criteria(criteria) => {
                self.get_select_condition_criteria_sql(em, criteria)?
            }
            PersisterCriteria::Array(criteria) => {
                self.get_select_condition_sql(em, criteria, assoc)?
            }
        };

        let mut order_by = order_by.cloned();
        if let Some(assoc) = assoc
            && assoc.is_ordered()
        {
            order_by = Some(assoc.order_by().clone());
        }

        let mut order_by_sql = String::new();
        if let Some(order_by) = order_by.filter(|o| !o.is_empty()) {
            order_by_sql = self.get_order_by_sql(em, &order_by, &base_table_alias)?;
        }

        let table_name = self
            .quote_strategy
            .get_table_name(&self.class.read(), platform);
        let from = format!(" FROM {table_name} {base_table_alias}");
        let where_ = if condition_sql.is_empty() {
            String::new()
        } else {
            format!(" WHERE {condition_sql}")
        };
        let lock = platform.append_lock_hint(&from, lock_mode.unwrap_or(LockMode::None));
        let column_list = self.get_select_columns_sql(em)?;
        let query = format!("SELECT {column_list}{lock}{join_sql}{where_}{order_by_sql}");

        platform
            .modify_limit_query(&query, limit, offset.unwrap_or(0))
            .map_err(|e| {
                Thrown::new(
                    "Doctrine\\DBAL\\Exception\\InvalidArgumentException",
                    e.message(),
                    0,
                )
            })
    }

    /// `JoinedSubclassPersister::getCountSQL()`.
    pub(super) fn joined_get_count_sql(
        &self,
        em: &Arc<EntityManager>,
        criteria: PersisterCriteria<'_>,
    ) -> Result<String, Thrown> {
        let table_name = self
            .quote_strategy
            .get_table_name(&self.class.read(), em.get_platform());
        let base_table_alias = self.get_sql_table_alias(&self.class.read().name.clone(), "");
        let join_sql = self.get_join_sql(em, &base_table_alias)?;

        let condition_sql = match criteria {
            PersisterCriteria::Criteria(criteria) => {
                self.get_select_condition_criteria_sql(em, criteria)?
            }
            PersisterCriteria::Array(criteria) => {
                self.get_select_condition_sql(em, criteria, None)?
            }
        };

        Ok(format!(
            "SELECT COUNT(*) FROM {table_name} {base_table_alias}{join_sql}{}",
            if condition_sql.is_empty() {
                String::new()
            } else {
                format!(" WHERE {condition_sql}")
            }
        ))
    }

    /// The INNER JOINs of the parent tables that `getLockTablesSql()`
    /// appends to the parent's FROM.
    pub(super) fn joined_lock_tables_join_sql(&self, em: &EntityManager) -> Result<String, Thrown> {
        let mut join_sql = String::new();
        let (identifier_columns, parent_classes, class_name) = {
            let info = self.class.read();
            (
                info.get_identifier_column_names(),
                info.parent_classes.clone(),
                info.name.clone(),
            )
        };
        let base_table_alias = self.get_sql_table_alias(&class_name, "");

        // INNER JOIN parent tables
        for parent_class_name in &parent_classes {
            let table_alias = self.get_sql_table_alias(parent_class_name, "");
            let parent_class = em.get_class_metadata(parent_class_name)?;
            let conditions: Vec<String> = identifier_columns
                .iter()
                .map(|id_column| {
                    format!("{base_table_alias}.{id_column} = {table_alias}.{id_column}")
                })
                .collect();
            let _ = write!(
                join_sql,
                " INNER JOIN {} {table_alias} ON {}",
                self.quote_strategy
                    .get_table_name(&parent_class.read(), em.get_platform()),
                conditions.join(" AND ")
            );
        }
        Ok(join_sql)
    }

    /// `JoinedSubclassPersister::getSelectColumnsSQL()`.
    #[allow(clippy::too_many_lines, reason = "upstream's method, in its order")]
    pub(super) fn joined_get_select_columns_sql(
        &self,
        em: &Arc<EntityManager>,
    ) -> Result<String, Thrown> {
        // Create the column list fragment only once
        if let Some(sql) = self.st().ctx().select_column_list_sql.clone() {
            return Ok(sql);
        }

        let platform = em.get_platform();
        let mut column_list = Vec::new();
        let (class_name, root_name, discr_column_name, discr_column_type, sub_classes) = {
            let info = self.class.read();
            let discr = info.get_discriminator_column()?;
            (
                info.name.clone(),
                info.root_entity_name.clone(),
                discr.name.clone(),
                discr.r#type.clone(),
                info.sub_classes.clone(),
            )
        };
        let base_table_alias = self.get_sql_table_alias(&class_name, "");
        let result_column_name = get_sql_result_casing(platform, &discr_column_name);

        {
            let mut st = self.st();
            let rsm = &mut st.ctx().rsm;
            rsm.add_entity_result(&class_name, "r", None);
            rsm.set_discriminator_column("r", &result_column_name);
            rsm.add_meta_result(
                "r",
                &result_column_name,
                &discr_column_name,
                false,
                Some(&discr_column_type),
            );
        }

        // Add regular columns
        let fields: Vec<(String, Option<String>)> = self
            .class
            .read()
            .field_mappings
            .iter()
            .map(|(field, mapping)| (field.to_owned(), mapping.inherited.clone()))
            .collect();
        for (field_name, inherited) in fields {
            let class = match inherited {
                Some(inherited) => em.get_class_metadata(&inherited)?,
                None => self.class.clone(),
            };
            column_list.push(self.get_select_column_sql(em, &field_name, &class, "r")?);
        }

        // Add foreign key columns
        let associations: Vec<_> = self
            .class
            .read()
            .association_mappings
            .values()
            .filter(|mapping| mapping.is_to_one_owning_side())
            .cloned()
            .collect();
        for mapping in associations {
            let table_alias = match &mapping.inherited {
                Some(inherited) => self.get_sql_table_alias(inherited, ""),
                None => base_table_alias.clone(),
            };
            let target_class = em.get_class_metadata(&mapping.target_entity)?;
            for join_column in &mapping.join_columns {
                let quoted = self.quote_strategy.get_join_column_name(
                    join_column,
                    &self.class.read(),
                    platform,
                );
                let ty = get_type_of_column(
                    &join_column.referenced_column_name,
                    &target_class.read(),
                    em,
                )?;
                column_list.push(self.get_select_join_column_sql(
                    em,
                    &table_alias,
                    &join_column.name,
                    &quoted,
                    &ty,
                ));
            }
        }

        // Add discriminator column (DO NOT ALIAS, see
        // AbstractEntityInheritancePersister#processSQLResult).
        let table_alias = if root_name == class_name {
            base_table_alias.clone()
        } else {
            self.get_sql_table_alias(&root_name, "")
        };
        column_list.push(format!("{table_alias}.{discr_column_name}"));

        // sub tables
        for sub_class_name in &sub_classes {
            let sub_class = em.get_class_metadata(sub_class_name)?;
            let table_alias = self.get_sql_table_alias(sub_class_name, "");

            // Add subclass columns
            let fields: Vec<String> = sub_class
                .read()
                .field_mappings
                .iter()
                .filter(|(_, mapping)| mapping.inherited.is_none())
                .map(|(field, _)| field.to_owned())
                .collect();
            for field_name in fields {
                column_list.push(self.get_select_column_sql(em, &field_name, &sub_class, "r")?);
            }

            // Add join columns (foreign keys)
            let associations: Vec<_> = sub_class
                .read()
                .association_mappings
                .values()
                .filter(|mapping| mapping.is_to_one_owning_side() && mapping.inherited.is_none())
                .cloned()
                .collect();
            for mapping in associations {
                let target_class = em.get_class_metadata(&mapping.target_entity)?;
                for join_column in &mapping.join_columns {
                    let quoted = self.quote_strategy.get_join_column_name(
                        join_column,
                        &sub_class.read(),
                        platform,
                    );
                    let ty = get_type_of_column(
                        &join_column.referenced_column_name,
                        &target_class.read(),
                        em,
                    )?;
                    column_list.push(self.get_select_join_column_sql(
                        em,
                        &table_alias,
                        &join_column.name,
                        &quoted,
                        &ty,
                    ));
                }
            }
        }

        let sql = column_list.join(", ");
        self.st().ctx().select_column_list_sql = Some(sql.clone());
        Ok(sql)
    }

    /// `JoinedSubclassPersister::getInsertColumnList()`.
    pub(super) fn joined_get_insert_column_list(
        &self,
        platform: &dyn AbstractPlatform,
    ) -> Vec<String> {
        let info = self.class.read();
        // Identifier columns must always come first in the column list of
        // subclasses.
        let mut columns = if info.parent_classes.is_empty() {
            Vec::new()
        } else {
            info.get_identifier_column_names()
        };

        for (name, _) in info.property_accessors.iter() {
            let field = info.field_mappings.get(name);
            let assoc = info.association_mappings.get(name);
            if (field.is_some_and(|f| f.inherited.is_some())
                && field.is_none_or(|f| f.id.is_none()))
                || assoc.is_some_and(|a| a.inherited.is_some())
                || (info.is_versioned && info.version_field.as_deref() == Some(name))
                || info.embedded_classes.contains_key(name)
                || field.is_some_and(|f| f.not_insertable.is_some())
            {
                continue;
            }

            if let Some(assoc) = assoc {
                if assoc.is_to_one_owning_side() {
                    for source_col in assoc.target_to_source_key_columns.values() {
                        columns.push(to_text(source_col));
                    }
                }
            } else if info.name != info.root_entity_name
                || !info.is_id_generator_identity()
                || info.identifier.first().map(String::as_str) != Some(name)
            {
                columns.push(self.quote_strategy.get_column_name(name, &info, platform));
                if let Some(field) = field {
                    self.st()
                        .column_types
                        .insert(name.to_owned(), field.r#type.clone());
                }
            }
        }

        // Add discriminator column if it is the topmost class.
        if info.name == info.root_entity_name
            && let Ok(discr) = info.get_discriminator_column()
        {
            columns.push(discr.name.clone());
        }
        columns
    }

    /// `JoinedSubclassPersister::assignDefaultVersionAndUpsertableValues()`.
    pub(super) async fn joined_assign_default_version_and_upsertable_values(
        &self,
        em: &Arc<EntityManager>,
        entity: &Object,
        id: &PhpArray,
    ) -> Result<(), Thrown> {
        let versioned_class = self.get_versioned_class_metadata(em)?;
        let values = self
            .joined_fetch_version_and_not_upsertable_values(em, &versioned_class, id)
            .await?;
        let platform = em.get_platform();
        for (field, value) in values {
            let ty = self
                .class
                .read()
                .field_mappings
                .get(&field)
                .map(|m| m.r#type.clone())
                .unwrap_or_default();
            let ty = types::get_type(&ty).map_err(|e| Thrown::new(e.class(), e.message(), 0))?;
            let value = ty
                .convert_to_php_value(&value, platform)
                .map_err(|e| exception::from_conversion(&e))?;
            self.class.read().set_field_value(entity, &field, value)?;
        }
        Ok(())
    }

    /// `JoinedSubclassPersister::fetchVersionAndNotUpsertableValues()`.
    async fn joined_fetch_version_and_not_upsertable_values(
        &self,
        em: &Arc<EntityManager>,
        versioned_class: &Arc<ClassMetadata>,
        id: &PhpArray,
    ) -> Result<Vec<(String, Value)>, Thrown> {
        let platform = em.get_platform();
        let mut column_names: Vec<(String, String)> = Vec::new();
        let fields: Vec<(String, bool, Option<String>)> = {
            let info = self.class.read();
            info.field_mappings
                .iter()
                .map(|(key, column)| {
                    (
                        key.to_owned(),
                        column.generated.is_some(),
                        column.inherited.clone(),
                    )
                })
                .collect()
        };
        let (is_versioned, version_field) = {
            let info = versioned_class.read();
            (self.class.read().is_versioned, info.version_field.clone())
        };
        for (key, generated, inherited) in fields {
            let class = if is_versioned && version_field.as_deref() == Some(key.as_str()) {
                versioned_class.clone()
            } else if generated {
                match inherited {
                    Some(inherited) => em.get_class_metadata(&inherited)?,
                    None => self.class.clone(),
                }
            } else {
                continue;
            };
            column_names.push((
                key.clone(),
                self.get_select_column_sql(em, &key, &class, "r")?,
            ));
        }

        let table_name = self
            .quote_strategy
            .get_table_name(&versioned_class.read(), platform);
        let base_table_alias = self.get_sql_table_alias(&self.class.read().name.clone(), "");
        let join_sql = self.get_join_sql(em, &base_table_alias)?;
        let identifier: Vec<String> = self
            .quote_strategy
            .get_identifier_column_names(&versioned_class.read(), platform)
            .into_iter()
            .map(|id_value| format!("{base_table_alias}.{id_value}"))
            .collect();

        let sql = format!(
            "SELECT {} FROM {table_name} {base_table_alias}{join_sql} WHERE {} = ?",
            column_names
                .iter()
                .map(|(_, c)| c.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            identifier.join(" = ? AND ")
        );

        let flat_id = flatten_identifier(em, &versioned_class.read(), id)?;
        let types = self.extract_identifier_types_of(em, id, versioned_class)?;
        let values = em
            .get_connection()
            .fetch_numeric(
                &sql,
                &PhpArray::list(flat_id.values().cloned()),
                &types_of(types),
            )
            .await
            .map_err(|e| dbal(&e))?;
        let Some(values) = values else {
            return Err(Thrown::new(
                "LengthException",
                "Unexpected empty result for database query.",
                0,
            ));
        };
        if values.len() != column_names.len() || values.is_empty() {
            return Err(Thrown::new(
                "LengthException",
                "Unexpected number of database columns.",
                0,
            ));
        }
        Ok(column_names
            .into_iter()
            .map(|(field, _)| field)
            .zip(values.values().cloned())
            .collect())
    }

    /// `getJoinSql()`: the INNER JOINs of the parent tables and the LEFT
    /// JOINs of the sub tables.
    fn get_join_sql(&self, em: &EntityManager, base_table_alias: &str) -> Result<String, Thrown> {
        let mut join_sql = String::new();
        let (identifier_column, parent_classes, sub_classes) = {
            let info = self.class.read();
            (
                info.get_identifier_column_names(),
                info.parent_classes.clone(),
                info.sub_classes.clone(),
            )
        };
        let platform = em.get_platform();

        // INNER JOIN parent tables
        for parent_class_name in &parent_classes {
            let parent_class = em.get_class_metadata(parent_class_name)?;
            let table_alias = self.get_sql_table_alias(parent_class_name, "");
            let conditions: Vec<String> = identifier_column
                .iter()
                .map(|id_column| {
                    format!("{base_table_alias}.{id_column} = {table_alias}.{id_column}")
                })
                .collect();
            let _ = write!(
                join_sql,
                " INNER JOIN {} {table_alias} ON {}",
                self.quote_strategy
                    .get_table_name(&parent_class.read(), platform),
                conditions.join(" AND ")
            );
        }

        // OUTER JOIN sub tables
        for sub_class_name in &sub_classes {
            let sub_class = em.get_class_metadata(sub_class_name)?;
            let table_alias = self.get_sql_table_alias(sub_class_name, "");
            let conditions: Vec<String> = identifier_column
                .iter()
                .map(|id_column| {
                    format!("{base_table_alias}.{id_column} = {table_alias}.{id_column}")
                })
                .collect();
            let _ = write!(
                join_sql,
                " LEFT JOIN {} {table_alias} ON {}",
                self.quote_strategy
                    .get_table_name(&sub_class.read(), platform),
                conditions.join(" AND ")
            );
        }
        Ok(join_sql)
    }
}
