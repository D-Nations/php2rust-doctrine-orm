//! DDC117Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` with the `ddc117` model set (composite keys made of
//! associations). The models' constructors and methods are the functions
//! below. Reading a property of a lazy proxy does not load it, so the
//! tests call `initialize_object()` where PHP's property read would; the
//! lazy collections load in the async collection functions
//! (`count()`, `get()`, `get_values()`) where PHP's `count()`, `[0]` and
//! `foreach` would.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC117Test.php
//! Upstream: doctrine/orm tests/Tests/Models/DDC117/DDC117Article.php tests/Tests/Models/DDC117/DDC117ArticleDetails.php
//! Upstream: doctrine/orm tests/Tests/Models/DDC117/DDC117Reference.php tests/Tests/Models/DDC117/DDC117Translation.php
//! Upstream: doctrine/orm tests/Tests/Models/DDC117/DDC117Link.php tests/Tests/Models/DDC117/DDC117Editor.php
//! Upstream: doctrine/orm tests/Tests/Models/DDC117/DDC117ApproveChanges.php

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

use php_value::datetime::{DateTime, DateTimeImmutable, TimeZone};
use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;
use php2rust_doctrine_orm::unit_of_work::{STATE_MANAGED, STATE_NEW};

use orm::*;

const DDC117_APPROVE_CHANGES: &str = "Doctrine\\Tests\\Models\\DDC117\\DDC117ApproveChanges";
const DDC117_ARTICLE: &str = "Doctrine\\Tests\\Models\\DDC117\\DDC117Article";
const DDC117_ARTICLE_DETAILS: &str = "Doctrine\\Tests\\Models\\DDC117\\DDC117ArticleDetails";
const DDC117_EDITOR: &str = "Doctrine\\Tests\\Models\\DDC117\\DDC117Editor";
const DDC117_LINK: &str = "Doctrine\\Tests\\Models\\DDC117\\DDC117Link";
const DDC117_REFERENCE: &str = "Doctrine\\Tests\\Models\\DDC117\\DDC117Reference";
const DDC117_TRANSLATION: &str = "Doctrine\\Tests\\Models\\DDC117\\DDC117Translation";

/// The `ddc117` model set.
const DDC117_MODELS: [&str; 7] = [
    DDC117_ARTICLE,
    DDC117_REFERENCE,
    DDC117_TRANSLATION,
    DDC117_ARTICLE_DETAILS,
    DDC117_APPROVE_CHANGES,
    DDC117_EDITOR,
    DDC117_LINK,
];

fn new_collection() -> Value {
    ArrayCollectionObject::value(PhpArray::new())
}

/// `new DDC117Article($title)`.
fn article(title: &str) -> Object {
    let article = new_object(DDC117_ARTICLE);
    set(&article, "title", s(title));
    set(&article, "references", new_collection());
    set(&article, "translations", new_collection());
    article
}

/// `$article->addTranslation($language, $title)`.
fn add_translation(article: &Object, language: &str, title: &str) {
    coll::add(
        &get(article, "translations"),
        Value::Object(translation(article, language, title)),
    )
    .unwrap();
}

/// `new DDC117ArticleDetails($article, $text)`.
fn article_details(article: &Object, text: &str) -> Object {
    let details = new_object(DDC117_ARTICLE_DETAILS);
    set(&details, "article", Value::Object(article.clone()));
    set(article, "details", Value::Object(details.clone()));
    set(&details, "text", s(text));
    details
}

/// `new DDC117Link($source, $target, $description)`.
fn link(source: &Object, target: &Object) -> Object {
    let link = new_object(DDC117_LINK);
    set(&link, "source", Value::Object(source.clone()));
    set(&link, "target", Value::Object(target.clone()));
    link
}

/// `new DDC117Reference($source, $target, $description)`.
fn reference(source: &Object, target: &Object, description: &str) -> Object {
    let reference = new_object(DDC117_REFERENCE);
    set(&reference, "description", s(description));
    coll::add(&get(source, "references"), Value::Object(reference.clone())).unwrap();
    coll::add(&get(target, "references"), Value::Object(reference.clone())).unwrap();
    set(&reference, "source", Value::Object(source.clone()));
    set(&reference, "target", Value::Object(target.clone()));
    set(
        &reference,
        "created",
        DateTime::object(DateTimeImmutable::now(TimeZone::Utc)),
    );
    reference
}

