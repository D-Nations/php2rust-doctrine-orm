//! Port of `StaticPHPMappingDriverTest`, and of the `MappingDriverTestCase`
//! tests it inherits (in `common/mapping_driver_test_case.rs`).
//!
//! The `StaticPHPDriver` calls a class's static `loadMetadata()`. Those
//! methods are PHP code, so each one is written below as a Rust fn of the
//! same calls on the metadata, and declared on the class
//! (`models::register_with_load_metadata`). A class without its own
//! method inherits its parent's, as PHP's static methods do.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/StaticPHPMappingDriverTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]
#![allow(
    clippy::unnecessary_wraps,
    clippy::too_many_lines,
    reason = "every loadMetadata() has the signature the driver calls, at upstream's length"
)]

#[path = "common/mapping_driver_test_case.rs"]
mod mapping_driver_test_case;
#[path = "common/naming.rs"]
mod naming;
#[path = "common/models.rs"]
mod recorded_models;

/// The models, with the static `loadMetadata()` methods below.
mod models {
    pub fn register() {
        super::recorded_models::register_with_load_metadata(super::loaders);
    }
}

use std::sync::Arc;

use php_value::{Key, PhpArray, Thrown, Value, php_array};
use php2rust_doctrine_orm::events;
use php2rust_doctrine_orm::mapping::class_metadata::{self as cm, sort_direction};
use php2rust_doctrine_orm::mapping::{ClassMetadata, ClassMetadataInfo};
use php2rust_doctrine_orm::query::SortDirection;
use php2rust_doctrine_persistence::mapping::driver::{
    MappingDriver, StaticPHPDriver, StaticPHPDriverPaths,
};
use php2rust_doctrine_persistence::php_reflection::LoadMetadataFn;

use mapping_driver_test_case::{create_class_metadata, model, ns};

/// Uses the XML driver (`$this->loadDriver() instanceof XmlDriver`).
pub const USES_XML_DRIVER: bool = false;

/// `loadDriver()`: `new StaticPHPDriver(__DIR__ . DIRECTORY_SEPARATOR . 'php')`.
pub fn load_driver() -> Arc<dyn MappingDriver> {
    models::register();
    let dir = format!("{}/tests/data/Mapping/php", env!("CARGO_MANIFEST_DIR"));
    Arc::new(StaticPHPDriver::new(StaticPHPDriverPaths::Paths(vec![dir])))
}

type Load = fn(&mut ClassMetadataInfo) -> Result<(), Thrown>;

/// `Class::loadMetadata(ClassMetadata $metadata)` as a `StaticPHPDriver`
/// call.
fn loader(load: Load) -> LoadMetadataFn {
    Arc::new(move |metadata| {
        let metadata = metadata
            .downcast_ref::<ClassMetadata>()
            .expect("an ORM ClassMetadata");
        load(&mut metadata.write())
    })
}

fn loaders() -> Vec<(String, LoadMetadataFn)> {
    let classes: Vec<(String, Load)> = vec![
        (ns("User"), user),
        (ns("UserIncorrectIndex"), user_incorrect_index),
        (
            ns("UserIncorrectUniqueConstraint"),
            user_incorrect_unique_constraint,
        ),
        (ns("Animal"), animal),
        (ns("Cat"), nothing),
        (ns("Dog"), nothing),
        (ns("DDC1170Entity"), ddc1170_entity),
        (ns("DDC807Entity"), ddc807_entity),
        (ns("Comment"), comment),
        (
            ns("SingleTableEntityNoDiscriminatorColumnMapping"),
            id_without_generator,
        ),
        (
            ns("SingleTableEntityIncompleteDiscriminatorColumnMapping"),
            id_without_generator,
        ),
        (
            ns("ReservedWordInTableColumn"),
            reserved_word_in_table_column,
        ),
        (ns("GH10288EnumTypePerson"), gh10288_enum_type_person),
        (ns("BlogPostComment"), blog_post_comment),
        (ns("UserWithStringOrderBy"), user_with_string_order_by),
        (ns("UserWithInvalidOrderBy"), user_with_invalid_order_by),
        (model("DDC964\\DDC964User"), ddc964_user),
        (model("DDC964\\DDC964Admin"), ddc964_admin),
        (model("DDC964\\DDC964Guest"), ddc964_guest),
        (model("CMS\\CmsAddress"), cms_address),
        (model("CMS\\CmsUser"), cms_user),
        (model("DDC869\\DDC869ChequePayment"), ddc869_cheque_payment),
        (
            model("DDC869\\DDC869CreditCardPayment"),
            ddc869_credit_card_payment,
        ),
        (model("DDC869\\DDC869Payment"), ddc869_payment),
        (model("Enums\\Card"), card),
        (model("TypedProperties\\UserTyped"), user_typed),
        (
            model("TypedProperties\\UserTypedWithCustomTypedField"),
            user_typed_with_custom_typed_field,
        ),
        (model("Upsertable\\Insertable"), insertable),
        (model("Upsertable\\Updatable"), updatable),
        (
            model("DDC1476\\DDC1476EntityWithDefaultFieldType"),
            ddc1476_entity_with_default_field_type,
        ),
        (model("Cache\\City"), city),
        (model("Company\\CompanyFixContract"), company_fix_contract),
        (model("Company\\CompanyFlexContract"), company_flex_contract),
        (model("Company\\CompanyContract"), company_contract),
        (model("Company\\CompanyPerson"), company_person),
        (
            model("Company\\CompanyFlexUltraContract"),
            company_flex_ultra_contract,
        ),
        (model("DDC5934\\DDC5934BaseContract"), ddc5934_base_contract),
        (model("DDC5934\\DDC5934Contract"), ddc5934_contract),
        (model("DDC889\\DDC889SuperClass"), ddc889_super_class),
        (model("DDC889\\DDC889Class"), ddc889_class),
        (model("DDC889\\DDC889Entity"), nothing),
        (model("DDC3579\\DDC3579User"), ddc3579_user),
        (model("DDC3579\\DDC3579Admin"), ddc3579_admin),
    ];
    classes
        .into_iter()
        .map(|(class, load)| (class, loader(load)))
        .collect()
}

