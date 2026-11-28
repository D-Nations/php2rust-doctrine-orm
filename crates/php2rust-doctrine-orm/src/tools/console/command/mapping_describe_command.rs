//! Port of `Doctrine\ORM\Tools\Console\Command\MappingDescribeCommand` and
//! `MappingDescribeCommandFormat`: shows the mapping of an entity.
//!
//! The metadata's properties are read as PHP values
//! (`ClassMetadataInfo::property_value()`), and `(array) $mapping` is a
//! mapping's initialized properties in declaration order.
//!
//! Upstream: doctrine/orm src/Tools/Console/Command/MappingDescribeCommand.php src/Tools/Console/Command/MappingDescribeCommandFormat.php

use std::sync::Arc;

use php_regex::{Pcre, preg_quote};
use php_value::json::{JSON_PRETTY_PRINT, JSON_UNESCAPED_SLASHES, JSON_UNESCAPED_UNICODE, encode};
use php_value::{PhpArray, Value};
use symfony_console::command::command::{Command, CommandHandler};
use symfony_console::completion::completion_input::CompletionInput;
use symfony_console::completion::suggestion::CompletionSuggestions;
use symfony_console::exception::Result;
use symfony_console::helper::table::Row;
use symfony_console::helper::table_cell::Cell;
use symfony_console::input::SuggestedValues;
use symfony_console::input::input::Input;
use symfony_console::input::input_argument::REQUIRED;
use symfony_console::input::input_option::{Shortcut, VALUE_REQUIRED};
use symfony_console::output::output::OutputInterface;
use symfony_console::style::symfony_style::SymfonyStyle;

use super::abstract_entity_manager_command::{
    AbstractEntityManagerCommand, exception, thrown, to_string,
};
use crate::entity_manager::EntityManager;
use crate::mapping::ClassMetadata;
use crate::tools::console::entity_manager_provider::EntityManagerProvider;
use php2rust_doctrine_persistence::mapping::mapping_exception::is_mapping_exception;

/// `MappingDescribeCommand::class`.
pub const CLASS: &str = "Doctrine\\ORM\\Tools\\Console\\Command\\MappingDescribeCommand";
/// `MappingDescribeCommandFormat::class`.
pub const FORMAT_CLASS: &str =
    "Doctrine\\ORM\\Tools\\Console\\Command\\MappingDescribeCommandFormat";

/// `MappingDescribeCommandFormat`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingDescribeCommandFormat {
    Text,
    Json,
}

impl MappingDescribeCommandFormat {
    /// `cases()`.
    pub const CASES: [Self; 2] = [Self::Text, Self::Json];

    /// `->value`.
    pub fn value(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
        }
    }

    /// `from($value)`.
    pub fn from(value: &Value) -> Result<Self> {
        let text = to_string(value);
        Self::CASES
            .into_iter()
            .find(|case| case.value() == text)
            .ok_or_else(|| {
                exception(
                    "ValueError",
                    format!("\"{text}\" is not a valid backing value for enum {FORMAT_CLASS}"),
                )
            })
    }

    fn values() -> Vec<Value> {
        Self::CASES.iter().map(|c| Value::from(c.value())).collect()
    }
}

const HELP: &str = r"The %command.full_name% command describes the metadata for the given full or partial entity class name.

    <info>%command.full_name%</info> My\Namespace\Entity\MyEntity

Or:

    <info>%command.full_name%</info> MyEntity
    
To output the metadata in JSON format, use the <info>--format</info> option:
  <info>%command.full_name% My\Namespace\Entity\MyEntity --format=json</info>

To use a specific entity manager (e.g., for multi-DB projects), use the <info>--em</info> option:
  <info>%command.full_name% My\Namespace\Entity\MyEntity --em=my_custom_entity_manager</info>
";

const JSON_FLAGS: i64 = JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES | JSON_PRETTY_PRINT;

/// `MappingDescribeCommand`.
pub struct MappingDescribeCommand(AbstractEntityManagerCommand);

impl MappingDescribeCommand {
    /// `new MappingDescribeCommand($entityManagerProvider)`: the command with this handler.
    pub fn command(entity_manager_provider: Arc<dyn EntityManagerProvider>) -> Result<Command> {
        Command::with_handler(
            Self(AbstractEntityManagerCommand::new(entity_manager_provider)),
            None,
        )
    }

