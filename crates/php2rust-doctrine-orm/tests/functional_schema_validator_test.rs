//! Functional/SchemaValidatorTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa:
//! the validity of all model sets. The data provider's cases run in one
//! test, one model set after the other. `setUp()` registers the
//! `CustomIdObjectType`, `UpperCaseStringType` and `NegativeToPositiveType`
//! types: here Rust types with their names and declarations (the
//! validator reads no more of them).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/SchemaValidatorTest.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;
#[path = "common/rot13.rs"]
mod rot13;

use std::sync::Arc;

use php_value::{PhpArray, PhpString};
use php2rust_doctrine_dbal::platforms::AbstractPlatform;
use php2rust_doctrine_dbal::types::{Type, TypeArgument, add_type, has_type, override_type};
use php2rust_doctrine_orm::tools::schema_validator::SchemaValidator;

/// `Doctrine\Tests\DbalTypes\CustomIdObjectType extends Type`.
#[derive(Debug)]
struct CustomIdObjectType;

impl Type for CustomIdObjectType {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\DbalTypes\\CustomIdObjectType"
    }

    fn get_sql_declaration(
        &self,
        column: &PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<PhpString, php2rust_doctrine_dbal::Exception> {
        platform.get_string_type_declaration_sql(column)
    }
}

/// `Doctrine\Tests\DbalTypes\UpperCaseStringType extends StringType`.
#[derive(Debug)]
struct UpperCaseStringType;

impl Type for UpperCaseStringType {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\DbalTypes\\UpperCaseStringType"
    }

    fn get_sql_declaration(
        &self,
        column: &PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<PhpString, php2rust_doctrine_dbal::Exception> {
        platform.get_string_type_declaration_sql(column)
    }
}

/// `Doctrine\Tests\DbalTypes\NegativeToPositiveType extends Type`.
#[derive(Debug)]
struct NegativeToPositiveType;

impl Type for NegativeToPositiveType {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\DbalTypes\\NegativeToPositiveType"
    }

    fn get_sql_declaration(
        &self,
        column: &PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<PhpString, php2rust_doctrine_dbal::Exception> {
        platform.get_integer_type_declaration_sql(column)
    }
}

/// `registerType($className)`: `addType()`, or `overrideType()` when the
/// type is there.
fn register_type(name: &str, type_: impl Fn() -> TypeArgument) {
    if has_type(name) {
        override_type(name, type_()).unwrap();
    } else if add_type(name, type_()).is_err() {
        // Another test thread may have added it meanwhile.
        override_type(name, type_()).unwrap();
    }
}

/// `setUp()` (and `OrmFunctionalTestCase::setUp()`'s `rot13` type).
fn set_up() {
    rot13::register_rot13_type();
    register_type("CustomIdObject", || {
        TypeArgument::Instance(Arc::new(CustomIdObjectType))
    });
    register_type("upper_case_string", || {
        TypeArgument::Instance(Arc::new(UpperCaseStringType))
    });
    register_type("negative_to_positive", || {
        TypeArgument::Instance(Arc::new(NegativeToPositiveType))
    });
}