fn list<const N: usize>(values: [Value; N]) -> Value {
    Value::Array(PhpArray::list(values))
}

fn s(text: &str) -> Value {
    Value::string(text)
}

/// An empty `loadMetadata()`.
fn nothing(_: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    Ok(())
}

/// `MappingDriverTestCase.php`'s `User::loadMetadata()`.
fn user(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.set_inheritance_type(cm::INHERITANCE_TYPE_NONE)?;
    metadata.set_primary_table(&php_array! {
        "name" => "cms_users",
        "options" => php_array! { "foo" => "bar", "baz" => php_array! { "key" => "val" } },
    });
    metadata.set_change_tracking_policy(cm::CHANGETRACKING_DEFERRED_IMPLICIT);
    metadata.add_lifecycle_callback("doStuffOnPrePersist", "prePersist")?;
    metadata.add_lifecycle_callback("doOtherStuffOnPrePersistToo", "prePersist")?;
    metadata.add_lifecycle_callback("doStuffOnPostPersist", "postPersist")?;
    metadata.map_field(php_array! {
        "id" => true,
        "fieldName" => "id",
        "type" => "integer",
        "columnName" => "id",
        "options" => php_array! { "foo" => "bar", "unsigned" => false },
    })?;
    metadata.map_field(php_array! {
        "fieldName" => "name",
        "type" => "string",
        "length" => 50_i64,
        "unique" => true,
        "nullable" => true,
        "columnName" => "name",
        "options" => php_array! {
            "foo" => "bar",
            "baz" => php_array! { "key" => "val" },
            "fixed" => false,
        },
    })?;
    metadata.map_field(php_array! {
        "fieldName" => "email",
        "type" => "string",
        "columnName" => "user_email",
        "columnDefinition" => "CHAR(32) NOT NULL",
    })?;
    let mut mapping = php_array! { "fieldName" => "version", "type" => "integer" };
    metadata.set_version_mapping(&mut mapping)?;
    metadata.map_field(mapping)?;
    metadata.map_field(php_array! {
        "fieldName" => "indexed",
        "type" => "string",
        "columnName" => "indexed",
        "index" => true,
    })?;
    metadata.set_id_generator_type(cm::GENERATOR_TYPE_AUTO);
    metadata.map_one_to_one(php_array! {
        "fieldName" => "address",
        "targetEntity" => ns("Address"),
        "cascade" => php_array! { 0_i64 => "remove" },
        "mappedBy" => Value::Null,
        "inversedBy" => "user",
        "joinColumns" => php_array! {
            0_i64 => php_array! {
                "name" => "address_id",
                "referencedColumnName" => "id",
                "onDelete" => "CASCADE",
            },
        },
        "orphanRemoval" => false,
    })?;
    metadata.map_one_to_many(php_array! {
        "fieldName" => "phonenumbers",
        "targetEntity" => ns("Phonenumber"),
        "cascade" => php_array! { 1_i64 => "persist" },
        "mappedBy" => "user",
        "orphanRemoval" => true,
        "orderBy" => php_array! { "number" => sort_direction(SortDirection::Ascending) },
    })?;
    metadata.map_many_to_many(php_array! {
        "fieldName" => "groups",
        "targetEntity" => ns("Group"),
        "cascade" => php_array! {
            0_i64 => "remove",
            1_i64 => "persist",
            2_i64 => "refresh",
            3_i64 => "detach",
        },
        "mappedBy" => Value::Null,
        "joinTable" => php_array! {
            "name" => "cms_users_groups",
            "joinColumns" => php_array! {
                0_i64 => php_array! {
                    "name" => "user_id",
                    "referencedColumnName" => "id",
                    "unique" => false,
                },
            },
            "inverseJoinColumns" => php_array! {
                0_i64 => php_array! {
                    "name" => "group_id",
                    "referencedColumnName" => "id",
                    "columnDefinition" => "INT NULL",
                },
            },
        },
        "orderBy" => PhpArray::new(),
    })?;
    metadata.table.insert(
        Key::from("uniqueConstraints"),
        Value::Array(php_array! {
            "search_idx" => php_array! {
                "columns" => list([s("name"), s("user_email")]),
                "options" => php_array! { "where" => "name IS NOT NULL" },
            },
            "phone_idx" => php_array! { "fields" => list([s("name"), s("phone")]) },
        }),
    );
    metadata.table.insert(
        Key::from("indexes"),
        Value::Array(php_array! {
            "name_idx" => php_array! { "columns" => list([s("name")]) },
            0_i64 => php_array! { "columns" => list([s("user_email")]) },
            "fields" => php_array! { "fields" => list([s("name"), s("email")]) },
        }),
    );
    metadata.set_sequence_generator_definition(php_array! {
        "sequenceName" => "tablename_seq",
        "allocationSize" => 100_i64,
        "initialValue" => 1_i64,
    })?;
    Ok(())
}