    /// `displayEntity()`.
    #[allow(clippy::too_many_lines, reason = "one line per property, as upstream")]
    fn display_entity(
        entity_name: &str,
        entity_manager: &EntityManager,
        ui: &mut SymfonyStyle<'_>,
        format: MappingDescribeCommandFormat,
    ) -> Result<()> {
        let metadata = Self::get_class_metadata(entity_name, entity_manager)?;
        let metadata = metadata.read();
        let property = |name: &str| metadata.property_value(name).unwrap_or_default();

        if format == MappingDescribeCommandFormat::Json {
            let mut json = PhpArray::new();
            for name in [
                "name",
                "rootEntityName",
                "customGeneratorDefinition",
                "customRepositoryClassName",
                "isMappedSuperclass",
                "isEmbeddedClass",
                "parentClasses",
                "subClasses",
                "embeddedClasses",
                "identifier",
                "inheritanceType",
                "discriminatorColumn",
                "discriminatorValue",
                "discriminatorMap",
                "generatorType",
                "table",
                "isIdentifierComposite",
                "containsForeignIdentifier",
                "containsEnumIdentifier",
                "sequenceGeneratorDefinition",
                "changeTrackingPolicy",
                "isVersioned",
                "versionField",
                "isReadOnly",
                "entityListeners",
            ] {
                let value = property(name);
                let value = match name {
                    "customGeneratorDefinition"
                    | "discriminatorColumn"
                    | "table"
                    | "sequenceGeneratorDefinition" => format_value_as_json(&value),
                    _ => value,
                };
                json.insert(name, value);
            }
            json.insert(
                "associationMappings",
                format_mappings_as_json(&association_mappings(&metadata)),
            );
            json.insert(
                "fieldMappings",
                format_mappings_as_json(&field_mappings(&metadata)),
            );
            ui.text(&[&json_encode(&Value::Array(json))?])?;

            return Ok(());
        }

        let mut rows = vec![
            format_field("Name", &property("name")),
            format_field("Root entity name", &property("rootEntityName")),
            format_field(
                "Custom generator definition",
                &property("customGeneratorDefinition"),
            ),
            format_field(
                "Custom repository class",
                &property("customRepositoryClassName"),
            ),
            format_field("Mapped super class?", &property("isMappedSuperclass")),
            format_field("Embedded class?", &property("isEmbeddedClass")),
            format_field("Parent classes", &property("parentClasses")),
            format_field("Sub classes", &property("subClasses")),
            format_field("Embedded classes", &property("embeddedClasses")),
            format_field("Identifier", &property("identifier")),
            format_field("Inheritance type", &property("inheritanceType")),
            format_field("Discriminator column", &property("discriminatorColumn")),
            format_field("Discriminator value", &property("discriminatorValue")),
            format_field("Discriminator map", &property("discriminatorMap")),
            format_field("Generator type", &property("generatorType")),
            format_field("Table", &property("table")),
            format_field("Composite identifier?", &property("isIdentifierComposite")),
            format_field(
                "Foreign identifier?",
                &property("containsForeignIdentifier"),
            ),
            format_field("Enum identifier?", &property("containsEnumIdentifier")),
            format_field(
                "Sequence generator definition",
                &property("sequenceGeneratorDefinition"),
            ),
            format_field("Change tracking policy", &property("changeTrackingPolicy")),
            format_field("Versioned?", &property("isVersioned")),
            format_field("Version field", &property("versionField")),
            format_field("Read only?", &property("isReadOnly")),
            format_entity_listeners(&property("entityListeners")),
        ];
        rows.push(format_field("Association mappings:", &Value::from("")));
        rows.extend(format_mappings(&association_mappings(&metadata)));
        rows.push(format_field("Field mappings:", &Value::from("")));
        rows.extend(format_mappings(&field_mappings(&metadata)));

        let rows = rows
            .into_iter()
            .map(|row| row.map(|[label, value]| Row::cells([label, value])))
            .collect::<Result<Vec<_>>>()?;
        ui.table(vec![Cell::from("Field"), Cell::from("Value")], rows)
    }

    /// `getMappedEntities()`.
    fn get_mapped_entities(entity_manager: &EntityManager) -> Result<Vec<String>> {
        let entity_class_names = entity_manager
            .get_configuration()
            .get_metadata_driver_impl()
            .ok_or_else(|| {
                exception(
                    "Error",
                    "Call to a member function getAllClassNames() on null",
                )
            })?
            .get_all_class_names()
            .map_err(thrown)?;

        if entity_class_names.is_empty() {
            return Err(exception(
                "InvalidArgumentException",
                "You do not have any mapped Doctrine ORM entities according to the current configuration. \
                 If you have entities or mapping files you should check your mapping configuration for errors.",
            ));
        }

        Ok(entity_class_names)
    }

