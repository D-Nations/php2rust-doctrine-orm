//! Upstream: doctrine/orm tests/Tests/ORM/Tools/SchemaValidatorTest.php
//!
//! `getTestEntityManager()` is an entity manager on an in-memory `SQLite`
//! connection. The data provider rows run in one test. A model set's
//! directory (`addPaths([$path])`) is a class locator of the recorded
//! classes of its namespace, given to the driver before the entity
//! manager is made.

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

use std::sync::Arc;

use php_value::Thrown;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::driver::AttributeDriver;
use php2rust_doctrine_orm::mapping::driver::attribute_driver::Paths;
use php2rust_doctrine_orm::tools::schema_validator::SchemaValidator;
use php2rust_doctrine_persistence::mapping::driver::ClassLocator;
use php2rust_doctrine_persistence::php_reflection::{self, ClassKind};

/// A class of this test file's namespace.
macro_rules! tools {
    ($name:literal) => {
        concat!("Doctrine\\Tests\\ORM\\Tools\\", $name)
    };
}

const BIG_INTEGERS: &str = "Doctrine\\Tests\\Models\\BigIntegers\\BigIntegers";

struct Fixture {
    em: Arc<EntityManager>,
    validator: SchemaValidator,
}

fn set_up() -> Fixture {
    let em = orm::entity_manager(orm::connection(), None);
    let validator = SchemaValidator::new(&em, true);
    Fixture { em, validator }
}

impl Fixture {
    /// `$this->validator->validateClass($this->em->getClassMetadata($class))`.
    fn validate_class(&self, class_name: &str) -> Vec<String> {
        let class = self.em.get_class_metadata(class_name).unwrap();
        self.validator.validate_class(&class).unwrap()
    }
}

/// The classes of a model set's directory: the recorded classes and enums
/// of its namespace.
struct ModelSetLocator {
    namespace: String,
}

impl ClassLocator for ModelSetLocator {
    fn get_class_names(&self) -> Result<Vec<String>, Thrown> {
        Ok(php_reflection::declared()
            .iter()
            .filter(|class| matches!(class.kind(), ClassKind::Class | ClassKind::Enum))
            .map(|class| class.name().to_owned())
            .filter(|name| name.starts_with(&self.namespace))
            .collect())
    }
}

/// `modelSetProvider()`: the namespaces of the model directories.
fn model_set_provider() -> [(&'static str, &'static str); 6] {
    [
        ("cms", "Doctrine\\Tests\\Models\\CMS\\"),
        ("company", "Doctrine\\Tests\\Models\\Company\\"),
        ("ecommerce", "Doctrine\\Tests\\Models\\ECommerce\\"),
        ("forum", "Doctrine\\Tests\\Models\\Forum\\"),
        ("navigation", "Doctrine\\Tests\\Models\\Navigation\\"),
        ("routing", "Doctrine\\Tests\\Models\\Routing\\"),
    ]
}

#[test]
fn testCmsModelSet() {
    for (row, path) in model_set_provider() {
        models::register();
        let em = orm::entity_manager_with(orm::connection(), |config| {
            let locator: Arc<dyn ClassLocator> = Arc::new(ModelSetLocator {
                namespace: path.to_owned(),
            });
            config.set_metadata_driver_impl(Arc::new(
                AttributeDriver::new(Paths::Locator(locator), true).unwrap(),
            ));
        });
        let validator = SchemaValidator::new(&em, true);

        let errors = validator.validate_mapping().unwrap();
        assert!(errors.is_empty(), "{row}: {errors:?}");
    }
}