/// The fields of `UserIncorrectIndex::loadMetadata()` and
/// `UserIncorrectUniqueConstraint::loadMetadata()`.
fn map_incorrect_user(metadata: &mut ClassMetadataInfo, key: &str) -> Result<(), Thrown> {
    metadata.set_inheritance_type(cm::INHERITANCE_TYPE_NONE)?;
    metadata.set_primary_table(&PhpArray::new());
    metadata.map_field(php_array! {
        "id" => true,
        "fieldName" => "id",
        "type" => "integer",
        "columnName" => "id",
    })?;
    metadata.map_field(php_array! { "fieldName" => "name", "type" => "string" })?;
    metadata.map_field(php_array! {
        "fieldName" => "email",
        "type" => "string",
        "columnName" => "user_email",
    })?;
    metadata.table.insert(
        Key::from(key),
        Value::Array(php_array! {
            "name_idx" => php_array! {
                "columns" => list([s("name")]),
                "fields" => list([s("email")]),
            },
        }),
    );
    Ok(())
}

fn user_incorrect_index(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    map_incorrect_user(metadata, "indexes")
}

fn user_incorrect_unique_constraint(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    map_incorrect_user(metadata, "uniqueConstraints")
}

fn animal(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.set_id_generator_type(cm::GENERATOR_TYPE_CUSTOM);
    metadata.set_custom_generator_definition(php_array! { "class" => "stdClass" });
    Ok(())
}

fn ddc1170_entity(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.map_field(php_array! {
        "id" => true,
        "fieldName" => "id",
        "columnDefinition" => "INT unsigned NOT NULL",
    })?;
    metadata.map_field(php_array! {
        "fieldName" => "value",
        "columnDefinition" => "VARCHAR(255) NOT NULL",
    })?;
    metadata.set_id_generator_type(cm::GENERATOR_TYPE_NONE);
    Ok(())
}

fn ddc807_entity(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.map_field(php_array! { "id" => true, "fieldName" => "id" })?;
    metadata.set_discriminator_column(Some(php_array! {
        "name" => "dtype",
        "type" => "string",
        "columnDefinition" => "ENUM('ONE','TWO')",
    }))?;
    metadata.set_id_generator_type(cm::GENERATOR_TYPE_NONE);
    Ok(())
}

fn comment(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.set_inheritance_type(cm::INHERITANCE_TYPE_NONE)?;
    metadata.set_primary_table(&php_array! {
        "indexes" => php_array! {
            0_i64 => php_array! {
                "columns" => list([s("content")]),
                "flags" => list([s("fulltext")]),
                "options" => php_array! { "where" => "content IS NOT NULL" },
            },
        },
    });
    metadata.map_field(php_array! {
        "fieldName" => "content",
        "type" => "text",
        "scale" => 0_i64,
        "length" => Value::Null,
        "unique" => false,
        "nullable" => false,
        "precision" => 0_i64,
        "columnName" => "content",
    })?;
    Ok(())
}