/// `new DDC117Translation($article, $language, $title)`.
fn translation(article: &Object, language: &str, title: &str) -> Object {
    let translation = new_object(DDC117_TRANSLATION);
    set(&translation, "article", Value::Object(article.clone()));
    set(&translation, "language", s(language));
    set(&translation, "title", s(title));
    set(&translation, "reviewedByEditors", new_collection());
    set(&translation, "lastTranslatedBy", new_collection());
    translation
}

/// `new DDC117Editor($name)`.
fn editor(name: &str) -> Object {
    let editor = new_object(DDC117_EDITOR);
    set(&editor, "name", s(name));
    set(&editor, "reviewingTranslations", new_collection());
    editor
}

/// `$editor->addLastTranslation($t)`.
fn add_last_translation(editor: &Object, t: &Object) {
    set(editor, "lastTranslation", Value::Object(t.clone()));
    coll::add(&get(t, "lastTranslatedBy"), Value::Object(editor.clone())).unwrap();
}

/// `new DDC117ApproveChanges($articleDetails, $reference, $translation)`.
fn approve_changes(details: &Object, reference: &Object, translation: &Object) -> Object {
    let approve_changes = new_object(DDC117_APPROVE_CHANGES);
    set(
        &approve_changes,
        "articleDetails",
        Value::Object(details.clone()),
    );
    set(
        &approve_changes,
        "reference",
        Value::Object(reference.clone()),
    );
    set(
        &approve_changes,
        "translation",
        Value::Object(translation.clone()),
    );
    approve_changes
}

struct Fixture {
    em: Arc<EntityManager>,
    article1: Object,
    article2: Object,
    translation: Object,
}

impl Fixture {
    fn article1_id(&self) -> Value {
        get(&self.article1, "id")
    }

    fn article2_id(&self) -> Value {
        get(&self.article2, "id")
    }

    /// `['source' => $this->article1->id(), 'target' => $this->article2->id()]`.
    fn reference_id(&self) -> Value {
        let mut criteria = PhpArray::new();
        criteria.insert("source", self.article1_id());
        criteria.insert("target", self.article2_id());
        Value::Array(criteria)
    }

    /// `['article' => $this->article1->id(), 'language' => 'en']`.
    fn translation_id(&self) -> Value {
        let mut criteria = PhpArray::new();
        criteria.insert("article", self.article1_id());
        criteria.insert("language", s("en"));
        Value::Array(criteria)
    }

    async fn find(&self, class: &str, id: Value) -> Option<Object> {
        self.em.find(class, id).await.unwrap()
    }
}

async fn set_up() -> Fixture {
    let em = orm::set_up(&DDC117_MODELS).await;

    let article1 = article("Foo");
    let article2 = article("Bar");

    em.persist(&article1).unwrap();
    em.persist(&article2).unwrap();
    em.flush().await.unwrap();

    let link = link(&article1, &article2);
    em.persist(&link).unwrap();

    let reference = reference(&article1, &article2, "Test-Description");
    em.persist(&reference).unwrap();

    let translation = translation(&article1, "en", "Bar");
    em.persist(&translation).unwrap();

    let article_details = article_details(&article1, "Very long text");
    em.persist(&article_details).unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    Fixture {
        em,
        article1,
        article2,
        translation,
    }
}

fn assert_instance_of(class: &str, object: &Object) {
    assert_eq!(class, object.class());
}

fn assert_value_instance_of(class: &str, value: &Value) {
    assert_instance_of(class, &obj(value));
}

/// `assertContainsOnly($class, $collection)`.
async fn assert_contains_only(class: &str, collection: &Value) {
    for element in coll::get_values(collection).await.unwrap() {
        assert_value_instance_of(class, &element);
    }
}

fn is_initialized(collection: &Value) -> bool {
    PersistentCollection::from_value(collection)
        .expect("a persistent collection")
        .is_initialized()
}

async fn single_result(em: &Arc<EntityManager>, dql: &str, parameters: &[Value]) -> Object {
    let query = em.create_query(dql);
    for (i, parameter) in parameters.iter().enumerate() {
        query.set_parameter(i64::try_from(i).unwrap() + 1, parameter.clone(), None);
    }
    obj(&query.get_single_result(None).await.unwrap())
}