/// `OrmFunctionalTestCase::$modelSets`.
const MODEL_SETS: &[(&str, &[&str])] = &[
    (
        "cms",
        &[
            "Doctrine\\Tests\\Models\\CMS\\CmsUser",
            "Doctrine\\Tests\\Models\\CMS\\CmsPhonenumber",
            "Doctrine\\Tests\\Models\\CMS\\CmsAddress",
            "Doctrine\\Tests\\Models\\CMS\\CmsEmail",
            "Doctrine\\Tests\\Models\\CMS\\CmsGroup",
            "Doctrine\\Tests\\Models\\CMS\\CmsTag",
            "Doctrine\\Tests\\Models\\CMS\\CmsArticle",
            "Doctrine\\Tests\\Models\\CMS\\CmsComment",
        ],
    ),
    (
        "company",
        &[
            "Doctrine\\Tests\\Models\\Company\\CompanyPerson",
            "Doctrine\\Tests\\Models\\Company\\CompanyEmployee",
            "Doctrine\\Tests\\Models\\Company\\CompanyManager",
            "Doctrine\\Tests\\Models\\Company\\CompanyOrganization",
            "Doctrine\\Tests\\Models\\Company\\CompanyEvent",
            "Doctrine\\Tests\\Models\\Company\\CompanyAuction",
            "Doctrine\\Tests\\Models\\Company\\CompanyRaffle",
            "Doctrine\\Tests\\Models\\Company\\CompanyCar",
            "Doctrine\\Tests\\Models\\Company\\CompanyContract",
        ],
    ),
    (
        "ecommerce",
        &[
            "Doctrine\\Tests\\Models\\ECommerce\\ECommerceCart",
            "Doctrine\\Tests\\Models\\ECommerce\\ECommerceCustomer",
            "Doctrine\\Tests\\Models\\ECommerce\\ECommerceProduct",
            "Doctrine\\Tests\\Models\\ECommerce\\ECommerceShipping",
            "Doctrine\\Tests\\Models\\ECommerce\\ECommerceFeature",
            "Doctrine\\Tests\\Models\\ECommerce\\ECommerceCategory",
        ],
    ),
    (
        "generic",
        &[
            "Doctrine\\Tests\\Models\\Generic\\BooleanModel",
            "Doctrine\\Tests\\Models\\Generic\\DateTimeModel",
            "Doctrine\\Tests\\Models\\Generic\\DecimalModel",
        ],
    ),
    (
        "routing",
        &[
            "Doctrine\\Tests\\Models\\Routing\\RoutingLeg",
            "Doctrine\\Tests\\Models\\Routing\\RoutingLocation",
            "Doctrine\\Tests\\Models\\Routing\\RoutingRoute",
            "Doctrine\\Tests\\Models\\Routing\\RoutingRouteBooking",
        ],
    ),
    (
        "navigation",
        &[
            "Doctrine\\Tests\\Models\\Navigation\\NavUser",
            "Doctrine\\Tests\\Models\\Navigation\\NavCountry",
            "Doctrine\\Tests\\Models\\Navigation\\NavPhotos",
            "Doctrine\\Tests\\Models\\Navigation\\NavTour",
            "Doctrine\\Tests\\Models\\Navigation\\NavPointOfInterest",
        ],
    ),
    (
        "directorytree",
        &[
            "Doctrine\\Tests\\Models\\DirectoryTree\\AbstractContentItem",
            "Doctrine\\Tests\\Models\\DirectoryTree\\File",
            "Doctrine\\Tests\\Models\\DirectoryTree\\Directory",
        ],
    ),
    (
        "ddc117",
        &[
            "Doctrine\\Tests\\Models\\DDC117\\DDC117Article",
            "Doctrine\\Tests\\Models\\DDC117\\DDC117Reference",
            "Doctrine\\Tests\\Models\\DDC117\\DDC117Translation",
            "Doctrine\\Tests\\Models\\DDC117\\DDC117ArticleDetails",
            "Doctrine\\Tests\\Models\\DDC117\\DDC117ApproveChanges",
            "Doctrine\\Tests\\Models\\DDC117\\DDC117Editor",
            "Doctrine\\Tests\\Models\\DDC117\\DDC117Link",
        ],
    ),
    (
        "ddc3699",
        &[
            "Doctrine\\Tests\\Models\\DDC3699\\DDC3699Parent",
            "Doctrine\\Tests\\Models\\DDC3699\\DDC3699RelationOne",
            "Doctrine\\Tests\\Models\\DDC3699\\DDC3699RelationMany",
            "Doctrine\\Tests\\Models\\DDC3699\\DDC3699Child",
        ],
    ),
    (
        "stockexchange",
        &[
            "Doctrine\\Tests\\Models\\StockExchange\\Bond",
            "Doctrine\\Tests\\Models\\StockExchange\\Stock",
            "Doctrine\\Tests\\Models\\StockExchange\\Market",
        ],
    ),
    (
        "legacy",
        &[
            "Doctrine\\Tests\\Models\\Legacy\\LegacyUser",
            "Doctrine\\Tests\\Models\\Legacy\\LegacyUserReference",
            "Doctrine\\Tests\\Models\\Legacy\\LegacyArticle",
            "Doctrine\\Tests\\Models\\Legacy\\LegacyCar",
        ],
    ),
    (
        "customtype",
        &[
            "Doctrine\\Tests\\Models\\CustomType\\CustomTypeChild",
            "Doctrine\\Tests\\Models\\CustomType\\CustomTypeParent",
            "Doctrine\\Tests\\Models\\CustomType\\CustomTypeUpperCase",
        ],
    ),
    (
        "compositekeyinheritance",
        &[
            "Doctrine\\Tests\\Models\\CompositeKeyInheritance\\JoinedRootClass",
            "Doctrine\\Tests\\Models\\CompositeKeyInheritance\\JoinedChildClass",
            "Doctrine\\Tests\\Models\\CompositeKeyInheritance\\SingleRootClass",
            "Doctrine\\Tests\\Models\\CompositeKeyInheritance\\SingleChildClass",
        ],
    ),
    (
        "compositekeyrelations",
        &[
            "Doctrine\\Tests\\Models\\CompositeKeyRelations\\InvoiceClass",
            "Doctrine\\Tests\\Models\\CompositeKeyRelations\\CustomerClass",
        ],
    ),
    (
        "taxi",
        &[
            "Doctrine\\Tests\\Models\\Taxi\\PaidRide",
            "Doctrine\\Tests\\Models\\Taxi\\Ride",
            "Doctrine\\Tests\\Models\\Taxi\\Car",
            "Doctrine\\Tests\\Models\\Taxi\\Driver",
        ],
    ),
    (
        "cache",
        &[
            "Doctrine\\Tests\\Models\\Cache\\Country",
            "Doctrine\\Tests\\Models\\Cache\\State",
            "Doctrine\\Tests\\Models\\Cache\\City",
            "Doctrine\\Tests\\Models\\Cache\\Traveler",
            "Doctrine\\Tests\\Models\\Cache\\TravelerProfileInfo",
            "Doctrine\\Tests\\Models\\Cache\\TravelerProfile",
            "Doctrine\\Tests\\Models\\Cache\\Travel",
            "Doctrine\\Tests\\Models\\Cache\\Attraction",
            "Doctrine\\Tests\\Models\\Cache\\Restaurant",
            "Doctrine\\Tests\\Models\\Cache\\Beach",
            "Doctrine\\Tests\\Models\\Cache\\Bar",
            "Doctrine\\Tests\\Models\\Cache\\Flight",
            "Doctrine\\Tests\\Models\\Cache\\Token",
            "Doctrine\\Tests\\Models\\Cache\\Login",
            "Doctrine\\Tests\\Models\\Cache\\Client",
            "Doctrine\\Tests\\Models\\Cache\\Person",
            "Doctrine\\Tests\\Models\\Cache\\Address",
            "Doctrine\\Tests\\Models\\Cache\\Action",
            "Doctrine\\Tests\\Models\\Cache\\ComplexAction",
            "Doctrine\\Tests\\Models\\Cache\\AttractionInfo",
            "Doctrine\\Tests\\Models\\Cache\\AttractionContactInfo",
            "Doctrine\\Tests\\Models\\Cache\\AttractionLocationInfo",
        ],
    ),
    (
        "tweet",
        &[
            "Doctrine\\Tests\\Models\\Tweet\\User",
            "Doctrine\\Tests\\Models\\Tweet\\Tweet",
            "Doctrine\\Tests\\Models\\Tweet\\UserList",
        ],
    ),
    (
        "ddc2504",
        &[
            "Doctrine\\Tests\\Models\\DDC2504\\DDC2504RootClass",
            "Doctrine\\Tests\\Models\\DDC2504\\DDC2504ChildClass",
            "Doctrine\\Tests\\Models\\DDC2504\\DDC2504OtherClass",
        ],
    ),
    (
        "ddc3346",
        &[
            "Doctrine\\Tests\\Models\\DDC3346\\DDC3346Author",
            "Doctrine\\Tests\\Models\\DDC3346\\DDC3346Article",
        ],
    ),
    (
        "quote",
        &[
            "Doctrine\\Tests\\Models\\Quote\\Address",
            "Doctrine\\Tests\\Models\\Quote\\City",
            "Doctrine\\Tests\\Models\\Quote\\FullAddress",
            "Doctrine\\Tests\\Models\\Quote\\Group",
            "Doctrine\\Tests\\Models\\Quote\\NumericEntity",
            "Doctrine\\Tests\\Models\\Quote\\Phone",
            "Doctrine\\Tests\\Models\\Quote\\User",
        ],
    ),
    (
        "vct_onetoone",
        &[
            "Doctrine\\Tests\\Models\\ValueConversionType\\InversedOneToOneEntity",
            "Doctrine\\Tests\\Models\\ValueConversionType\\OwningOneToOneEntity",
        ],
    ),
    (
        "vct_onetoone_compositeid",
        &[
            "Doctrine\\Tests\\Models\\ValueConversionType\\InversedOneToOneCompositeIdEntity",
            "Doctrine\\Tests\\Models\\ValueConversionType\\OwningOneToOneCompositeIdEntity",
        ],
    ),
    (
        "vct_onetoone_compositeid_foreignkey",
        &[
            "Doctrine\\Tests\\Models\\ValueConversionType\\AuxiliaryEntity",
            "Doctrine\\Tests\\Models\\ValueConversionType\\InversedOneToOneCompositeIdForeignKeyEntity",
            "Doctrine\\Tests\\Models\\ValueConversionType\\OwningOneToOneCompositeIdForeignKeyEntity",
        ],
    ),
    (
        "vct_onetomany",
        &[
            "Doctrine\\Tests\\Models\\ValueConversionType\\InversedOneToManyEntity",
            "Doctrine\\Tests\\Models\\ValueConversionType\\OwningManyToOneEntity",
        ],
    ),
    (
        "vct_onetomany_compositeid",
        &[
            "Doctrine\\Tests\\Models\\ValueConversionType\\InversedOneToManyCompositeIdEntity",
            "Doctrine\\Tests\\Models\\ValueConversionType\\OwningManyToOneCompositeIdEntity",
        ],
    ),
    (
        "vct_onetomany_compositeid_foreignkey",
        &[
            "Doctrine\\Tests\\Models\\ValueConversionType\\AuxiliaryEntity",
            "Doctrine\\Tests\\Models\\ValueConversionType\\InversedOneToManyCompositeIdForeignKeyEntity",
            "Doctrine\\Tests\\Models\\ValueConversionType\\OwningManyToOneCompositeIdForeignKeyEntity",
        ],
    ),
    (
        "vct_onetomany_extralazy",
        &[
            "Doctrine\\Tests\\Models\\ValueConversionType\\InversedOneToManyExtraLazyEntity",
            "Doctrine\\Tests\\Models\\ValueConversionType\\OwningManyToOneExtraLazyEntity",
        ],
    ),
    (
        "vct_manytomany",
        &[
            "Doctrine\\Tests\\Models\\ValueConversionType\\InversedManyToManyEntity",
            "Doctrine\\Tests\\Models\\ValueConversionType\\OwningManyToManyEntity",
        ],
    ),
    (
        "vct_manytomany_compositeid",
        &[
            "Doctrine\\Tests\\Models\\ValueConversionType\\InversedManyToManyCompositeIdEntity",
            "Doctrine\\Tests\\Models\\ValueConversionType\\OwningManyToManyCompositeIdEntity",
        ],
    ),
    (
        "vct_manytomany_compositeid_foreignkey",
        &[
            "Doctrine\\Tests\\Models\\ValueConversionType\\AuxiliaryEntity",
            "Doctrine\\Tests\\Models\\ValueConversionType\\InversedManyToManyCompositeIdForeignKeyEntity",
            "Doctrine\\Tests\\Models\\ValueConversionType\\OwningManyToManyCompositeIdForeignKeyEntity",
        ],
    ),
    (
        "vct_manytomany_extralazy",
        &[
            "Doctrine\\Tests\\Models\\ValueConversionType\\InversedManyToManyExtraLazyEntity",
            "Doctrine\\Tests\\Models\\ValueConversionType\\OwningManyToManyExtraLazyEntity",
        ],
    ),
    (
        "geonames",
        &[
            "Doctrine\\Tests\\Models\\GeoNames\\Country",
            "Doctrine\\Tests\\Models\\GeoNames\\Admin1",
            "Doctrine\\Tests\\Models\\GeoNames\\Admin1AlternateName",
            "Doctrine\\Tests\\Models\\GeoNames\\City",
        ],
    ),
    (
        "custom_id_object_type",
        &[
            "Doctrine\\Tests\\Models\\CustomType\\CustomIdObjectTypeParent",
            "Doctrine\\Tests\\Models\\CustomType\\CustomIdObjectTypeChild",
        ],
    ),
    (
        "pagination",
        &[
            "Doctrine\\Tests\\Models\\Pagination\\Company",
            "Doctrine\\Tests\\Models\\Pagination\\Logo",
            "Doctrine\\Tests\\Models\\Pagination\\Department",
            "Doctrine\\Tests\\Models\\Pagination\\User",
            "Doctrine\\Tests\\Models\\Pagination\\User1",
        ],
    ),
    (
        "versioned_many_to_one",
        &[
            "Doctrine\\Tests\\Models\\VersionedManyToOne\\Category",
            "Doctrine\\Tests\\Models\\VersionedManyToOne\\Article",
        ],
    ),
    (
        "issue5989",
        &[
            "Doctrine\\Tests\\Models\\Issue5989\\Issue5989Person",
            "Doctrine\\Tests\\Models\\Issue5989\\Issue5989Employee",
            "Doctrine\\Tests\\Models\\Issue5989\\Issue5989Manager",
        ],
    ),
    (
        "issue9300",
        &[
            "Doctrine\\Tests\\Models\\Issue9300\\Issue9300Child",
            "Doctrine\\Tests\\Models\\Issue9300\\Issue9300Parent",
        ],
    ),
];

/// `dataValidateModelSets()` and `testValidateModelSets($modelSet)`.
#[tokio::test]
async fn testValidateModelSets() {
    set_up();
    let em = orm::entity_manager(orm::connection(), None);
    for (model_set, class_names) in MODEL_SETS {
        let validator = SchemaValidator::new(&em, true);
        let classes: Vec<_> = class_names
            .iter()
            .map(|class_name| em.get_class_metadata(class_name).unwrap())
            .collect();

        for class in &classes {
            let ce = validator.validate_class(class).unwrap();
            assert!(
                ce.is_empty(),
                "Invalid Modelset: {model_set} class {}: {}",
                class.read().name,
                ce.join("\n")
            );
        }
    }
}