/// `SingleTableEntityNoDiscriminatorColumnMapping::loadMetadata()` and
/// `SingleTableEntityIncompleteDiscriminatorColumnMapping::loadMetadata()`.
fn id_without_generator(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.map_field(php_array! { "id" => true, "fieldName" => "id" })?;
    metadata.set_id_generator_type(cm::GENERATOR_TYPE_NONE);
    Ok(())
}

fn reserved_word_in_table_column(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.map_field(php_array! {
        "id" => true,
        "fieldName" => "id",
        "type" => "integer",
    })?;
    metadata.map_field(php_array! {
        "fieldName" => "count",
        "type" => "integer",
        "columnName" => "`count`",
    })?;
    Ok(())
}

fn gh10288_enum_type_person(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.map_field(php_array! { "id" => true, "fieldName" => "id" })?;
    metadata.set_discriminator_column(Some(php_array! {
        "name" => "discr",
        "enumType" => model("GH10288\\GH10288People"),
    }))?;
    metadata.set_id_generator_type(cm::GENERATOR_TYPE_NONE);
    Ok(())
}

fn blog_post_comment(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.map_field(php_array! {
        "id" => true,
        "fieldName" => "id",
        "type" => "integer",
    })?;
    metadata.set_id_generator_type(cm::GENERATOR_TYPE_AUTO);
    metadata.map_many_to_one(php_array! {
        "fieldName" => "blogPost",
        "targetEntity" => ns("BlogPost"),
        "joinColumns" => php_array! { 0_i64 => php_array! { "nullable" => false } },
    })?;
    Ok(())
}

/// The id of `UserWithStringOrderBy` and `UserWithInvalidOrderBy`.
fn map_auto_id(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.map_field(php_array! {
        "id" => true,
        "fieldName" => "id",
        "type" => "integer",
    })?;
    metadata.set_id_generator_type(cm::GENERATOR_TYPE_AUTO);
    Ok(())
}

fn user_with_string_order_by(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    map_auto_id(metadata)?;
    metadata.map_one_to_many(php_array! {
        "fieldName" => "phonenumbersAsc",
        "mappedBy" => "user",
        "targetEntity" => ns("Phonenumber"),
        "orderBy" => php_array! { "number" => "ASC" },
    })?;
    metadata.map_one_to_many(php_array! {
        "fieldName" => "phonenumbersDesc",
        "mappedBy" => "user",
        "targetEntity" => ns("Phonenumber"),
        "orderBy" => php_array! { "number" => "DESC" },
    })?;
    metadata.map_many_to_many(php_array! {
        "fieldName" => "groupsMixed",
        "targetEntity" => ns("Group"),
        "orderBy" => php_array! { "name" => "ASC", "id" => "DESC" },
    })?;
    Ok(())
}

fn user_with_invalid_order_by(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    map_auto_id(metadata)?;
    metadata.map_one_to_many(php_array! {
        "fieldName" => "items",
        "mappedBy" => "user",
        "targetEntity" => ns("Phonenumber"),
        "orderBy" => php_array! { "name" => "INVALID" },
    })?;
    Ok(())
}

/// The id and name of `DDC964User::loadMetadata()` and
/// `DDC3579User::loadMetadata()`.
fn map_mapped_superclass_user(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.is_mapped_superclass = true;
    metadata.map_field(php_array! {
        "id" => true,
        "fieldName" => "id",
        "type" => "integer",
        "columnName" => "user_id",
        "length" => 150_i64,
    })?;
    metadata.map_field(php_array! {
        "fieldName" => "name",
        "type" => "string",
        "columnName" => "user_name",
        "nullable" => true,
        "unique" => false,
        "length" => 250_i64,
    })?;
    Ok(())
}

fn ddc964_user(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    map_mapped_superclass_user(metadata)?;
    metadata.map_many_to_one(php_array! {
        "fieldName" => "address",
        "targetEntity" => "DDC964Address",
        "cascade" => list([s("persist")]),
        "joinColumns" => list([Value::Array(php_array! {
            "name" => "address_id",
            "referencedColumnName" => "id",
        })]),
    })?;
    metadata.map_many_to_many(php_array! {
        "fieldName" => "groups",
        "targetEntity" => "DDC964Group",
        "inversedBy" => "users",
        "cascade" => list([s("persist"), s("detach")]),
        "joinTable" => php_array! {
            "name" => "ddc964_users_groups",
            "joinColumns" => list([Value::Array(php_array! {
                "name" => "user_id",
                "referencedColumnName" => "id",
            })]),
            "inverseJoinColumns" => list([Value::Array(php_array! {
                "name" => "group_id",
                "referencedColumnName" => "id",
            })]),
        },
    })?;
    metadata.set_id_generator_type(cm::GENERATOR_TYPE_AUTO);
    Ok(())
}