#[tokio::test]
async fn testAssociationOnlyCompositeKey() {
    let t = set_up().await;
    let id_criteria = t.reference_id();

    let map_ref = t.find(DDC117_REFERENCE, id_criteria.clone()).await.unwrap();
    assert_instance_of(DDC117_REFERENCE, &map_ref);
    assert_value_instance_of(DDC117_ARTICLE, &get(&map_ref, "target"));
    assert_value_instance_of(DDC117_ARTICLE, &get(&map_ref, "source"));
    assert!(same(
        &map_ref,
        &t.find(DDC117_REFERENCE, id_criteria.clone()).await.unwrap()
    ));

    t.em.clear().unwrap();

    let dql = format!("SELECT r, s FROM {DDC117_REFERENCE} r JOIN r.source s WHERE r.source = ?1");
    let dql_ref = single_result(&t.em, &dql, &[Value::Int(1)]).await;

    assert_instance_of(DDC117_REFERENCE, &map_ref);
    assert_value_instance_of(DDC117_ARTICLE, &get(&map_ref, "target"));
    assert_value_instance_of(DDC117_ARTICLE, &get(&map_ref, "source"));
    assert!(same(
        &dql_ref,
        &t.find(DDC117_REFERENCE, id_criteria.clone()).await.unwrap()
    ));

    t.em.clear().unwrap();

    let dql = format!("SELECT r, s FROM {DDC117_REFERENCE} r JOIN r.source s WHERE s.title = ?1");
    let dql_ref = single_result(&t.em, &dql, &[s("Foo")]).await;

    assert_instance_of(DDC117_REFERENCE, &dql_ref);
    assert_value_instance_of(DDC117_ARTICLE, &get(&dql_ref, "target"));
    assert_value_instance_of(DDC117_ARTICLE, &get(&dql_ref, "source"));
    assert!(same(
        &dql_ref,
        &t.find(DDC117_REFERENCE, id_criteria).await.unwrap()
    ));

    let dql_ref = single_result(&t.em, &dql, &[s("Foo")]).await;

    t.em.contains(&dql_ref).unwrap();
}

#[tokio::test]
async fn testUpdateAssociationEntity() {
    let t = set_up().await;
    let id_criteria = t.reference_id();

    let map_ref = t.find(DDC117_REFERENCE, id_criteria.clone()).await;
    let map_ref = map_ref.expect("assertNotNull");
    set(&map_ref, "description", s("New Description!!"));
    t.em.flush().await.unwrap();
    t.em.clear().unwrap();

    let map_ref = t.find(DDC117_REFERENCE, id_criteria).await.unwrap();

    assert_eq!(s("New Description!!"), get(&map_ref, "description"));
}

