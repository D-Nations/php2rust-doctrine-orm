//! Upstream: doctrine/orm tests/Tests/ORM/PersistentCollectionTest.php
//!
//! The entity manager is a `SQLite` one with the `ecommerce` model set
//! (`tests/common/orm.rs`) in place of `EntityManagerMock` over a stubbed
//! connection. The tests that mock `UnitOfWork::loadCollection()` give its
//! callback's elements through the database instead: the cart that owns
//! the collection is persisted, and the elements the callback adds are
//! persisted products of the cart (managed, so the loaded elements are the
//! same objects). The test that mocks `scheduleCollectionDeletion()`
//! checks `isCollectionScheduledForDeletion()`. `unserialize(serialize())`
//! is `serialized_copy()`, `$collection[3] = ...` is `set(3, ...)`, and
//! `stdClass` elements are `ECommerceProduct` objects where the collection
//! is loaded.

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

use php_value::object::StdClass;
use php_value::{Key, Object, PhpArray, Value};
use php2rust_doctrine_collections::ArrayCollection;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;

use orm::{ECOMMERCE_CART, ECOMMERCE_MODELS, ECOMMERCE_PRODUCT, ecommerce_cart, ecommerce_product};

struct Test {
    em: Arc<EntityManager>,
    collection: Arc<PersistentCollection>,
    /// The collection's owner (the collection holds it weakly).
    _owner: Object,
}

async fn set_up() -> Test {
    let em = orm::set_up(&ECOMMERCE_MODELS).await;
    let owner = ecommerce_cart("");
    let collection = set_up_persistent_collection(&em, &owner);
    Test {
        em,
        collection,
        _owner: owner,
    }
}

/// `setUpPersistentCollection()`, owned by `$owner`.
fn set_up_persistent_collection(
    em: &Arc<EntityManager>,
    owner: &Object,
) -> Arc<PersistentCollection> {
    let class_meta_data = em.get_class_metadata(ECOMMERCE_CART).unwrap();
    let collection = PersistentCollection::new(
        Some(em),
        Some(class_meta_data.clone()),
        ArrayCollection::new(PhpArray::new()),
    );
    collection.set_initialized(false);
    let mapping = class_meta_data
        .read()
        .get_association_mapping("products")
        .unwrap()
        .clone();
    collection.set_owner(owner, &mapping);
    collection
}

fn std_object() -> Object {
    Arc::new(StdClass::default())
}

/// The objects of `toArray()`, compared by identity.
async fn assert_same_elements(expected: &[&Object], collection: &Arc<PersistentCollection>) {
    let actual = collection.to_array().await.unwrap();
    assert_eq!(expected.len(), actual.len());
    for (expected, actual) in expected.iter().zip(actual.values()) {
        assert!(orm::same_value(actual, expected), "{actual:?}");
    }
}

/// The cart of the collection, persisted with the products the mocked
/// `loadCollection()` adds, and the collection of that managed cart.
async fn persisted_cart(
    em: &Arc<EntityManager>,
    products: &[&Object],
) -> (Object, Arc<PersistentCollection>) {
    let cart = ecommerce_cart("");
    for product in products {
        orm::append(&cart, "products", Value::Object((*product).clone()));
    }
    em.persist(&cart).unwrap();
    em.flush().await.unwrap();
    let collection = set_up_persistent_collection(em, &cart);
    (cart, collection)
}

#[tokio::test]
async fn testCanBePutInLazyLoadingMode() {
    let t = set_up().await;
    let class = t.em.get_class_metadata(ECOMMERCE_PRODUCT).unwrap();
    let collection = PersistentCollection::new(
        Some(&t.em),
        Some(class),
        ArrayCollection::new(PhpArray::new()),
    );
    collection.set_initialized(false);
    assert!(!collection.is_initialized());
}

/// Test that `PersistentCollection::current()` initializes the collection.
#[tokio::test]
async fn testCurrentInitializesCollection() {
    let t = set_up().await;
    t.collection.current().await.unwrap();
    assert!(t.collection.is_initialized());
}

/// Test that `PersistentCollection::key()` initializes the collection.
#[tokio::test]
async fn testKeyInitializesCollection() {
    let t = set_up().await;
    t.collection.key().await.unwrap();
    assert!(t.collection.is_initialized());
}

/// Test that `PersistentCollection::next()` initializes the collection.
#[tokio::test]
async fn testNextInitializesCollection() {
    let t = set_up().await;
    t.collection.next().await.unwrap();
    assert!(t.collection.is_initialized());
}

/// DDC-3382
#[tokio::test]
async fn testNonObjects() {
    let t = set_up().await;
    assert!(t.collection.is_empty().await.unwrap());

    t.collection.add(Value::string("dummy")).unwrap();

    assert!(!t.collection.is_empty().await.unwrap());

    let product = ecommerce_product("");

    t.collection
        .set(Key::Int(1), Value::Object(product.clone()))
        .await
        .unwrap();
    t.collection
        .set(Key::Int(2), Value::string("dummy"))
        .await
        .unwrap();
    t.collection.set(Key::Int(3), Value::Null).await.unwrap();

    assert!(orm::same_value(
        &t.collection.get(&Key::Int(1)).await.unwrap(),
        &product
    ));
    assert_eq!(
        Value::string("dummy"),
        t.collection.get(&Key::Int(2)).await.unwrap()
    );
    assert_eq!(Value::Null, t.collection.get(&Key::Int(3)).await.unwrap());
}