fn ddc964_admin(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.set_association_override(
        "address",
        &php_array! {
            "joinColumns" => list([Value::Array(php_array! {
                "name" => "adminaddress_id",
                "referencedColumnName" => "id",
            })]),
        },
    )?;
    metadata.set_association_override(
        "groups",
        &php_array! {
            "joinTable" => php_array! {
                "name" => "ddc964_users_admingroups",
                "joinColumns" => list([Value::Array(php_array! {
                    "name" => "adminuser_id",
                    "referencedColumnName" => "id",
                })]),
                "inverseJoinColumns" => list([Value::Array(php_array! {
                    "name" => "admingroup_id",
                    "referencedColumnName" => "id",
                })]),
            },
        },
    )?;
    Ok(())
}

fn ddc964_guest(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.set_attribute_override(
        "id",
        &php_array! {
            "columnName" => "guest_id",
            "type" => "integer",
            "length" => 140_i64,
        },
    )?;
    metadata.set_attribute_override(
        "name",
        &php_array! {
            "columnName" => "guest_name",
            "nullable" => false,
            "unique" => true,
            "length" => 240_i64,
        },
    )?;
    Ok(())
}

/// The eight entity listeners `CmsAddress` and `CompanyContract` declare:
/// a handler per event, named `{event}{suffix}`.
fn add_listeners(
    metadata: &mut ClassMetadataInfo,
    listener: &str,
    suffix: &str,
) -> Result<(), Thrown> {
    for event in [
        events::POST_PERSIST,
        events::PRE_PERSIST,
        events::POST_UPDATE,
        events::PRE_UPDATE,
        events::POST_REMOVE,
        events::PRE_REMOVE,
        events::PRE_FLUSH,
        events::POST_LOAD,
    ] {
        metadata.add_entity_listener(event, listener, &format!("{event}{suffix}"))?;
    }
    Ok(())
}

fn cms_address(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.set_primary_table(&php_array! { "name" => "company_person" });
    metadata.map_field(php_array! {
        "id" => true,
        "fieldName" => "id",
        "type" => "integer",
    })?;
    metadata.map_field(php_array! { "fieldName" => "zip", "length" => 50_i64 })?;
    metadata.map_field(php_array! { "fieldName" => "city", "length" => 50_i64 })?;
    metadata.map_one_to_one(php_array! {
        "fieldName" => "user",
        "targetEntity" => "CmsUser",
        "joinColumns" => list([Value::Array(php_array! { "referencedColumnName" => "id" })]),
    })?;
    add_listeners(metadata, "CmsAddressListener", "")
}

fn cms_user(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.set_primary_table(&php_array! { "name" => "cms_users" });
    Ok(())
}

fn ddc869_cheque_payment(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.map_field(php_array! { "fieldName" => "serialNumber", "type" => "string" })
}

fn ddc869_credit_card_payment(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.map_field(php_array! { "fieldName" => "creditCardNumber", "type" => "string" })
}

fn ddc869_payment(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.map_field(php_array! {
        "id" => true,
        "fieldName" => "id",
        "type" => "integer",
        "columnName" => "id",
    })?;
    metadata.map_field(php_array! { "fieldName" => "value", "type" => "float" })?;
    metadata.is_mapped_superclass = true;
    metadata.set_custom_repository_class(Some(&model("DDC869\\DDC869PaymentRepository")));
    metadata.set_id_generator_type(cm::GENERATOR_TYPE_AUTO);
    Ok(())
}

fn card(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.map_field(php_array! {
        "id" => true,
        "fieldName" => "id",
        "type" => "integer",
    })?;
    metadata.map_field(php_array! {
        "fieldName" => "suit",
        "type" => "string",
        "enumType" => model("Enums\\Suit"),
    })
}

fn user_typed(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.set_inheritance_type(cm::INHERITANCE_TYPE_NONE)?;
    metadata.set_primary_table(&php_array! { "name" => "cms_users_typed" });
    metadata.map_field(php_array! { "id" => true, "fieldName" => "id" })?;
    metadata.set_id_generator_type(cm::GENERATOR_TYPE_AUTO);
    metadata.map_field(php_array! { "fieldName" => "status", "length" => 50_i64 })?;
    metadata.map_field(php_array! {
        "fieldName" => "username",
        "length" => 255_i64,
        "unique" => true,
    })?;
    for field in [
        "dateInterval",
        "dateTime",
        "dateTimeImmutable",
        "array",
        "boolean",
        "float",
    ] {
        metadata.map_field(php_array! { "fieldName" => field })?;
    }
    metadata.map_one_to_one(php_array! {
        "fieldName" => "email",
        "cascade" => php_array! { 0_i64 => "persist" },
        "joinColumns" => php_array! {
            0_i64 => php_array! { "referencedColumnName" => "id" },
        },
        "orphanRemoval" => true,
    })?;
    metadata.map_many_to_one(php_array! { "fieldName" => "mainEmail" })?;
    metadata.map_embedded(php_array! { "fieldName" => "contact" })
}

