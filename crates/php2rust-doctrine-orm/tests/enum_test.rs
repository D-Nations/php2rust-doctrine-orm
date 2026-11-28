//! Functional/EnumTest.php on `SQLite` (`tests/common/orm.rs`).
//!
//! The attribute driver reads the registered models (upstream points it
//! at `Models/Enums`). A data provider's rows run in one test, each on a
//! new entity manager. `NEW` objects register their constructors
//! (`register_new_object_class()`). `uniqid('', true)` is a counter-based
//! unique id. `ReflectionProperty::getAttributes()` is the attribute
//! registry's `property_attributes()`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/EnumTest.php
//! Upstream: doctrine/orm tests/Tests/Models/Enums/Library.php tests/Tests/Models/Enums/BookWithGenre.php
//! Upstream: doctrine/orm tests/Tests/Models/Enums/BookCategory.php
//! Upstream: doctrine/orm tests/Tests/Models/DataTransferObjects/DtoWithEnum.php tests/Tests/Models/DataTransferObjects/DtoWithArrayOfEnums.php

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
use std::sync::atomic::{AtomicU64, Ordering};

use php_value::{Object, PhpArray, Value, php_array};
use php2rust_doctrine_collections::Criteria;
use php2rust_doctrine_collections::expr::Comparison;
use php2rust_doctrine_dbal::ParameterTypes;
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::internal::hydration::register_new_object_class;
use php2rust_doctrine_orm::mapping::attributes::property_attributes;
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;
use php2rust_doctrine_orm::query::context::{HYDRATE_ARRAY, HYDRATE_OBJECT};
use php2rust_doctrine_orm::query::expr::Func;
use php2rust_doctrine_persistence::php_reflection;

use orm::{get, message, new_object, obj, s, set};

const CARD: &str = "Doctrine\\Tests\\Models\\Enums\\Card";
const TYPED_CARD: &str = "Doctrine\\Tests\\Models\\Enums\\TypedCard";
const CARD_NATIVE_ENUM: &str = "Doctrine\\Tests\\Models\\Enums\\CardNativeEnum";
const TYPED_CARD_NATIVE_ENUM: &str = "Doctrine\\Tests\\Models\\Enums\\TypedCardNativeEnum";
const CARD_WITH_DEFAULT: &str = "Doctrine\\Tests\\Models\\Enums\\CardWithDefault";
const CARD_WITH_NULLABLE: &str = "Doctrine\\Tests\\Models\\Enums\\CardWithNullable";
const SCALE: &str = "Doctrine\\Tests\\Models\\Enums\\Scale";
const PRODUCT: &str = "Doctrine\\Tests\\Models\\Enums\\Product";
const QUANTITY: &str = "Doctrine\\Tests\\Models\\Enums\\Quantity";
const LIBRARY: &str = "Doctrine\\Tests\\Models\\Enums\\Library";
const BOOK_WITH_GENRE: &str = "Doctrine\\Tests\\Models\\Enums\\BookWithGenre";
const BOOK_CATEGORY: &str = "Doctrine\\Tests\\Models\\Enums\\BookCategory";
const SUIT: &str = "Doctrine\\Tests\\Models\\Enums\\Suit";
const UNIT: &str = "Doctrine\\Tests\\Models\\Enums\\Unit";
const BOOK_GENRE: &str = "Doctrine\\Tests\\Models\\Enums\\BookGenre";
const DTO_WITH_ENUM: &str = "Doctrine\\Tests\\Models\\DataTransferObjects\\DtoWithEnum";
const DTO_WITH_ARRAY_OF_ENUMS: &str =
    "Doctrine\\Tests\\Models\\DataTransferObjects\\DtoWithArrayOfEnums";
const MAPPING_EXCEPTION: &str = "Doctrine\\ORM\\Mapping\\MappingException";
const COLUMN: &str = "Doctrine\\ORM\\Mapping\\Column";