    /// `getClassMetadata()`.
    fn get_class_metadata(
        entity_name: &str,
        entity_manager: &EntityManager,
    ) -> Result<Arc<ClassMetadata>> {
        match entity_manager.get_class_metadata(entity_name) {
            Ok(metadata) => return Ok(metadata),
            Err(e) if is_mapping_exception(&e) => {}
            Err(e) => return Err(thrown(e)),
        }

        let mut pattern = b"{".to_vec();
        pattern.extend(preg_quote(entity_name.as_bytes(), None));
        pattern.push(b'}');
        let pattern =
            Pcre::new(&pattern).map_err(|e| exception("RuntimeException", format!("{e:?}")))?;
        let mut matches = Vec::new();
        for mapped_entity in Self::get_mapped_entities(entity_manager)? {
            if pattern
                .is_match(mapped_entity.as_bytes())
                .map_err(|e| exception("RuntimeException", format!("{e:?}")))?
            {
                matches.push(mapped_entity);
            }
        }

        if matches.is_empty() {
            return Err(exception(
                "InvalidArgumentException",
                format!("Could not find any mapped Entity classes matching \"{entity_name}\""),
            ));
        }

        if matches.len() > 1 {
            return Err(exception(
                "InvalidArgumentException",
                format!(
                    "Entity name \"{entity_name}\" is ambiguous, possible matches: \"{}\"",
                    matches.join(", ")
                ),
            ));
        }

        entity_manager
            .get_class_metadata(&matches[0])
            .map_err(thrown)
    }
}

impl CommandHandler for MappingDescribeCommand {
    fn class(&self) -> &str {
        CLASS
    }

    fn configure(&mut self, command: &mut Command) -> Result<()> {
        command
            .set_name("orm:mapping:describe")?
            .add_argument(
                "entityName",
                Some(REQUIRED),
                "Full or partial name of entity",
                Value::Null,
                SuggestedValues::None,
            )?
            .set_description("Display information about mapped objects")
            .add_option(
                "em",
                Shortcut::None,
                Some(VALUE_REQUIRED),
                "Name of the entity manager to operate on",
                Value::Null,
                SuggestedValues::None,
            )?
            .add_option(
                "format",
                Shortcut::None,
                Some(VALUE_REQUIRED),
                "Output format (text, json)",
                Value::from(MappingDescribeCommandFormat::Text.value()),
                SuggestedValues::List(MappingDescribeCommandFormat::values()),
            )?
            .set_help(HELP);
        Ok(())
    }

    fn execute(
        &mut self,
        _command: &mut Command,
        input: &mut Input,
        output: &mut dyn OutputInterface,
    ) -> Result<i64> {
        let format = MappingDescribeCommandFormat::from(&input.get_option("format")?)?;
        let entity_manager = self.0.get_entity_manager(input)?;
        let entity_name = to_string(&input.get_argument("entityName")?);
        let mut ui = SymfonyStyle::new(input, output)?;

        Self::display_entity(&entity_name, &entity_manager, &mut ui, format)?;

        Ok(0)
    }

    fn complete(
        &mut self,
        _command: &mut Command,
        input: &CompletionInput,
        suggestions: &mut CompletionSuggestions,
    ) -> Result<()> {
        if input.must_suggest_argument_values_for("entityName") {
            let entity_manager = self.0.get_entity_manager(input.input())?;

            let entities: Vec<Value> = Self::get_mapped_entities(&entity_manager)?
                .iter()
                .map(|fqcn| Value::from(fqcn.replace('\\', "\\\\")))
                .collect();

            suggestions.suggest_values(&entities);
        }

        if input.must_suggest_option_values_for("format") {
            suggestions.suggest_values(&MappingDescribeCommandFormat::values());
        }

        Ok(())
    }
}

/// `(array) $mapping` of each association mapping, by name.
fn association_mappings(metadata: &crate::mapping::ClassMetadataInfo) -> Vec<(String, PhpArray)> {
    metadata
        .association_mappings
        .iter()
        .map(|(name, mapping)| {
            let mut array = PhpArray::new();
            for property in mapping.class.properties() {
                if let Some(value) = mapping.property(property) {
                    array.insert(property, value);
                }
            }
            (name.to_owned(), array)
        })
        .collect()
}

/// `(array) $mapping` of each field mapping, by name.
fn field_mappings(metadata: &crate::mapping::ClassMetadataInfo) -> Vec<(String, PhpArray)> {
    metadata
        .field_mappings
        .iter()
        .map(|(name, mapping)| (name.to_owned(), mapping.to_array()))
        .collect()
}