fn user_typed_with_custom_typed_field(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.set_inheritance_type(cm::INHERITANCE_TYPE_NONE)?;
    metadata.set_primary_table(&php_array! {
        "name" => "cms_users_typed_with_custom_typed_field",
    });
    metadata.map_field(php_array! { "id" => true, "fieldName" => "id" })?;
    metadata.set_id_generator_type(cm::GENERATOR_TYPE_AUTO);
    metadata.map_field(php_array! { "fieldName" => "customId" })?;
    metadata.map_field(php_array! { "fieldName" => "customIntTypedField" })
}

fn insertable(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.set_primary_table(&php_array! { "name" => "insertable_column" });
    metadata.map_field(php_array! { "id" => true, "fieldName" => "id" })?;
    metadata.set_id_generator_type(cm::GENERATOR_TYPE_AUTO);
    metadata.map_field(php_array! {
        "fieldName" => "nonInsertableContent",
        "notInsertable" => true,
        "options" => php_array! { "default" => "1234" },
        "generated" => cm::GENERATED_INSERT,
    })?;
    metadata.map_field(php_array! { "fieldName" => "insertableContent" })
}

fn updatable(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.set_primary_table(&php_array! { "name" => "updatable_column" });
    metadata.map_field(php_array! { "id" => true, "fieldName" => "id" })?;
    metadata.set_id_generator_type(cm::GENERATOR_TYPE_AUTO);
    metadata.map_field(php_array! {
        "fieldName" => "nonUpdatableContent",
        "notUpdatable" => true,
        "generated" => cm::GENERATED_ALWAYS,
    })?;
    metadata.map_field(php_array! { "fieldName" => "updatableContent" })
}

fn ddc1476_entity_with_default_field_type(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.map_field(php_array! { "id" => true, "fieldName" => "id" })?;
    metadata.map_field(php_array! { "fieldName" => "name" })?;
    metadata.set_id_generator_type(cm::GENERATOR_TYPE_NONE);
    Ok(())
}

fn city(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.set_inheritance_type(cm::INHERITANCE_TYPE_NONE)?;
    metadata.set_primary_table(&php_array! { "name" => "cache_city" });
    metadata.set_id_generator_type(cm::GENERATOR_TYPE_IDENTITY);
    metadata.set_change_tracking_policy(cm::CHANGETRACKING_DEFERRED_IMPLICIT);
    metadata.enable_cache(php_array! { "usage" => cm::CACHE_USAGE_READ_ONLY });
    metadata.map_field(php_array! {
        "fieldName" => "id",
        "type" => "integer",
        "id" => true,
    })?;
    metadata.map_field(php_array! { "fieldName" => "name", "type" => "string" })?;
    metadata.map_one_to_one(php_array! {
        "fieldName" => "state",
        "targetEntity" => model("Cache\\State"),
        "inversedBy" => "cities",
        "joinColumns" => list([Value::Array(php_array! {
            "name" => "state_id",
            "referencedColumnName" => "id",
        })]),
    })?;
    metadata.enable_association_cache("state", php_array! { "usage" => cm::CACHE_USAGE_READ_ONLY });
    metadata.map_many_to_many(php_array! {
        "fieldName" => "travels",
        "targetEntity" => model("Cache\\Travel"),
        "mappedBy" => "visitedCities",
    })?;
    metadata.map_one_to_many(php_array! {
        "fieldName" => "attractions",
        "targetEntity" => model("Cache\\Attraction"),
        "mappedBy" => "city",
        "orderBy" => php_array! { "name" => sort_direction(SortDirection::Ascending) },
    })?;
    metadata.enable_association_cache(
        "attractions",
        php_array! { "usage" => cm::CACHE_USAGE_READ_ONLY },
    );
    Ok(())
}

/// An integer column of the `Company` contracts (`name` is the column
/// key the models pass, which `mapField()` ignores).
fn map_integer(metadata: &mut ClassMetadataInfo, field: &str) -> Result<(), Thrown> {
    metadata.map_field(php_array! {
        "type" => "integer",
        "name" => field,
        "fieldName" => field,
    })
}

fn company_fix_contract(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    map_integer(metadata, "fixPrice")
}