/// `$enum::Case`.
fn case(enum_type: &str, name: &str) -> Value {
    models::register();
    let class = php_reflection::class_info(enum_type).expect("the enum is declared");
    let case = class
        .cases()
        .iter()
        .find(|case| {
            case.property(b"name")
                .is_some_and(|n| n == Value::string(name))
        })
        .cloned()
        .expect("the case is declared");
    Value::Object(case)
}

fn suit(name: &str) -> Value {
    case(SUIT, name)
}

fn unit(name: &str) -> Value {
    case(UNIT, name)
}

fn genre(name: &str) -> Value {
    case(BOOK_GENRE, name)
}

fn set_up() -> Arc<EntityManager> {
    orm::entity_manager(orm::connection(), None)
}

/// `setUpEntitySchema($classes)`.
async fn set_up_entity_schema(em: &Arc<EntityManager>, classes: &[&str]) {
    orm::create_schema(em, classes).await;
}

fn assert_same(expected: &Value, actual: &Value) {
    assert!(expected.identical(actual), "{expected:?} !== {actual:?}");
}

/// `assertInstanceOf($class, $value)`.
fn assert_instance_of(class: &str, value: &Value) {
    assert!(
        matches!(value, Value::Object(o) if o.class() == class),
        "{value:?} is not a {class}"
    );
}

fn list(value: &Value) -> Vec<Value> {
    match value {
        Value::Array(array) => array.values().cloned().collect(),
        other => panic!("not a list: {other:?}"),
    }
}

fn field(row: &Value, name: &str) -> Value {
    match row {
        Value::Array(row) => row.get(&name.into()).cloned().unwrap_or(Value::Null),
        Value::Object(_) => get(&obj(row), name),
        other => panic!("not a row: {other:?}"),
    }
}

/// The names of enum cases, sorted (`assertEqualsCanonicalizing()`).
fn canonical_cases(value: &Value) -> Vec<(String, String)> {
    let mut cases: Vec<(String, String)> = list(value)
        .iter()
        .map(|case| {
            let case = obj(case);
            let name = case
                .property(b"name")
                .and_then(|n| n.to_php_string())
                .map_or_default(|n| n.to_string_lossy().into_owned());
            (case.class().to_owned(), name)
        })
        .collect();
    cases.sort();
    cases
}

fn gram_and_meter() -> Vec<(String, String)> {
    canonical_cases(&Value::Array(PhpArray::list([unit("Gram"), unit("Meter")])))
}

/// `new Card()` (or any card class) with a suit.
fn card(class: &str, suit: Value) -> Object {
    let card = new_object(class);
    set(&card, "suit", suit);
    card
}

/// `new Scale()` with its units.
fn scale(units: Vec<Value>) -> Object {
    let scale = new_object(SCALE);
    set(
        &scale,
        "supportedUnits",
        Value::Array(PhpArray::list(units)),
    );
    scale
}

async fn persist_flush_clear(em: &Arc<EntityManager>, entities: &[&Object]) {
    for entity in entities {
        em.persist(entity).unwrap();
    }
    em.flush().await.unwrap();
    em.clear().unwrap();
}

/// `$qb->from($class, $alias)->select($select)->getQuery()->getResult($mode)`.
async fn select(
    em: &Arc<EntityManager>,
    class: &str,
    alias: &str,
    select: Value,
    mode: i64,
) -> Value {
    let mut qb = em.create_query_builder();
    qb.from(class, alias, None)
        .unwrap()
        .select(vec![select])
        .unwrap();
    qb.get_query().unwrap().get_result(mode).await.unwrap()
}

fn register_dtos() {
    // public function __construct(Suit|null $suit)
    register_new_object_class(
        DTO_WITH_ENUM,
        Arc::new(|args: Vec<Value>| {
            let dto = new_object(DTO_WITH_ENUM);
            set(&dto, "suit", args.first().cloned().unwrap_or(Value::Null));
            Ok(Value::Object(dto))
        }),
    );
    // public function __construct(array $supportedUnits)
    register_new_object_class(
        DTO_WITH_ARRAY_OF_ENUMS,
        Arc::new(|args: Vec<Value>| {
            let dto = new_object(DTO_WITH_ARRAY_OF_ENUMS);
            set(
                &dto,
                "supportedUnits",
                args.first().cloned().unwrap_or(Value::Null),
            );
            Ok(Value::Object(dto))
        }),
    );
}