#[test]
fn testInvalidManyToManyJoinColumnSchema() {
    let f = set_up();
    f.em.get_class_metadata(tools!("InvalidEntity2")).unwrap();

    let ce = f.validate_class(tools!("InvalidEntity1"));

    assert_eq!(
        vec![
            "The inverse join columns of the many-to-many table 'Entity1Entity2' have to contain to ALL identifier columns of the target entity 'Doctrine\\Tests\\ORM\\Tools\\InvalidEntity2', however 'key4' are missing.",
            "The join columns of the many-to-many table 'Entity1Entity2' have to contain to ALL identifier columns of the source entity 'Doctrine\\Tests\\ORM\\Tools\\InvalidEntity1', however 'key2' are missing.",
        ],
        ce,
    );
}

#[test]
fn testInvalidToOneJoinColumnSchema() {
    let f = set_up();
    f.em.get_class_metadata(tools!("InvalidEntity1")).unwrap();

    let ce = f.validate_class(tools!("InvalidEntity2"));

    assert_eq!(
        vec![
            "The referenced column name 'id' has to be a primary key column on the target entity class 'Doctrine\\Tests\\ORM\\Tools\\InvalidEntity1'.",
            "The join columns of the association 'assoc' have to match to ALL identifier columns of the target entity 'Doctrine\\Tests\\ORM\\Tools\\InvalidEntity1', however 'key1, key2' are missing.",
        ],
        ce,
    );
}

#[test]
fn testValidOneToOneAsIdentifierSchema() {
    let f = set_up();
    f.em.get_class_metadata(tools!("DDC1587ValidEntity1")).unwrap();

    let ce = f.validate_class(tools!("DDC1587ValidEntity2"));

    assert!(ce.is_empty(), "{ce:?}");
}

#[test]
fn testInvalidManyToOneAsSoleIdentifier() {
    let f = set_up();
    let ce = f.validate_class(tools!("DDC1649Two"));

    assert_eq!(
        vec![
            "The association 'Doctrine\\Tests\\ORM\\Tools\\DDC1649Two#one' is a many-to-one association and is the sole identifier of the entity. This effectively makes the association one-to-one; use a one-to-one association instead or add another identifier field."
        ],
        ce,
    );
}

#[test]
fn testValidManyToOneAsPartOfCompositeIdentifier() {
    let f = set_up();
    let ce = f.validate_class(tools!("ValidCompositeIdentifier"));

    assert!(ce.is_empty(), "{ce:?}");
}

#[test]
fn testInvalidTripleAssociationAsKeyMapping() {
    let f = set_up();
    let ce = f.validate_class(tools!("DDC1649Three"));

    assert_eq!(
        vec![
            "The association 'Doctrine\\Tests\\ORM\\Tools\\DDC1649Three#two' is a many-to-one association and is the sole identifier of the entity. This effectively makes the association one-to-one; use a one-to-one association instead or add another identifier field.",
            "Cannot map association 'Doctrine\\Tests\\ORM\\Tools\\DDC1649Three#two as identifier, because the target entity 'Doctrine\\Tests\\ORM\\Tools\\DDC1649Two' also maps an association as identifier.",
            "The referenced column name 'id' has to be a primary key column on the target entity class 'Doctrine\\Tests\\ORM\\Tools\\DDC1649Two'.",
        ],
        ce,
    );
}

#[test]
fn testInvalidBiDirectionalRelationMappingMissingInversedByAttribute() {
    let f = set_up();
    let ce = f.validate_class(tools!("DDC3274One"));

    assert_eq!(
        vec![concat!(
            "The field Doctrine\\Tests\\ORM\\Tools\\DDC3274One#two is on the inverse side of a bi-directional ",
            "relationship, but the specified mappedBy association on the target-entity ",
            "Doctrine\\Tests\\ORM\\Tools\\DDC3274Two#one does not contain the required 'inversedBy: \"two\"' attribute.",
        )],
        ce,
    );
}