fn company_flex_contract(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    map_integer(metadata, "hoursWorked")?;
    map_integer(metadata, "pricePerHour")
}

fn company_contract(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.set_inheritance_type(cm::INHERITANCE_TYPE_JOINED)?;
    metadata.set_table_name("company_contracts");
    metadata.set_discriminator_column(Some(php_array! {
        "name" => "discr",
        "type" => "string",
    }))?;
    metadata.map_field(php_array! {
        "id" => true,
        "name" => "id",
        "fieldName" => "id",
    })?;
    metadata.map_field(php_array! {
        "type" => "boolean",
        "name" => "completed",
        "fieldName" => "completed",
    })?;
    metadata.set_discriminator_map(&php_array! {
        "fix" => "CompanyFixContract",
        "flexible" => "CompanyFlexContract",
        "flexultra" => "CompanyFlexUltraContract",
    })?;
    add_listeners(metadata, "CompanyContractListener", "Handler")
}

fn company_person(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.set_primary_table(&php_array! { "name" => "company_person" });
    Ok(())
}

fn company_flex_ultra_contract(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    map_integer(metadata, "maxPrice")?;
    add_listeners(metadata, "CompanyContractListener", "Handler")?;
    metadata.add_entity_listener(
        events::PRE_PERSIST,
        "CompanyFlexUltraContractListener",
        "prePersistHandler1",
    )?;
    metadata.add_entity_listener(
        events::PRE_PERSIST,
        "CompanyFlexUltraContractListener",
        "prePersistHandler2",
    )
}

fn ddc5934_base_contract(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.is_mapped_superclass = true;
    metadata.map_field(php_array! {
        "id" => true,
        "fieldName" => "id",
        "type" => "integer",
        "columnName" => "id",
    })?;
    metadata.map_many_to_many(php_array! {
        "fieldName" => "members",
        "targetEntity" => "DDC5934Member",
    })?;
    metadata.set_id_generator_type(cm::GENERATOR_TYPE_AUTO);
    Ok(())
}

fn ddc5934_contract(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.set_association_override("members", &php_array! { "fetch" => cm::FETCH_EXTRA_LAZY })
}

fn ddc889_super_class(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.map_field(php_array! { "fieldName" => "name" })?;
    metadata.is_mapped_superclass = true;
    metadata.set_id_generator_type(cm::GENERATOR_TYPE_NONE);
    Ok(())
}

fn ddc889_class(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.map_field(php_array! {
        "id" => true,
        "fieldName" => "id",
        "type" => "integer",
        "columnName" => "id",
    })?;
    metadata.set_id_generator_type(cm::GENERATOR_TYPE_AUTO);
    Ok(())
}

fn ddc3579_user(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    map_mapped_superclass_user(metadata)?;
    metadata.map_many_to_many(php_array! {
        "fieldName" => "groups",
        "targetEntity" => "DDC3579Group",
    })?;
    metadata.set_id_generator_type(cm::GENERATOR_TYPE_AUTO);
    Ok(())
}

fn ddc3579_admin(metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
    metadata.set_association_override("groups", &php_array! { "inversedBy" => "admins" })
}

// MappingDriverTestCase

#[test]
fn testEntityTableNameAndInheritance() {
    mapping_driver_test_case::testEntityTableNameAndInheritance();
}

#[test]
fn testEntityIndexes() {
    mapping_driver_test_case::testEntityIndexes();
}

#[test]
fn testEntityIndexFlagsAndPartialIndexes() {
    mapping_driver_test_case::testEntityIndexFlagsAndPartialIndexes();
}

#[test]
fn testEntityUniqueConstraints() {
    mapping_driver_test_case::testEntityUniqueConstraints();
}

#[test]
fn testEntityOptions() {
    mapping_driver_test_case::testEntityOptions();
}

#[test]
fn testEntitySequence() {
    mapping_driver_test_case::testEntitySequence();
}

#[test]
fn testEntityCustomGenerator() {
    mapping_driver_test_case::testEntityCustomGenerator();
}

#[test]
fn testFieldMappings() {
    mapping_driver_test_case::testFieldMappings();
}

#[test]
fn testVersionedField() {
    mapping_driver_test_case::testVersionedField();
}

#[test]
fn testFieldMappingsColumnNames() {
    mapping_driver_test_case::testFieldMappingsColumnNames();
}

#[test]
fn testStringFieldMappings() {
    mapping_driver_test_case::testStringFieldMappings();
}

#[test]
fn testFieldTypeFromReflection() {
    mapping_driver_test_case::testFieldTypeFromReflection();
}

#[test]
fn testCustomFieldTypeFromReflection() {
    mapping_driver_test_case::testCustomFieldTypeFromReflection();
}