/// `provideCardClasses()` (DBAL has `EnumType`).
const CARD_CLASSES: [&str; 4] = [CARD, TYPED_CARD, CARD_NATIVE_ENUM, TYPED_CARD_NATIVE_ENUM];

#[tokio::test]
async fn testEnumMapping() {
    for card_class in CARD_CLASSES {
        let em = set_up();
        set_up_entity_schema(&em, &[card_class]).await;

        let card = card(card_class, suit("Clubs"));

        em.persist(&card).unwrap();
        em.flush().await.unwrap();
        em.clear().unwrap();

        let fetched_card = em
            .find(card_class, get(&card, "id"))
            .await
            .unwrap()
            .unwrap();

        assert_instance_of(SUIT, &get(&fetched_card, "suit"));
        assert_same(&suit("Clubs"), &get(&fetched_card, "suit"));
    }
}

#[tokio::test]
async fn testEnumHydrationObjectHydrator() {
    let em = set_up();
    set_up_entity_schema(&em, &[CARD]).await;

    let card1 = card(CARD, suit("Clubs"));
    let card2 = card(CARD, suit("Hearts"));

    em.persist(&card1).unwrap();
    em.persist(&card2).unwrap();
    em.flush().await.unwrap();

    drop((card1, card2));
    em.clear().unwrap();

    let mut qb = em.create_query_builder();
    qb.select(vec![s("c")])
        .unwrap()
        .from(CARD, "c", None)
        .unwrap()
        .r#where(vec![s("c.suit = :suit")])
        .unwrap()
        .set_parameter("suit", suit("Clubs"), None);
    let found_cards = list(
        &qb.get_query()
            .unwrap()
            .get_result(HYDRATE_OBJECT)
            .await
            .unwrap(),
    );

    assert_ne!(found_cards.len(), 0);
    for card in &found_cards {
        assert_same(&suit("Clubs"), &field(card, "suit"));
    }
}

#[tokio::test]
async fn testEnumArrayHydrationObjectHydrator() {
    let em = set_up();
    set_up_entity_schema(&em, &[SCALE]).await;

    let scale = scale(vec![unit("Gram"), unit("Meter")]);
    persist_flush_clear(&em, &[&scale]).await;

    let result = list(&select(&em, SCALE, "s", s("s"), HYDRATE_OBJECT).await);

    assert_instance_of(SCALE, &result[0]);
    assert_eq!(
        gram_and_meter(),
        canonical_cases(&field(&result[0], "supportedUnits"))
    );
}

#[tokio::test]
async fn testEnumHydration() {
    let em = set_up();
    set_up_entity_schema(&em, &[CARD, CARD_WITH_NULLABLE]).await;

    let card = card(CARD, suit("Clubs"));
    persist_flush_clear(&em, &[&card]).await;

    let result = list(&select(&em, CARD, "c", s("c.id, c.suit"), HYDRATE_OBJECT).await);

    assert_instance_of(SUIT, &field(&result[0], "suit"));
    assert_same(&suit("Clubs"), &field(&result[0], "suit"));
}

#[tokio::test]
async fn testEnumHydrationArrayHydrator() {
    let em = set_up();
    set_up_entity_schema(&em, &[CARD, CARD_WITH_NULLABLE]).await;

    let card = card(CARD, suit("Clubs"));
    persist_flush_clear(&em, &[&card]).await;

    let result = list(&select(&em, CARD, "c", s("c"), HYDRATE_ARRAY).await);

    assert_instance_of(SUIT, &field(&result[0], "suit"));
    assert_same(&suit("Clubs"), &field(&result[0], "suit"));
}

