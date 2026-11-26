//! Functional/PostLoadEventTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the cms models). The mocked `PostLoadListener` is an
//! `EventListener` that counts its `postLoad()` calls, checked as the
//! mock's `expects()`; a proxy's first property access is
//! `initialize_object()`.
//!
//! Pending: `testEventRaisedCorrectTimesWhenOtherEntityLoadedInEventHandler`
//! (its listener initializes a lazy proxy inside `postLoad()`; a
//! listener's call is synchronous and the load is async).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/PostLoadEventTest.php

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

use std::any::Any;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use php_value::{Thrown, Value};
use php2rust_doctrine_event_manager::{
    EventArgs, EventListener, EventListenerRegistry, ListenerError,
};
use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::event::LifecycleEventArgs;
use php2rust_doctrine_orm::events;
use php2rust_doctrine_orm::query::context::HYDRATE_OBJECT;

use orm::{
    CMS_MODELS, CMS_USER, add_phonenumber, cms_address, cms_email, cms_phonenumber, cms_user_with,
    get, set_address, set_email,
};

struct Fixture {
    em: Arc<EntityManager>,
    user_id: Value,
}

async fn set_up() -> Fixture {
    let em = orm::set_up(&CMS_MODELS).await;
    let user_id = load_fixture(&em).await;
    Fixture { em, user_id }
}

/// The mocked `PostLoadListener`: counts the `postLoad()` calls.
#[derive(Default)]
struct PostLoadListener {
    calls: AtomicUsize,
}