/// `json_encode($value, JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES
/// | JSON_PRETTY_PRINT | JSON_THROW_ON_ERROR)`.
fn json_encode(value: &Value) -> Result<String> {
    let encoded =
        encode(value, JSON_FLAGS, 512).map_err(|e| exception("JsonException", format!("{e:?}")))?;
    match (encoded.json, encoded.error) {
        (Some(json), None) => Ok(json),
        (_, error) => Err(exception("JsonException", format!("{error:?}"))),
    }
}

/// `get_debug_type($value)` of an object.
fn debug_type(value: &Value) -> String {
    match value {
        Value::Object(object) => object.class().to_owned(),
        _ => String::new(),
    }
}

/// `empty($value)`.
fn is_empty(value: &Value) -> bool {
    match value {
        Value::Array(array) => array.is_empty(),
        Value::Object(_) => false,
        other => !other.is_truthy(),
    }
}

/// `formatValue()`.
fn format_value(value: &Value) -> Result<String> {
    if matches!(value, Value::String(s) if s.as_bytes().is_empty()) {
        return Ok(String::new());
    }

    if value.is_null() {
        return Ok("<comment>Null</comment>".to_owned());
    }

    if let Value::Bool(b) = value {
        return Ok(format!(
            "<comment>{}</comment>",
            if *b { "True" } else { "False" }
        ));
    }

    if is_empty(value) {
        return Ok("<comment>Empty</comment>".to_owned());
    }

    if let Value::Array(_) = value {
        return json_encode(value);
    }

    if let Value::Object(_) = value {
        return Ok(format!("<{}>", debug_type(value)));
    }

    value
        .to_php_string()
        .map(|s| s.to_string_lossy().into_owned())
        .ok_or_else(|| {
            exception(
                "InvalidArgumentException",
                "Do not know how to format value",
            )
        })
}

/// `formatValueAsJson()`.
fn format_value_as_json(value: &Value) -> Value {
    let value = match value {
        Value::Object(object) => Value::Array(object.properties()),
        other => other.clone(),
    };

    if let Value::Array(array) = value {
        let mut out = PhpArray::new();
        for (key, item) in array.iter() {
            out.insert(key.clone(), format_value_as_json(item));
        }
        return Value::Array(out);
    }

    value
}

/// `formatField()`.
fn format_field(label: &str, value: &Value) -> Result<[String; 2]> {
    let value = if value.is_null() {
        Value::from("<comment>None</comment>")
    } else {
        value.clone()
    };

    Ok([format!("<info>{label}</info>"), format_value(&value)?])
}

/// `formatMappings()`.
fn format_mappings(property_mappings: &[(String, PhpArray)]) -> Vec<Result<[String; 2]>> {
    let mut output = Vec::new();

    for (property_name, mapping) in property_mappings {
        output.push(format_field(
            &format!("  {property_name}"),
            &Value::from(""),
        ));

        for (field, value) in mapping.iter() {
            output.push(format_value(value).and_then(|formatted| {
                format_field(
                    &format!("    {}", to_string(&Value::from(key_to_string(field)))),
                    &Value::from(formatted),
                )
            }));
        }
    }

    output
}

fn key_to_string(key: &php_value::Key) -> String {
    match key {
        php_value::Key::Int(i) => i.to_string(),
        php_value::Key::Str(s) => s.to_string_lossy().into_owned(),
    }
}

/// `formatMappingsAsJson()`.
fn format_mappings_as_json(property_mappings: &[(String, PhpArray)]) -> Value {
    let mut output = PhpArray::new();

    for (property_name, mapping) in property_mappings {
        output.insert(
            property_name.as_str(),
            format_value_as_json(&Value::Array(mapping.clone())),
        );
    }

    Value::Array(output)
}

/// `formatEntityListeners()`.
fn format_entity_listeners(entity_listeners: &Value) -> Result<[String; 2]> {
    let mut listeners = Vec::new();

    if let Value::Array(entity_listeners) = entity_listeners {
        for (event_name, event_listeners) in entity_listeners.iter() {
            let Value::Array(event_listeners) = event_listeners else {
                continue;
            };
            for (_, listener) in event_listeners.iter() {
                let Value::Array(listener) = listener else {
                    continue;
                };
                let get = |k: &str| {
                    listener
                        .get(&php_value::Key::from(k))
                        .map_or_default(to_string)
                };
                listeners.push(Value::from(format!(
                    "{}: {}::{}",
                    key_to_string(event_name),
                    get("class"),
                    get("method")
                )));
            }
        }
    }

    format_field("Entity listeners", &Value::Array(PhpArray::list(listeners)))
}