#[tokio::test]
async fn testNullableEnumHydration() {
    let em = set_up();
    set_up_entity_schema(&em, &[CARD, CARD_WITH_NULLABLE]).await;

    let card_with_nullable = card(CARD_WITH_NULLABLE, Value::Null);
    persist_flush_clear(&em, &[&card_with_nullable]).await;

    let result = list(
        &select(
            &em,
            CARD_WITH_NULLABLE,
            "c",
            s("c.id, c.suit"),
            HYDRATE_OBJECT,
        )
        .await,
    );

    assert!(field(&result[0], "suit").is_null());
}

#[tokio::test]
async fn testEnumArrayHydration() {
    let em = set_up();
    set_up_entity_schema(&em, &[SCALE]).await;

    let scale = scale(vec![unit("Gram"), unit("Meter")]);
    persist_flush_clear(&em, &[&scale]).await;

    let result = list(&select(&em, SCALE, "s", s("s.id, s.supportedUnits"), HYDRATE_OBJECT).await);

    assert_eq!(
        gram_and_meter(),
        canonical_cases(&field(&result[0], "supportedUnits"))
    );
}

#[tokio::test]
async fn testEnumInDtoHydration() {
    register_dtos();
    let em = set_up();
    set_up_entity_schema(&em, &[CARD, CARD_WITH_NULLABLE]).await;

    let card = card(CARD, suit("Clubs"));
    persist_flush_clear(&em, &[&card]).await;

    let result = list(
        &select(
            &em,
            CARD_WITH_NULLABLE,
            "c",
            s(&format!("NEW {DTO_WITH_ENUM}(c.suit)")),
            HYDRATE_OBJECT,
        )
        .await,
    );

    // Upstream selects from CardWithNullable, whose table is empty.
    assert!(
        result
            .first()
            .map_or(Value::Null, |dto| field(dto, "suit"))
            .is_null()
    );
}

#[tokio::test]
async fn testNullableEnumInDtoHydration() {
    register_dtos();
    let em = set_up();
    set_up_entity_schema(&em, &[CARD, CARD_WITH_NULLABLE]).await;

    let card_with_nullable = card(CARD_WITH_NULLABLE, Value::Null);
    persist_flush_clear(&em, &[&card_with_nullable]).await;

    let result = list(
        &select(
            &em,
            CARD_WITH_NULLABLE,
            "c",
            s(&format!("NEW {DTO_WITH_ENUM}(c.suit)")),
            HYDRATE_OBJECT,
        )
        .await,
    );

    assert!(field(&result[0], "suit").is_null());
}

#[tokio::test]
async fn testEnumArrayInDtoHydration() {
    register_dtos();
    let em = set_up();
    set_up_entity_schema(&em, &[SCALE]).await;

    let scale = scale(vec![unit("Gram"), unit("Meter")]);
    persist_flush_clear(&em, &[&scale]).await;

    let func = Func::new(
        format!("NEW {DTO_WITH_ARRAY_OF_ENUMS}"),
        Value::Array(PhpArray::list([s("s.supportedUnits")])),
    );
    let result = list(
        &select(
            &em,
            SCALE,
            "s",
            Value::Object(Arc::new(func)),
            HYDRATE_OBJECT,
        )
        .await,
    );

    assert_eq!(
        gram_and_meter(),
        canonical_cases(&field(&result[0], "supportedUnits"))
    );
}

/// `recomputeSingleEntityChangeSet(getClassMetadata($class), $entity)`.
fn recompute(em: &Arc<EntityManager>, class: &str, entity: &Object) {
    em.get_unit_of_work()
        .recompute_single_entity_change_set(&em.get_class_metadata(class).unwrap(), entity)
        .unwrap();
}

#[tokio::test]
async fn testEnumSingleEntityChangeSetsSimpleObjectHydrator() {
    let em = set_up();
    set_up_entity_schema(&em, &[CARD]).await;

    let card = card(CARD, suit("Clubs"));
    persist_flush_clear(&em, &[&card]).await;

    let result = em.find(CARD, get(&card, "id")).await.unwrap().unwrap();

    recompute(&em, CARD, &result);

    assert!(!em.get_unit_of_work().is_scheduled_for_update(&result));

    set(&result, "suit", suit("Hearts"));

    recompute(&em, CARD, &result);

    assert!(em.get_unit_of_work().is_scheduled_for_update(&result));
}