impl EventListener for PostLoadListener {
    fn call(&self, event_name: &str, _event_args: &dyn EventArgs) -> Result<(), ListenerError> {
        assert_eq!(events::POST_LOAD, event_name);
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

impl PostLoadListener {
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl Fixture {
    /// `$eventManager->addEventListener([Events::postLoad], $mockListener)`.
    fn add_listener(&self, listener: &Arc<PostLoadListener>) {
        let listener: Arc<dyn EventListener> = listener.clone();
        self.em.with_event_manager(|evm| {
            evm.add_event_listener(&[events::POST_LOAD], listener);
        });
    }

    /// `$query->setParameter('id', $this->userId)->getResult()`.
    async fn result(&self, dql: &str) {
        let query = self.em.create_query(dql);
        query.set_parameter("id", self.user_id.clone(), None);
        query.get_result(HYDRATE_OBJECT).await.unwrap();
    }

    async fn find_user(&self) -> php_value::Object {
        self.em
            .find(CMS_USER, self.user_id.clone())
            .await
            .unwrap()
            .unwrap()
    }
}

#[tokio::test]
async fn testLoadedEntityUsingFindShouldTriggerEvent() {
    let f = set_up().await;
    let mock_listener = Arc::new(PostLoadListener::default());

    // CmsUser and CmsAddres, because it's a ToOne inverse side on CmsUser
    f.add_listener(&mock_listener);

    f.find_user().await;
    assert_eq!(2, mock_listener.calls());
}

#[tokio::test]
async fn testLoadedEntityUsingQueryShouldTriggerEvent() {
    let f = set_up().await;
    let mock_listener = Arc::new(PostLoadListener::default());

    // CmsUser and CmsAddres, because it's a ToOne inverse side on CmsUser
    f.add_listener(&mock_listener);

    f.result("SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u WHERE u.id = :id")
        .await;
    assert_eq!(2, mock_listener.calls());
}

#[tokio::test]
async fn testLoadedAssociationToOneShouldTriggerEvent() {
    let f = set_up().await;
    let mock_listener = Arc::new(PostLoadListener::default());

    // CmsUser (root), CmsAddress (ToOne inverse side), CmsEmail (joined association)
    f.add_listener(&mock_listener);

    f.result(
        "SELECT u, e FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u JOIN u.email e WHERE u.id = :id",
    )
    .await;
    assert_eq!(3, mock_listener.calls());
}

#[tokio::test]
async fn testLoadedAssociationToManyShouldTriggerEvent() {
    let f = set_up().await;
    let mock_listener = Arc::new(PostLoadListener::default());

    // CmsUser (root), CmsAddress (ToOne inverse side), 2 CmsPhonenumber (joined association)
    f.add_listener(&mock_listener);

    f.result("SELECT u, p FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u JOIN u.phonenumbers p WHERE u.id = :id")
        .await;
    assert_eq!(4, mock_listener.calls());
}

#[tokio::test]
async fn testLoadedProxyEntityShouldTriggerEvent() {
    let f = set_up().await;

    // Should not be invoked during getReference call
    let mock_listener = Arc::new(PostLoadListener::default());
    f.add_listener(&mock_listener);

    let user_proxy =
        f.em.get_reference(CMS_USER, f.user_id.clone())
            .await
            .unwrap()
            .unwrap();
    assert_eq!(0, mock_listener.calls());

    // Now deactivate original listener and attach new one
    let removed: Arc<dyn EventListener> = mock_listener.clone();
    f.em.with_event_manager(|evm| {
        evm.remove_event_listener(&[events::POST_LOAD], &removed);
    });

    let mock_listener2 = Arc::new(PostLoadListener::default());
    f.add_listener(&mock_listener2);

    // $userProxy->getName()
    f.em.initialize_object(&user_proxy).await.unwrap();
    get(&user_proxy, "name");
    assert_eq!(2, mock_listener2.calls());
    assert_eq!(0, mock_listener.calls());
}

#[tokio::test]
async fn testLoadedProxyPartialShouldTriggerEvent() {
    let f = set_up().await;

    // Should not be invoked during getReference call
    let mock_listener = Arc::new(PostLoadListener::default());

    // CmsUser (partially loaded), CmsAddress (inverse ToOne), 2 CmsPhonenumber
    f.add_listener(&mock_listener);

    f.result("SELECT PARTIAL u.{id, name}, p FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u JOIN u.phonenumbers p WHERE u.id = :id")
        .await;
    assert_eq!(4, mock_listener.calls());
}

#[tokio::test]
async fn testLoadedProxyAssociationToOneShouldTriggerEvent() {
    let f = set_up().await;
    let user = f.find_user().await;

    let mock_listener = Arc::new(PostLoadListener::default());

    // CmsEmail (proxy)
    f.add_listener(&mock_listener);

    let Value::Object(email_proxy) = get(&user, "email") else {
        panic!("an email proxy");
    };
    // $emailProxy->getEmail()
    f.em.initialize_object(&email_proxy).await.unwrap();
    get(&email_proxy, "email");
    assert_eq!(1, mock_listener.calls());
}

#[tokio::test]
async fn testLoadedProxyAssociationToManyShouldTriggerEvent() {
    let f = set_up().await;
    let user = f.find_user().await;

    let mock_listener = Arc::new(PostLoadListener::default());

    // 2 CmsPhonenumber (proxy)
    f.add_listener(&mock_listener);

    let phonenumbers_col = get(&user, "phonenumbers");
    coll::first(&phonenumbers_col).await.unwrap();
    assert_eq!(2, mock_listener.calls());
}

/// `PostLoadListenerCheckAssociationsArePopulated`.
#[derive(Default)]
struct PostLoadListenerCheckAssociationsArePopulated {
    checked: AtomicBool,
    populated: AtomicBool,
}

impl EventListener for PostLoadListenerCheckAssociationsArePopulated {
    fn call(&self, _event_name: &str, event: &dyn EventArgs) -> Result<(), ListenerError> {
        let any: &dyn Any = event;
        let event = any
            .downcast_ref::<LifecycleEventArgs>()
            .expect("PostLoadEventArgs");
        let object = event.get_object();
        if object.instance_of(CMS_USER) {
            if self.checked.load(Ordering::SeqCst) {
                return Err(Box::new(Thrown::new(
                    "RuntimeException",
                    "Expected to be one user!",
                    0,
                )));
            }
            self.checked.store(true, Ordering::SeqCst);
            self.populated
                .store(!get(object, "email").is_null(), Ordering::SeqCst);
        }
        Ok(())
    }
}

#[tokio::test]
async fn testAssociationsArePopulatedWhenEventIsFired() {
    let f = set_up().await;
    let checker_listener = Arc::new(PostLoadListenerCheckAssociationsArePopulated::default());
    let listener: Arc<dyn EventListener> = checker_listener.clone();
    f.em.with_event_manager(|evm| {
        evm.add_event_listener(&[events::POST_LOAD], listener);
    });

    let mut qb =
        f.em.get_repository(CMS_USER)
            .unwrap()
            .create_query_builder("u", None)
            .unwrap();
    qb.left_join("u.email", "email", None, None, None).unwrap();
    qb.add_select(vec![Value::string("email")]).unwrap();
    qb.get_query()
        .unwrap()
        .get_single_result(None)
        .await
        .unwrap();

    assert!(
        checker_listener.checked.load(Ordering::SeqCst),
        "postLoad event is not invoked"
    );
    assert!(
        checker_listener.populated.load(Ordering::SeqCst),
        "Association of email is not populated in postLoad event"
    );
}

async fn load_fixture(em: &EntityManager) -> Value {
    let user = cms_user_with("Roman", "romanb", "developer");

    let address = cms_address("Germany", "12345", "Berlin");
    set_address(&user, &address);

    let email = cms_email("roman@domain.com");
    set_email(&user, &email);

    let ph1 = cms_phonenumber("0301234");
    let ph2 = cms_phonenumber("987654321");
    add_phonenumber(&user, &ph1);
    add_phonenumber(&user, &ph2);

    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    let user_id = get(&user, "id");
    em.clear().unwrap();
    user_id
}