/// The keys of `toArray()`.
async fn keys(collection: &Arc<PersistentCollection>) -> Vec<Key> {
    collection
        .to_array()
        .await
        .unwrap()
        .keys()
        .cloned()
        .collect()
}

/// 6110
#[tokio::test]
async fn testRemovingElementsAlsoRemovesKeys() {
    let t = set_up().await;
    let dummy = Value::Object(std_object());

    t.collection.add(dummy.clone()).unwrap();
    assert_eq!(vec![Key::Int(0)], keys(&t.collection).await);

    t.collection.remove_element(&dummy).await.unwrap();
    assert_eq!(Vec::<Key>::new(), keys(&t.collection).await);
}

/// 6110
#[tokio::test]
async fn testClearWillAlsoClearKeys() {
    let t = set_up().await;
    t.collection.add(Value::Object(std_object())).unwrap();
    t.collection.clear().await.unwrap();
    assert_eq!(Vec::<Key>::new(), keys(&t.collection).await);
}

/// 6110
#[tokio::test]
async fn testClearWillAlsoResetKeyPositions() {
    let t = set_up().await;
    let dummy = Value::Object(std_object());

    t.collection.add(dummy.clone()).unwrap();
    t.collection.remove_element(&dummy).await.unwrap();
    t.collection.clear().await.unwrap();
    t.collection.add(dummy).unwrap();
    assert_eq!(vec![Key::Int(0)], keys(&t.collection).await);
}

/// 6613, 6614, 6616
#[tokio::test]
async fn testWillKeepNewItemsInDirtyCollectionAfterInitialization() {
    let t = set_up().await;

    let new_element = ecommerce_product("new");
    let persisted_element = ecommerce_product("persisted");
    let (_cart, collection) = persisted_cart(&t.em, &[&persisted_element]).await;

    collection.add(Value::Object(new_element.clone())).unwrap();

    assert!(!collection.is_initialized());
    assert!(collection.is_dirty());

    collection.initialize().await.unwrap();

    assert_same_elements(&[&persisted_element, &new_element], &collection).await;
    assert!(collection.is_initialized());
    assert!(collection.is_dirty());
}

/// 6613, 6614, 6616
#[tokio::test]
async fn testWillDeDuplicateNewItemsThatWerePreviouslyPersistedInDirtyCollectionAfterInitialization()
 {
    let t = set_up().await;

    let new_element = ecommerce_product("new");
    let new_element_that_is_also_persisted = ecommerce_product("also persisted");
    let persisted_element = ecommerce_product("persisted");
    let (_cart, collection) = persisted_cart(
        &t.em,
        &[&new_element_that_is_also_persisted, &persisted_element],
    )
    .await;

    collection
        .add(Value::Object(new_element_that_is_also_persisted.clone()))
        .unwrap();
    collection.add(Value::Object(new_element.clone())).unwrap();

    assert!(!collection.is_initialized());
    assert!(collection.is_dirty());

    collection.initialize().await.unwrap();

    assert_same_elements(
        &[
            &new_element_that_is_also_persisted,
            &persisted_element,
            &new_element,
        ],
        &collection,
    )
    .await;
    assert!(collection.is_initialized());
    assert!(collection.is_dirty());
}

/// 6613, 6614, 6616
#[tokio::test]
async fn testWillNotMarkCollectionAsDirtyAfterInitializationIfNoElementsWereAdded() {
    let t = set_up().await;

    let new_element_that_is_also_persisted = ecommerce_product("also persisted");
    let persisted_element = ecommerce_product("persisted");
    let (_cart, collection) = persisted_cart(
        &t.em,
        &[&new_element_that_is_also_persisted, &persisted_element],
    )
    .await;

    collection
        .add(Value::Object(new_element_that_is_also_persisted.clone()))
        .unwrap();

    assert!(!collection.is_initialized());
    assert!(collection.is_dirty());

    collection.initialize().await.unwrap();

    assert_same_elements(
        &[&new_element_that_is_also_persisted, &persisted_element],
        &collection,
    )
    .await;
    assert!(collection.is_initialized());
    assert!(!collection.is_dirty());
}

#[tokio::test]
async fn testModifyUOWForDeferredImplicitOwnerOnClear() {
    let t = set_up().await;
    let unit_of_work = t.em.get_unit_of_work();
    assert!(!unit_of_work.is_collection_scheduled_for_deletion(&t.collection));

    t.collection.clear().await.unwrap();

    assert!(unit_of_work.is_collection_scheduled_for_deletion(&t.collection));
}

#[tokio::test]
async fn testItCanBeSerializedAndUnserializedBack() {
    let t = set_up().await;
    t.collection.add(Value::Object(std_object())).unwrap();
    let collection = t.collection.serialized_copy();
    collection.add(Value::Object(std_object())).unwrap();
    collection
        .set(Key::Int(3), Value::Object(std_object()))
        .await
        .unwrap();
    assert_eq!(3, collection.count().await.unwrap());
}