#[tokio::test]
async fn testEnumSingleEntityChangeSetsObjectHydrator() {
    let em = set_up();
    set_up_entity_schema(&em, &[CARD]).await;

    let card = card(CARD, suit("Clubs"));
    persist_flush_clear(&em, &[&card]).await;

    let result = em.find(CARD, get(&card, "id")).await.unwrap().unwrap();

    recompute(&em, CARD, &result);

    assert!(!em.get_unit_of_work().is_scheduled_for_update(&result));
}

#[tokio::test]
async fn testEnumArraySingleEntityChangeSets() {
    let em = set_up();
    set_up_entity_schema(&em, &[SCALE]).await;

    let scale = scale(vec![unit("Gram")]);
    persist_flush_clear(&em, &[&scale]).await;

    let result = em.find(SCALE, get(&scale, "id")).await.unwrap().unwrap();

    recompute(&em, SCALE, &result);

    assert!(!em.get_unit_of_work().is_scheduled_for_update(&result));
}

#[tokio::test]
async fn testEnumChangeSetsSimpleObjectHydrator() {
    let em = set_up();
    set_up_entity_schema(&em, &[CARD]).await;

    let card = card(CARD, suit("Clubs"));
    persist_flush_clear(&em, &[&card]).await;

    let result = em.find(CARD, get(&card, "id")).await.unwrap().unwrap();

    em.get_unit_of_work().compute_change_sets().await.unwrap();

    assert!(!em.get_unit_of_work().is_scheduled_for_update(&result));
}

#[tokio::test]
async fn testEnumChangeSetsObjectHydrator() {
    let em = set_up();
    set_up_entity_schema(&em, &[CARD]).await;

    let card = card(CARD, suit("Clubs"));
    persist_flush_clear(&em, &[&card]).await;

    let result = list(&select(&em, CARD, "c", s("c"), HYDRATE_OBJECT).await);

    em.get_unit_of_work().compute_change_sets().await.unwrap();

    assert!(
        !em.get_unit_of_work()
            .is_scheduled_for_update(&obj(&result[0]))
    );
}

#[tokio::test]
async fn testEnumArrayChangeSets() {
    let em = set_up();
    set_up_entity_schema(&em, &[SCALE]).await;

    let scale = scale(vec![unit("Gram")]);
    persist_flush_clear(&em, &[&scale]).await;

    let result = list(&select(&em, SCALE, "s", s("s"), HYDRATE_OBJECT).await);

    em.get_unit_of_work().compute_change_sets().await.unwrap();

    assert!(
        !em.get_unit_of_work()
            .is_scheduled_for_update(&obj(&result[0]))
    );
}

#[tokio::test]
async fn testFindByEnum() {
    let em = set_up();
    set_up_entity_schema(&em, &[CARD]).await;

    let card1 = card(CARD, suit("Clubs"));
    let card2 = card(CARD, suit("Hearts"));

    em.persist(&card1).unwrap();
    em.persist(&card2).unwrap();
    em.flush().await.unwrap();

    drop((card1, card2));
    em.clear().unwrap();

    let found_cards = em
        .get_repository(CARD)
        .unwrap()
        .find_by(&php_array! { "suit" => suit("Clubs") }, None, None, None)
        .await
        .unwrap();
    assert_ne!(found_cards.len(), 0);
    for card in &found_cards {
        assert_same(&suit("Clubs"), &get(card, "suit"));
    }
}