#[tokio::test]
async fn testFetchDql() {
    let t = set_up().await;
    let dql = r"SELECT r, s FROM Doctrine\Tests\Models\DDC117\DDC117Reference r JOIN r.source s WHERE s.title = ?1";
    let query = t.em.create_query(dql);
    query.set_parameter(1, s("Foo"), None);
    let refs = list(&query.get_result(HYDRATE_OBJECT).await.unwrap());

    assert!(!refs.is_empty(), "Has to contain at least one Reference.");

    for r#ref in &refs {
        assert_value_instance_of(DDC117_REFERENCE, r#ref);
        assert!(
            t.em.contains(&obj(r#ref)).unwrap(),
            "Contains Reference in the IdentityMap."
        );
    }
}

#[tokio::test]
async fn testRemoveCompositeElement() {
    let t = set_up().await;
    let id_criteria = t.reference_id();

    let ref_rep = t.find(DDC117_REFERENCE, id_criteria.clone()).await.unwrap();

    t.em.remove(&ref_rep).await.unwrap();
    t.em.flush().await.unwrap();
    t.em.clear().unwrap();

    assert!(t.find(DDC117_REFERENCE, id_criteria).await.is_none());
}

#[tokio::test]
async fn testDqlRemoveCompositeElement() {
    let t = set_up().await;
    let id_criteria = t.reference_id();

    let dql = r"DELETE Doctrine\Tests\Models\DDC117\DDC117Reference r WHERE r.source = ?1 AND r.target = ?2";
    let query = t.em.create_query(dql);
    query.set_parameter(1, t.article1_id(), None);
    query.set_parameter(2, t.article2_id(), None);
    query.execute(None, None).await.unwrap();

    assert!(t.find(DDC117_REFERENCE, id_criteria).await.is_none());
}

#[tokio::test]
async fn testInverseSideAccess() {
    let t = set_up().await;
    let article1 = t.find(DDC117_ARTICLE, t.article1_id()).await.unwrap();

    assert_eq!(1, coll::count(&get(&article1, "references")).await.unwrap());

    for reference in coll::get_values(&get(&article1, "references"))
        .await
        .unwrap()
    {
        assert_value_instance_of(DDC117_REFERENCE, &reference);
        assert!(same_value(&get(&obj(&reference), "source"), &article1));
    }

    t.em.clear().unwrap();

    let dql = r"SELECT a, r FROM Doctrine\Tests\Models\DDC117\DDC117Article a INNER JOIN a.references r WHERE a.id = ?1";
    let _article_dql = single_result(&t.em, dql, &[get(&article1, "id")]).await;

    assert_eq!(1, coll::count(&get(&article1, "references")).await.unwrap());

    for reference in coll::get_values(&get(&article1, "references"))
        .await
        .unwrap()
    {
        assert_value_instance_of(DDC117_REFERENCE, &reference);
        assert!(same_value(&get(&obj(&reference), "source"), &article1));
    }
}

#[tokio::test]
async fn testMixedCompositeKey() {
    let t = set_up().await;
    let id_criteria = t.translation_id();

    let translation = t
        .find(DDC117_TRANSLATION, id_criteria.clone())
        .await
        .unwrap();
    assert_instance_of(DDC117_TRANSLATION, &translation);

    assert!(same(
        &translation,
        &t.find(DDC117_TRANSLATION, id_criteria).await.unwrap()
    ));

    t.em.clear().unwrap();

    let dql = r"SELECT t, a FROM Doctrine\Tests\Models\DDC117\DDC117Translation t JOIN t.article a WHERE t.article = ?1 AND t.language = ?2";
    let _dql_trans = single_result(&t.em, dql, &[t.article1_id(), s("en")]).await;

    assert_instance_of(DDC117_TRANSLATION, &translation);
}

#[tokio::test]
async fn testMixedCompositeKeyViolateUniqueness() {
    let t = set_up().await;
    let article1 = t.find(DDC117_ARTICLE, t.article1_id()).await.unwrap();
    add_translation(&article1, "en", "Bar");
    add_translation(&article1, "en", "Baz");

    // exception depending on the underlying Database Driver
    assert!(t.em.flush().await.is_err());
}

#[tokio::test]
async fn testOneToOneForeignObjectId() {
    let t = set_up().await;
    let article1 = article("Foo");
    t.em.persist(&article1).unwrap();
    t.em.flush().await.unwrap();

    let article_details = article_details(&article1, "Very long text");
    t.em.persist(&article_details).unwrap();
    t.em.flush().await.unwrap();

    set(&article_details, "text", s("not so very long text!"));
    t.em.flush().await.unwrap();
    t.em.clear().unwrap();

    let article = t
        .find(article1.class(), get(&article1, "id"))
        .await
        .unwrap();
    assert_instance_of(DDC117_ARTICLE, &article);
    // $article->getText(): $this->details->getText()
    let details = get_object(&article, "details").unwrap();
    t.em.initialize_object(&details).await.unwrap();
    assert_eq!(s("not so very long text!"), get(&details, "text"));
}

#[tokio::test]
async fn testOneToOneCascadeRemove() {
    let t = set_up().await;
    let article = t.find(t.article1.class(), t.article1_id()).await.unwrap();
    t.em.remove(&article).await.unwrap();
    t.em.flush().await.unwrap();

    assert!(
        !t.em
            .contains(&get_object(&article, "details").unwrap())
            .unwrap()
    );
}

#[tokio::test]
async fn testOneToOneCascadePersist() {
    let t = set_up().await;
    let article1 = article("Foo");
    let article_details = article_details(&article1, "Very long text");

    t.em.persist(&article1).unwrap();
    t.em.flush().await.unwrap();

    assert!(same(
        &article_details,
        &t.find(DDC117_ARTICLE_DETAILS, Value::Object(article1.clone()))
            .await
            .unwrap()
    ));
}

#[tokio::test]
async fn testReferencesToForeignKeyEntities() {
    let t = set_up().await;
    let reference = t.find(DDC117_REFERENCE, t.reference_id()).await.unwrap();

    let translation = t
        .find(DDC117_TRANSLATION, t.translation_id())
        .await
        .unwrap();

    let source = get_object(&reference, "source").unwrap();
    t.em.initialize_object(&source).await.unwrap();
    let approve_changes = approve_changes(
        &get_object(&source, "details").unwrap(),
        &reference,
        &translation,
    );
    t.em.persist(&approve_changes).unwrap();
    t.em.flush().await.unwrap();
    t.em.clear().unwrap();

    let approve_changes = t
        .find(DDC117_APPROVE_CHANGES, get(&approve_changes, "id"))
        .await
        .unwrap();

    assert_value_instance_of(
        DDC117_ARTICLE_DETAILS,
        &get(&approve_changes, "articleDetails"),
    );
    assert_value_instance_of(DDC117_REFERENCE, &get(&approve_changes, "reference"));
    assert_value_instance_of(DDC117_TRANSLATION, &get(&approve_changes, "translation"));
}

#[tokio::test]
async fn testLoadOneToManyCollectionOfForeignKeyEntities() {
    let t = set_up().await;
    let article = t.find(t.article1.class(), t.article1_id()).await.unwrap();
    assert_instance_of(DDC117_ARTICLE, &article);

    let translations = get(&article, "translations");
    assert!(!is_initialized(&translations));
    assert_contains_only(DDC117_TRANSLATION, &translations).await;
    assert!(is_initialized(&translations));
}

/// `loadEditorFixture()`.
async fn load_editor_fixture(t: &Fixture) -> Object {
    let editor = editor("beberlei");

    let article1 = t.find(t.article1.class(), t.article1_id()).await.unwrap();
    assert_instance_of(DDC117_ARTICLE, &article1);
    for translation in coll::get_values(&get(&article1, "translations"))
        .await
        .unwrap()
    {
        coll::add(&get(&editor, "reviewingTranslations"), translation).unwrap();
    }

    let article2 = t.find(t.article2.class(), t.article2_id()).await.unwrap();
    assert_instance_of(DDC117_ARTICLE, &article2);
    add_translation(&article2, "de", "Vanille-Krapferl"); // omnomnom
    add_translation(&article2, "fr", "Sorry can't speak french!");

    for translation in coll::get_values(&get(&article2, "translations"))
        .await
        .unwrap()
    {
        t.em.persist(&obj(&translation)).unwrap(); // otherwise persisting the editor won't work, reachability!
        coll::add(&get(&editor, "reviewingTranslations"), translation).unwrap();
    }

    t.em.persist(&editor).unwrap();
    t.em.flush().await.unwrap();
    t.em.clear().unwrap();

    t.find(editor.class(), get(&editor, "id")).await.unwrap()
}

#[tokio::test]
async fn testLoadManyToManyCollectionOfForeignKeyEntities() {
    let t = set_up().await;
    let editor = load_editor_fixture(&t).await;

    assert!(!is_initialized(&get(&editor, "reviewingTranslations")));
    assert_contains_only(DDC117_TRANSLATION, &get(&editor, "reviewingTranslations")).await;
    assert!(is_initialized(&get(&editor, "reviewingTranslations")));

    t.em.clear().unwrap();

    let dql = r"SELECT e, t FROM Doctrine\Tests\Models\DDC117\DDC117Editor e JOIN e.reviewingTranslations t WHERE e.id = ?1";
    let editor = single_result(&t.em, dql, &[get(&editor, "id")]).await;
    assert!(is_initialized(&get(&editor, "reviewingTranslations")));
    assert_contains_only(DDC117_TRANSLATION, &get(&editor, "reviewingTranslations")).await;
}

#[tokio::test]
async fn testClearManyToManyCollectionOfForeignKeyEntities() {
    let t = set_up().await;
    let editor = load_editor_fixture(&t).await;
    assert_eq!(
        3,
        coll::count(&get(&editor, "reviewingTranslations"))
            .await
            .unwrap()
    );

    coll::clear(&get(&editor, "reviewingTranslations"))
        .await
        .unwrap();
    t.em.flush().await.unwrap();
    t.em.clear().unwrap();

    let editor = t.find(editor.class(), get(&editor, "id")).await.unwrap();
    assert_eq!(
        0,
        coll::count(&get(&editor, "reviewingTranslations"))
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn testLoadInverseManyToManyCollection() {
    let t = set_up().await;
    let editor = load_editor_fixture(&t).await;

    let first = coll::get(&get(&editor, "reviewingTranslations"), 0)
        .await
        .unwrap();
    assert_value_instance_of(DDC117_TRANSLATION, &first);

    let reviewed_by = get(&obj(&first), "reviewedByEditors");
    assert_eq!(1, coll::count(&reviewed_by).await.unwrap());
    assert!(same_value(
        &coll::get(&reviewed_by, 0).await.unwrap(),
        &editor
    ));

    t.em.clear().unwrap();

    let dql = r"SELECT t, e FROM Doctrine\Tests\Models\DDC117\DDC117Translation t JOIN t.reviewedByEditors e WHERE t.article = ?1 AND t.language = ?2";
    // $this->translation->getArticleId(), getLanguage()
    let article_id = get(&get_object(&t.translation, "article").unwrap(), "id");
    let trans = single_result(&t.em, dql, &[article_id, get(&t.translation, "language")]).await;

    assert_instance_of(DDC117_TRANSLATION, &trans);
    assert_contains_only(DDC117_EDITOR, &get(&trans, "reviewedByEditors")).await;
    assert_eq!(
        1,
        coll::count(&get(&trans, "reviewedByEditors"))
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn testLoadOneToManyOfSourceEntityWithAssociationIdentifier() {
    let t = set_up().await;
    let editor = load_editor_fixture(&t).await;

    let first = coll::get(&get(&editor, "reviewingTranslations"), 0)
        .await
        .unwrap();
    add_last_translation(&editor, &obj(&first));
    t.em.flush().await.unwrap();
    t.em.clear().unwrap();

    let editor = t.find(editor.class(), get(&editor, "id")).await.unwrap();
    let first = coll::get(&get(&editor, "reviewingTranslations"), 0)
        .await
        .unwrap();
    let last_translated_by = get(&obj(&first), "lastTranslatedBy");
    coll::count(&last_translated_by).await.unwrap();

    assert_eq!(1, coll::count(&last_translated_by).await.unwrap());
}

#[tokio::test]
async fn testArrayHydrationWithCompositeKey() {
    let t = set_up().await;
    let dql = r"SELECT r,s,t FROM Doctrine\Tests\Models\DDC117\DDC117Reference r INNER JOIN r.source s INNER JOIN r.target t";
    let before = list(
        &t.em
            .create_query(dql)
            .get_result(HYDRATE_OBJECT)
            .await
            .unwrap(),
    )
    .len();

    let article1 = t.find(DDC117_ARTICLE, t.article1_id()).await.unwrap();
    let article2 = t.find(DDC117_ARTICLE, t.article2_id()).await.unwrap();

    let reference1 = reference(&article2, &article1, "Test-Description");
    t.em.persist(&reference1).unwrap();

    let reference2 = reference(&article1, &article1, "Test-Description");
    t.em.persist(&reference2).unwrap();

    let reference3 = reference(&article2, &article2, "Test-Description");
    t.em.persist(&reference3).unwrap();

    t.em.flush().await.unwrap();

    let data = list(&t.em.create_query(dql).get_array_result().await.unwrap());

    assert_eq!(before + 3, data.len());
}

#[tokio::test]
async fn testGetEntityState() {
    let t = set_up().await;
    let article1 = t.find(DDC117_ARTICLE, t.article1_id()).await.unwrap();
    let article2 = t.find(DDC117_ARTICLE, t.article2_id()).await.unwrap();

    let new_reference = reference(&article2, &article1, "Test-Description");

    assert_eq!(
        STATE_NEW,
        t.em.get_unit_of_work()
            .get_entity_state(&new_reference)
            .await
            .unwrap()
    );

    let reference = t.find(DDC117_REFERENCE, t.reference_id()).await.unwrap();

    assert_eq!(
        STATE_MANAGED,
        t.em.get_unit_of_work()
            .get_entity_state(&reference)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn testIndexByOnCompositeKeyField() {
    let t = set_up().await;
    let article = t.find(DDC117_ARTICLE, t.article1_id()).await.unwrap();

    assert_instance_of(DDC117_ARTICLE, &article);
    assert_eq!(1, coll::count(&get(&article, "links")).await.unwrap());
    let key = match t.article2_id() {
        Value::Int(id) => id,
        other => panic!("not an int: {other:?}"),
    };
    assert!(
        coll::contains_key(&get(&article, "links"), key)
            .await
            .unwrap()
    );
}