#[test]
fn testFieldOptions() {
    mapping_driver_test_case::testFieldOptions();
}

#[test]
fn testIdFieldOptions() {
    mapping_driver_test_case::testIdFieldOptions();
}

#[test]
fn testIdentifier() {
    mapping_driver_test_case::testIdentifier();
}

#[test]
fn testBooleanValuesForOptionIsSetCorrectly() {
    mapping_driver_test_case::testBooleanValuesForOptionIsSetCorrectly();
}

#[test]
fn testAssociations() {
    mapping_driver_test_case::testAssociations();
}

#[test]
fn testOwningOneToOneAssociation() {
    mapping_driver_test_case::testOwningOneToOneAssociation();
}

#[test]
fn testInverseOneToManyAssociation() {
    mapping_driver_test_case::testInverseOneToManyAssociation();
}

#[test]
fn testOrderByWithStringValues() {
    mapping_driver_test_case::testOrderByWithStringValues();
}

#[test]
fn testOrderByWithInvalidDirection() {
    mapping_driver_test_case::testOrderByWithInvalidDirection();
}

#[test]
fn testManyToManyAssociationWithCascadeAll() {
    mapping_driver_test_case::testManyToManyAssociationWithCascadeAll();
}

#[test]
fn testLifecycleCallbacks() {
    mapping_driver_test_case::testLifecycleCallbacks();
}

#[test]
fn testLifecycleCallbacksSupportMultipleMethodNames() {
    mapping_driver_test_case::testLifecycleCallbacksSupportMultipleMethodNames();
}

#[test]
fn testJoinColumnUniqueAndNullable() {
    mapping_driver_test_case::testJoinColumnUniqueAndNullable();
}

#[test]
fn testColumnDefinition() {
    mapping_driver_test_case::testColumnDefinition();
}

#[test]
fn testJoinColumnOnDelete() {
    mapping_driver_test_case::testJoinColumnOnDelete();
}

#[test]
fn testDefaultFieldType() {
    mapping_driver_test_case::testDefaultFieldType();
}

#[test]
fn testIdentifierColumnDefinition() {
    mapping_driver_test_case::testIdentifierColumnDefinition();
}

#[test]
fn testNamingStrategy() {
    mapping_driver_test_case::testNamingStrategy();
}

#[test]
fn testDiscriminatorColumnDefinition() {
    mapping_driver_test_case::testDiscriminatorColumnDefinition();
}

#[test]
fn testDiscriminatorColumnEnumTypeDefinition() {
    mapping_driver_test_case::testDiscriminatorColumnEnumTypeDefinition();
}

#[test]
fn testIdentifierRequiredShouldMentionParentClasses() {
    mapping_driver_test_case::testIdentifierRequiredShouldMentionParentClasses();
}

#[test]
fn testInversedByOverrideMapping() {
    mapping_driver_test_case::testInversedByOverrideMapping();
}

#[test]
fn testFetchOverrideMapping() {
    mapping_driver_test_case::testFetchOverrideMapping();
}

#[test]
fn testAssociationOverridesMapping() {
    mapping_driver_test_case::testAssociationOverridesMapping();
}

#[test]
fn testAttributeOverridesMapping() {
    mapping_driver_test_case::testAttributeOverridesMapping();
}

#[test]
fn testEntityListeners() {
    mapping_driver_test_case::testEntityListeners();
}

#[test]
fn testEntityListenersOverride() {
    mapping_driver_test_case::testEntityListenersOverride();
}

#[test]
fn testEntityListenersNamingConvention() {
    mapping_driver_test_case::testEntityListenersNamingConvention();
}

#[test]
fn testSecondLevelCacheMapping() {
    mapping_driver_test_case::testSecondLevelCacheMapping();
}

#[test]
fn testReservedWordInTableColumn() {
    mapping_driver_test_case::testReservedWordInTableColumn();
}

#[test]
fn testInsertableColumn() {
    mapping_driver_test_case::testInsertableColumn();
}

#[test]
fn testUpdatableColumn() {
    mapping_driver_test_case::testUpdatableColumn();
}

#[test]
fn testEnumType() {
    mapping_driver_test_case::testEnumType();
}

#[test]
fn testCustomNamingStrategyIsRespected() {
    mapping_driver_test_case::testCustomNamingStrategyIsRespected();
}

// StaticPHPMappingDriverTest

/// All class with `static::loadMetadata` are entities for php driver
#[test]
fn testinvalidEntityOrMappedSuperClassShouldMentionParentClasses() {
    let class = create_class_metadata(&model("DDC889\\DDC889Class"));
    assert_eq!(model("DDC889\\DDC889Class"), class.get_name());
}