#[tokio::test]
async fn testEnumWithNonMatchingDatabaseValueThrowsException() {
    for card_class in CARD_CLASSES {
        if card_class == TYPED_CARD_NATIVE_ENUM {
            // markTestSkipped('MySQL won't allow us to insert invalid
            // values in this case.')
            continue;
        }

        let em = set_up();
        set_up_entity_schema(&em, &[card_class]).await;

        let card = card(card_class, suit("Clubs"));
        persist_flush_clear(&em, &[&card]).await;

        let metadata = em.get_class_metadata(card_class).unwrap();
        let (table, suit_column, id_column) = {
            let metadata = metadata.read();
            (
                metadata.get_table_name().unwrap(),
                metadata
                    .field_mappings
                    .get("suit")
                    .unwrap()
                    .column_name
                    .clone(),
                metadata
                    .field_mappings
                    .get("id")
                    .unwrap()
                    .column_name
                    .clone(),
            )
        };
        let mut data = PhpArray::new();
        data.insert(suit_column.as_str(), s("Z"));
        let mut criteria = PhpArray::new();
        criteria.insert(id_column.as_str(), get(&card, "id"));
        em.get_connection()
            .update(&table, &data, &criteria, &ParameterTypes::new())
            .await
            .unwrap();

        let e = em
            .find(card_class, get(&card, "id"))
            .await
            .expect_err("a MappingException");
        assert_eq!(e.class, MAPPING_EXCEPTION, "{e:?}");
        assert!(
            message(&e).contains(&format!(
                "Context: Trying to hydrate enum property \"{card_class}::$suit\"\nProblem: Case \"Z\" is not listed in enum \"Doctrine\\Tests\\Models\\Enums\\Suit\"\nSolution: Either add the case to the enum type or migrate the database column to use another case of the enum"
            )),
            "{}",
            message(&e)
        );
    }
}

#[tokio::test]
async fn testItAllowsReadingAttributes() {
    let em = set_up();
    let metadata = em.get_class_metadata(CARD).unwrap();
    let accessor = metadata.read().get_property_accessor("suit").unwrap();
    let property = accessor.get_underlying_reflector();

    let attributes = property_attributes(property.class(), property.get_name());

    assert_eq!(attributes.len(), 1);
    assert_eq!(COLUMN, attributes[0].class_name());
}

#[tokio::test]
async fn testEnumMappingWithEmbeddable() {
    let em = set_up();
    set_up_entity_schema(&em, &[PRODUCT]).await;

    let product = new_object(PRODUCT);
    let quantity = new_object(QUANTITY);
    set(&product, "quantity", Value::Object(quantity.clone()));
    set(&quantity, "value", Value::Int(10));
    set(&quantity, "unit", unit("Gram"));

    persist_flush_clear(&em, &[&product]).await;

    let fetched_product = em
        .find(PRODUCT, get(&product, "id"))
        .await
        .unwrap()
        .unwrap();

    let fetched_unit = get(&obj(&get(&fetched_product, "quantity")), "unit");
    assert_instance_of(UNIT, &fetched_unit);
    assert_same(&unit("Gram"), &fetched_unit);
}

#[tokio::test]
async fn testEnumArrayMapping() {
    let em = set_up();
    set_up_entity_schema(&em, &[SCALE]).await;

    let scale = scale(vec![unit("Gram"), unit("Meter")]);
    persist_flush_clear(&em, &[&scale]).await;

    let fetched_scale = em.find(SCALE, get(&scale, "id")).await.unwrap().unwrap();

    let units = get(&fetched_scale, "supportedUnits");
    assert!(matches!(units, Value::Array(_)));
    let units = list(&units);
    assert!(units.iter().any(|u| u.identical(&unit("Gram"))));
    assert!(units.iter().any(|u| u.identical(&unit("Meter"))));
}