#[test]
fn testInvalidBiDirectionalRelationMappingMissingMappedByAttribute() {
    let f = set_up();
    let ce = f.validate_class(tools!("Issue9536Owner"));

    assert_eq!(
        vec![concat!(
            "The field Doctrine\\Tests\\ORM\\Tools\\Issue9536Owner#one is on the owning side of a bi-directional ",
            "relationship, but the specified inversedBy association on the target-entity ",
            "Doctrine\\Tests\\ORM\\Tools\\Issue9536Target#two does not contain the required 'mappedBy: \"one\"' ",
            "attribute.",
        )],
        ce,
    );
}

#[test]
fn testInvalidOrderByInvalidField() {
    let f = set_up();
    let ce = f.validate_class(tools!("DDC3322One"));

    assert_eq!(
        vec![concat!(
            "The association Doctrine\\Tests\\ORM\\Tools\\DDC3322One#invalidAssoc is ordered by a foreign field ",
            "invalidField that is not a field on the target entity Doctrine\\Tests\\ORM\\Tools\\DDC3322ValidEntity1.",
        )],
        ce,
    );
}

#[test]
fn testInvalidOrderByCollectionValuedAssociation() {
    let f = set_up();
    let ce = f.validate_class(tools!("DDC3322Two"));

    assert_eq!(
        vec![concat!(
            "The association Doctrine\\Tests\\ORM\\Tools\\DDC3322Two#invalidAssoc is ordered by a field oneToMany ",
            "on Doctrine\\Tests\\ORM\\Tools\\DDC3322ValidEntity1 that is a collection-valued association.",
        )],
        ce,
    );
}

#[test]
fn testInvalidOrderByAssociationInverseSide() {
    let f = set_up();
    let ce = f.validate_class(tools!("DDC3322Three"));

    assert_eq!(
        vec![concat!(
            "The association Doctrine\\Tests\\ORM\\Tools\\DDC3322Three#invalidAssoc is ordered by a field oneToOneInverse ",
            "on Doctrine\\Tests\\ORM\\Tools\\DDC3322ValidEntity1 that is the inverse side of an association.",
        )],
        ce,
    );
}

#[test]
fn testMappedSuperclassNotPresentInDiscriminator() {
    let f = set_up();
    let ce = f.validate_class(tools!("MappedSuperclassEntity"));

    assert!(ce.is_empty(), "{ce:?}");
}

#[test]
fn testAbstractChildClassNotPresentInDiscriminator() {
    let f = set_up();
    let ce = f.validate_class(tools!("Issue9095AbstractChild"));

    assert!(ce.is_empty(), "{ce:?}");
}

#[test]
fn testInvalidAssociationTowardsMappedSuperclass() {
    let f = set_up();
    let ce = f.validate_class(tools!("InvalidMappedSuperClass"));

    assert_eq!(
        vec![
            "The target entity 'Doctrine\\Tests\\ORM\\Tools\\InvalidMappedSuperClass' specified on Doctrine\\Tests\\ORM\\Tools\\InvalidMappedSuperClass#selfWhatever is a mapped superclass. This is not possible since there is no table that a foreign key could refer to."
        ],
        ce,
    );
}

#[test]
fn testBigIntProperty() {
    let f = set_up();

    assert_eq!(
        vec![
            "The field 'Doctrine\\Tests\\Models\\BigIntegers\\BigIntegers#three' has the property type 'float' that differs from the metadata field type 'int|string' returned by the 'bigint' DBAL type."
        ],
        f.validate_class(BIG_INTEGERS),
    );
}

#[test]
fn testOwningSideInversedByTargetsDifferentEntity() {
    let f = set_up();
    let ce = f.validate_class(tools!("MultipleInversedByOwner2"));

    assert_eq!(
        vec![concat!(
            "The association Doctrine\\Tests\\ORM\\Tools\\MultipleInversedByOwner2#target refers to the inverse side ",
            "Doctrine\\Tests\\ORM\\Tools\\MultipleInversedByInverse#owners which targets a different entity ",
            "(Doctrine\\Tests\\ORM\\Tools\\MultipleInversedByOwner1).",
        )],
        ce,
    );
}
