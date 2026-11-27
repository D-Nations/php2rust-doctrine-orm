//! Upstream: doctrine/orm tests/Tests/ORM/Functional/OneToOneBidirectionalAssociationTest.php
//!
//! On `SQLite` (`tests/common/orm.rs`) with the ecommerce models; their
//! methods are the functions below. Changing an association's fetch mode
//! changes the entity manager's metadata in place (each test has its own).

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

use php_value::{Object, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::class_metadata::{FETCH_EAGER, FETCH_LAZY};

use orm::{
    ECOMMERCE_CART, ECOMMERCE_CUSTOMER, ECOMMERCE_MODELS, ecommerce_cart, get, new_object, s, same,
    same_value, set,
};

struct Fixture {
    em: Arc<EntityManager>,
    customer: Object,
    cart: Object,
}

/// `setUp()`.
async fn set_up() -> Fixture {
    let em = orm::set_up(&ECOMMERCE_MODELS).await;
    Fixture {
        em,
        customer: ecommerce_customer("John Doe"),
        cart: ecommerce_cart("Credit card"),
    }
}

/// `new ECommerceCustomer()` and `setName($name)`.
fn ecommerce_customer(name: &str) -> Object {
    let customer = new_object(ECOMMERCE_CUSTOMER);
    set(&customer, "name", s(name));
    customer
}

fn is(value: &Value, object: &Object) -> bool {
    same_value(value, object)
}

/// `$customer->setCart($cart)`.
fn set_cart(customer: &Object, cart: &Object) {
    if !is(&get(customer, "cart"), cart) {
        set(customer, "cart", Value::Object(cart.clone()));
        set_customer(cart, customer);
    }
}

/// `$cart->setCustomer($customer)`.
fn set_customer(cart: &Object, customer: &Object) {
    if !is(&get(cart, "customer"), customer) {
        set(cart, "customer", Value::Object(customer.clone()));
        set_cart(customer, cart);
    }
}

/// `$customer->brokenSetCart($cart)`: does not properly maintain the
/// bidirectional association!
fn broken_set_cart(customer: &Object, cart: &Object) {
    set(customer, "cart", Value::Object(cart.clone()));
}

/// `$customer->removeCart()`.
fn remove_cart(customer: &Object) {
    if let Value::Object(cart) = get(customer, "cart") {
        set(customer, "cart", Value::Null);
        remove_customer(&cart);
    }
}

/// `$cart->removeCustomer()`.
fn remove_customer(cart: &Object) {
    if let Value::Object(customer) = get(cart, "customer") {
        set(cart, "customer", Value::Null);
        remove_cart(&customer);
    }
}

/// `$customer->getCart()` or `$cart->getCustomer()`, through the entity
/// manager for a lazy object.
async fn get_one(em: &Arc<EntityManager>, entity: &Object, field: &str) -> Object {
    let object = orm::obj(&get(entity, field));
    em.initialize_object(&object).await.unwrap();
    object
}

impl Fixture {
    /// `createFixture()`.
    async fn create_fixture(&self) {
        let customer = ecommerce_customer("Giorgio");
        let cart = ecommerce_cart("paypal");
        set_cart(&customer, &cart);

        self.em.persist(&customer).unwrap();
        self.em.flush().await.unwrap();
        self.em.clear().unwrap();
    }

    /// `assertCartForeignKeyIs($value)`.
    async fn assert_cart_foreign_key_is(&self, value: Value) {
        let foreign_key = orm::fetch_one_with(
            &self.em,
            "SELECT customer_id FROM ecommerce_carts WHERE id=?",
            vec![get(&self.cart, "id")],
        )
        .await;
        orm::assert_loose_eq(&value, &foreign_key);
    }

    /// `$metadata->associationMappings[$field]->fetch = $fetch`.
    fn set_fetch(&self, class: &str, field: &str, fetch: i64) {
        let metadata = self.em.get_class_metadata(class).unwrap();
        metadata
            .write()
            .association_mappings
            .get_mut(field)
            .unwrap()
            .fetch = Some(fetch);
    }
}

#[tokio::test]
async fn testSavesAOneToOneAssociationWithCascadeSaveSet() {
    let f = set_up().await;
    set_cart(&f.customer, &f.cart);
    f.em.persist(&f.customer).unwrap();
    f.em.flush().await.unwrap();

    f.assert_cart_foreign_key_is(get(&f.customer, "id")).await;
}

#[tokio::test]
async fn testDoesNotSaveAnInverseSideSet() {
    let f = set_up().await;
    broken_set_cart(&f.customer, &f.cart);
    f.em.persist(&f.customer).unwrap();
    f.em.flush().await.unwrap();

    f.assert_cart_foreign_key_is(Value::Null).await;
}

#[tokio::test]
async fn testRemovesOneToOneAssociation() {
    let f = set_up().await;
    set_cart(&f.customer, &f.cart);
    f.em.persist(&f.customer).unwrap();
    remove_cart(&f.customer);

    f.em.flush().await.unwrap();

    f.assert_cart_foreign_key_is(Value::Null).await;
}

#[tokio::test]
async fn testEagerLoad() {
    let f = set_up().await;
    f.create_fixture().await;

    let query = f.em.create_query(
        "select c, ca from Doctrine\\Tests\\Models\\ECommerce\\ECommerceCustomer c join c.cart ca",
    );
    let result = query.get_object_result().await.unwrap();
    let customer = orm::obj(&orm::at(&result, 0));

    let cart = get_one(&f.em, &customer, "cart").await;
    assert_eq!(ECOMMERCE_CART, cart.class());
    assert_eq!(s("paypal"), get(&cart, "payment"));
}

#[tokio::test]
async fn testLazyLoadsObjectsOnTheOwningSide() {
    let f = set_up().await;
    f.create_fixture().await;
    f.set_fetch(ECOMMERCE_CART, "customer", FETCH_LAZY);

    let query =
        f.em.create_query("select c from Doctrine\\Tests\\Models\\ECommerce\\ECommerceCart c");
    let result = query.get_object_result().await.unwrap();
    let cart = orm::obj(&orm::at(&result, 0));

    let customer = get_one(&f.em, &cart, "customer").await;
    assert_eq!(ECOMMERCE_CUSTOMER, customer.class());
    assert_eq!(s("Giorgio"), get(&customer, "name"));
}

#[tokio::test]
async fn testInverseSideIsNeverLazy() {
    let f = set_up().await;
    f.create_fixture().await;
    f.set_fetch(ECOMMERCE_CUSTOMER, "mentor", FETCH_EAGER);

    let query =
        f.em.create_query("select c from Doctrine\\Tests\\Models\\ECommerce\\ECommerceCustomer c");
    let result = query.get_object_result().await.unwrap();
    let customer = orm::obj(&orm::at(&result, 0));

    assert!(get(&customer, "mentor").is_null());
    let cart = get(&customer, "cart");
    assert_eq!(ECOMMERCE_CART, orm::obj(&cart).class());
    assert!(!f.em.is_uninitialized_object(&cart));
    assert_eq!(s("paypal"), get(&orm::obj(&cart), "payment"));
}

#[tokio::test]
async fn testUpdateWithProxyObject() {
    let f = set_up().await;
    let cust = ecommerce_customer("Roman");
    let cart = ecommerce_cart("CARD");
    set_cart(&cust, &cart);

    f.em.persist(&cust).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    assert_eq!(ECOMMERCE_CART, orm::obj(&get(&cust, "cart")).class());
    assert_eq!(s("Roman"), get(&cust, "name"));
    assert!(same(&cust, &orm::obj(&get(&cart, "customer"))));

    let query = f.em.create_query(
        "select ca from Doctrine\\Tests\\Models\\ECommerce\\ECommerceCart ca where ca.id =?1",
    );
    query.set_parameter(1, get(&cart, "id"), None);
    let cart2 = orm::obj(&query.get_single_result(None).await.unwrap());
    set(&cart2, "payment", s("CHEQUE"));
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    let query2 = f.em.create_query(
        "select ca, c from Doctrine\\Tests\\Models\\ECommerce\\ECommerceCart ca left join ca.customer c where ca.id =?1",
    );
    query2.set_parameter(1, get(&cart, "id"), None);
    let cart3 = orm::obj(&query2.get_single_result(None).await.unwrap());
    let customer = get_one(&f.em, &cart3, "customer").await;
    assert_eq!(ECOMMERCE_CUSTOMER, customer.class());
    assert_eq!(s("Roman"), get(&customer, "name"));
}