/// `uniqid('', true)`: a unique id.
fn uniqid() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    format!("card{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

#[tokio::test]
async fn testEnumWithDefault() {
    let em = set_up();
    set_up_entity_schema(&em, &[CARD_WITH_DEFAULT]).await;

    let table = em
        .get_class_metadata(CARD_WITH_DEFAULT)
        .unwrap()
        .read()
        .get_table_name()
        .unwrap();
    let card_id = uniqid();

    em.get_connection()
        .insert(
            &table,
            &php_array! { "id" => card_id.as_str() },
            &ParameterTypes::new(),
        )
        .await
        .unwrap();
    let card = em
        .find(CARD_WITH_DEFAULT, s(&card_id))
        .await
        .unwrap()
        .unwrap();

    assert_same(&suit("Hearts"), &get(&card, "suit"));
}

/// `provideGenreMatchingExpressions()`.
fn genre_matching_expressions() -> Vec<Comparison> {
    vec![
        Criteria::expr().eq("genre", genre("NON_FICTION")),
        Criteria::expr().r#in("genre", PhpArray::list([genre("NON_FICTION")])),
    ]
}

/// `new Library()`.
fn library() -> Object {
    let library = new_object(LIBRARY);
    set(
        &library,
        "books",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    library
}

/// `new BookWithGenre($genre)`.
fn book_with_genre(genre: Value) -> Object {
    let book = new_object(BOOK_WITH_GENRE);
    set(&book, "genre", genre);
    set(
        &book,
        "categories",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    book
}

/// `new BookCategory()`.
fn book_category() -> Object {
    let category = new_object(BOOK_CATEGORY);
    set(
        &category,
        "books",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    category
}

fn is_initialized(object: &Object, name: &str) -> bool {
    PersistentCollection::from_value(&get(object, name))
        .expect("a persistent collection")
        .is_initialized()
}

async fn assert_matches_nonfiction(
    collection: &Value,
    comparison: Comparison,
    nonfiction: &Object,
) {
    let mut criteria = Criteria::create();
    criteria.r#where(comparison);
    let result = coll::matching(collection, &criteria).await.unwrap();

    assert_eq!(coll::count(&result).await.unwrap(), 1);
    let first = obj(&coll::get(&result, 0).await.unwrap());
    assert_same(&get(nonfiction, "id"), &get(&first, "id"));
}

#[tokio::test]
async fn testEnumCollectionMatchingOnOneToMany() {
    for comparison in genre_matching_expressions() {
        let em = set_up();
        set_up_entity_schema(&em, &[BOOK_WITH_GENRE, LIBRARY, BOOK_CATEGORY]).await;

        let library = library();

        let fiction_book = book_with_genre(genre("FICTION"));
        set(&fiction_book, "library", Value::Object(library.clone()));

        let nonfiction_book = book_with_genre(genre("NON_FICTION"));
        set(&nonfiction_book, "library", Value::Object(library.clone()));

        em.persist(&library).unwrap();
        em.persist(&nonfiction_book).unwrap();
        em.persist(&fiction_book).unwrap();

        em.flush().await.unwrap();
        em.clear().unwrap();

        let library = em
            .find(LIBRARY, get(&library, "id"))
            .await
            .unwrap()
            .unwrap();
        assert!(
            !is_initialized(&library, "books"),
            "Pre-condition: lazy collection"
        );

        assert_matches_nonfiction(&get(&library, "books"), comparison, &nonfiction_book).await;
    }
}

#[tokio::test]
async fn testEnumCollectionMatchingOnManyToMany() {
    for comparison in genre_matching_expressions() {
        let em = set_up();
        set_up_entity_schema(&em, &[LIBRARY, BOOK_WITH_GENRE, BOOK_CATEGORY]).await;

        let category = book_category();

        let fiction_book = book_with_genre(genre("FICTION"));
        coll::add(
            &get(&fiction_book, "categories"),
            Value::Object(category.clone()),
        )
        .unwrap();

        let nonfiction_book = book_with_genre(genre("NON_FICTION"));
        coll::add(
            &get(&nonfiction_book, "categories"),
            Value::Object(category.clone()),
        )
        .unwrap();

        em.persist(&category).unwrap();
        em.persist(&nonfiction_book).unwrap();
        em.persist(&fiction_book).unwrap();

        em.flush().await.unwrap();
        em.clear().unwrap();

        let category = em
            .find(BOOK_CATEGORY, get(&category, "id"))
            .await
            .unwrap()
            .unwrap();
        assert!(
            !is_initialized(&category, "books"),
            "Pre-condition: lazy collection"
        );

        assert_matches_nonfiction(&get(&category, "books"), comparison, &nonfiction_book).await;
    }
}
