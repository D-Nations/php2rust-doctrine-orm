//! The upstream test fixtures (`TestObject`, `OtherTestObject`,
//! `TestObjectProxy`, the mapping fixtures, `TestClassMetadata` and
//! `TestClassMetadataFactory`) as registered classes and Rust types, the
//! PHPUnit stubs and mocks the tests create (`StubClassMetadata`,
//! `MockMappingDriver`, `MockObjectManager`), and helpers for PHPUnit's
//! assertions.
//!
//! Upstream: doctrine/persistence tests/TestObject.php tests/OtherTestObject.php tests/TestObjectProxy.php tests/Mapping/TestClassMetadataFactory.php tests/Mapping/Fixtures/TestClassMetadata.php tests/Mapping/Fixtures/GlobalClass.php tests/Mapping/Fixtures/AnotherGlobalClass.php tests/Mapping/Fixtures/NotLoadedClass.php tests/Mapping/Fixtures/Model.php tests/Mapping/Fixtures/Manager/Manager.php

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::doc_markdown,
    clippy::disallowed_names,
    clippy::type_complexity,
    reason = "shared test helpers with upstream's names; a panic is a test failure"
)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Once};

use php_value::{Key, Object, PhpArray, PhpObject, Thrown, Value};
use php2rust_doctrine_collections::reflection::RawProperties;
use php2rust_doctrine_persistence::mapping::driver::MappingDriver;
use php2rust_doctrine_persistence::mapping::{
    AbstractClassMetadataFactory, ClassMetadata, ClassMetadataFactory, ClassMetadataFactoryState,
    ReflectionService, abstract_factory,
};
use php2rust_doctrine_persistence::php_reflection::{
    ClassInfo, MethodInfo, PropertyInfo, ReflectionClass, ReflectionObject, register_class,
    register_reflection_object,
};
use php2rust_doctrine_persistence::{ObjectManager, ObjectRepository, proxy};

pub const TEST_OBJECT: &str = "Doctrine\\Tests\\Persistence\\TestObject";
pub const OTHER_TEST_OBJECT: &str = "Doctrine\\Tests\\Persistence\\OtherTestObject";
pub const TEST_OBJECT_PROXY: &str = "Doctrine\\Tests\\Persistence\\TestObjectProxy";
pub const GLOBAL_CLASS: &str = "Doctrine\\Tests\\Persistence\\Mapping\\Fixtures\\GlobalClass";
pub const ANOTHER_GLOBAL_CLASS: &str =
    "Doctrine\\Tests\\Persistence\\Mapping\\Fixtures\\AnotherGlobalClass";
pub const NOT_LOADED_CLASS: &str =
    "Doctrine\\Tests\\Persistence\\Mapping\\Fixtures\\NotLoadedClass";
pub const MODEL: &str = "Doctrine\\Tests\\Persistence\\Mapping\\Fixtures\\Model";
pub const MANAGER: &str = "Doctrine\\Tests\\Persistence\\Mapping\\Fixtures\\Manager\\Manager";
pub const COLOCATED: &str = "Doctrine\\Tests\\Persistence\\Mapping\\_files\\colocated";
pub const ENTITY: &str = "Doctrine\\Tests\\Persistence\\Mapping\\_files\\colocated\\Entity";
pub const ENTITY_FIXTURE: &str =
    "Doctrine\\Tests\\Persistence\\Mapping\\_files\\colocated\\EntityFixture";
pub const TEST_CLASS: &str = "Doctrine\\Tests\\Persistence\\Mapping\\_files\\colocated\\TestClass";
pub const FOO: &str = "Doctrine\\Tests\\Persistence\\Mapping\\_files\\colocated\\Foo";

/// `tests/Mapping` (`__DIR__` of the mapping tests), where the fixture
/// files are.
pub fn mapping_dir() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("Mapping")
        .to_string_lossy()
        .into_owned()
}

/// `realpath()` of a fixture file under `tests/Mapping`.
fn fixture_file(relative: &str) -> PathBuf {
    let path = Path::new(&mapping_dir()).join(relative);
    let canonical = std::fs::canonicalize(&path).unwrap();
    let text = canonical.to_string_lossy().into_owned();
    match text.strip_prefix(r"\\?\") {
        Some(rest) => PathBuf::from(rest),
        None => canonical,
    }
}

/// Declares the fixture classes (once per test binary).
pub fn register_fixtures() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        register_class(
            ClassInfo::class(TEST_OBJECT)
                .property(
                    PropertyInfo::typed("id", "int", false)
                        .protected()
                        .default_value(Value::Int(1)),
                )
                .property(
                    PropertyInfo::typed("name", "string", false)
                        .protected()
                        .default_value(Value::from("beberlei")),
                )
                .property(PropertyInfo::typed("parent", TEST_OBJECT, false).protected())
                .property(PropertyInfo::typed("children", TEST_OBJECT, false).protected()),
        );
        register_class(
            ClassInfo::class(OTHER_TEST_OBJECT).property(
                PropertyInfo::typed("id", "int", false)
                    .protected()
                    .default_value(Value::Int(1)),
            ),
        );
        register_class(
            ClassInfo::class(TEST_OBJECT_PROXY)
                .extends(TEST_OBJECT)
                .implements(proxy::CLASS)
                .method(MethodInfo::new("__load"))
                .method(MethodInfo::new("__isInitialized")),
        );
        for name in [
            GLOBAL_CLASS,
            ANOTHER_GLOBAL_CLASS,
            NOT_LOADED_CLASS,
            MODEL,
            MANAGER,
        ] {
            register_class(ClassInfo::class(name));
        }
        register_class(
            ClassInfo::class(ENTITY)
                .file(fixture_file("_files/colocated/Entity.php"))
                .load_metadata(Arc::new(|metadata: &dyn ClassMetadata| {
                    metadata.get_field_names();
                    Ok(())
                })),
        );
        register_class(
            ClassInfo::class(ENTITY_FIXTURE)
                .file(fixture_file("_files/colocated/EntityFixture.php")),
        );
        register_class(
            ClassInfo::class(TEST_CLASS).file(fixture_file("_files/colocated/TestClass.php")),
        );
        register_class(ClassInfo::class(FOO).file(fixture_file("_files/colocated/Foo.mphp")));
        register_reflection_object::<FixtureObject>();
    });
}

/// `new TestObject()`.
pub fn new_test_object() -> Object {
    register_fixtures();
    Arc::new(FixtureObject::new(
        TEST_OBJECT,
        vec![
            FixtureProperty::typed("id", Some(Value::Int(1))).protected(),
            FixtureProperty::typed("name", Some(Value::from("beberlei"))).protected(),
            FixtureProperty::typed("parent", None).protected(),
            FixtureProperty::typed("children", None).protected(),
        ],
    ))
}

/// A property of a [`FixtureObject`].
#[derive(Debug, Clone)]
pub struct FixtureProperty {
    pub name: String,
    pub public: bool,
    pub typed: bool,
    /// `None` while uninitialized (or unset).
    pub value: Option<Value>,
}

impl FixtureProperty {
    pub fn typed(name: &str, value: Option<Value>) -> Self {
        Self {
            name: name.to_owned(),
            public: true,
            typed: true,
            value,
        }
    }

    pub fn untyped(name: &str, value: Value) -> Self {
        Self {
            name: name.to_owned(),
            public: true,
            typed: false,
            value: Some(value),
        }
    }

    pub fn protected(mut self) -> Self {
        self.public = false;
        self
    }

    pub fn private(self) -> Self {
        self.protected()
    }
}

/// An object of a fixture class: its properties in declaration order.
#[derive(Debug)]
pub struct FixtureObject {
    class: String,
    properties: Mutex<Vec<FixtureProperty>>,
}

impl FixtureObject {
    pub fn new(class: &str, properties: Vec<FixtureProperty>) -> Self {
        Self {
            class: class.to_owned(),
            properties: Mutex::new(properties),
        }
    }

    /// `$object->$name` from inside the class (`None` when uninitialized).
    pub fn get(&self, name: &str) -> Option<Value> {
        self.properties
            .lock()
            .unwrap()
            .iter()
            .find(|p| p.name == name)
            .and_then(|p| p.value.clone())
    }

    /// `$object->$name = $value`.
    pub fn set(&self, name: &str, value: Value) {
        let mut properties = self.properties.lock().unwrap();
        let property = properties.iter_mut().find(|p| p.name == name).unwrap();
        property.value = Some(value);
    }

    /// `unset($object->$name)`.
    pub fn unset(&self, name: &str) {
        let mut properties = self.properties.lock().unwrap();
        let property = properties.iter_mut().find(|p| p.name == name).unwrap();
        property.value = None;
    }
}

impl PhpObject for FixtureObject {
    fn class(&self) -> &str {
        &self.class
    }

    fn properties(&self) -> PhpArray {
        let mut array = PhpArray::new();
        for p in self.properties.lock().unwrap().iter() {
            if let (true, Some(value)) = (p.public, &p.value) {
                array.insert(Key::from(p.name.as_str()), value.clone());
            }
        }
        array
    }
}

impl RawProperties for FixtureObject {
    fn raw_property(&self, name: &str) -> Option<Result<Value, Thrown>> {
        let properties = self.properties.lock().unwrap();
        let p = properties.iter().find(|p| p.name == name)?;
        Some(match (&p.value, p.typed) {
            (Some(value), _) => Ok(value.clone()),
            (None, true) => Err(Thrown::new(
                "Error",
                format!(
                    "Typed property {}::${name} must not be accessed before initialization",
                    self.class
                ),
                0,
            )),
            (None, false) => Ok(Value::Null),
        })
    }
}

impl ReflectionObject for FixtureObject {
    fn set_raw_property(&self, name: &str, value: Value) -> Option<Result<(), Thrown>> {
        let mut properties = self.properties.lock().unwrap();
        let p = properties.iter_mut().find(|p| p.name == name)?;
        p.value = Some(value);
        Some(Ok(()))
    }

    fn unset_raw_property(&self, name: &str) -> Option<Result<(), Thrown>> {
        let mut properties = self.properties.lock().unwrap();
        let p = properties.iter_mut().find(|p| p.name == name)?;
        p.value = None;
        Some(Ok(()))
    }
}

/// `self::assertInstanceOf()` for objects: the object's class is or
/// extends `class`.
pub fn is_instance_of(object: &dyn PhpObject, class: &str) -> bool {
    php2rust_doctrine_persistence::php_reflection::is_a(object.class(), class)
}

/// `$this->expectException($class)` and `expectExceptionMessage()`
/// (which checks that the message contains the text).
#[track_caller]
pub fn assert_thrown<T: std::fmt::Debug>(result: Result<T, Thrown>, class: &str, message: &str) {
    let error = result.expect_err("an exception");
    assert!(
        php2rust_doctrine_persistence::php_reflection::thrown_is_a(&error, class),
        "expected {class}, got {error}"
    );
    let text = String::from_utf8_lossy(error.message.as_bytes()).into_owned();
    assert!(
        text.contains(message),
        "expected a message containing {message:?}, got {text:?}"
    );
}

/// `self::createStub(ClassMetadata::class)`, with `getName()` configured
/// (PHPUnit's stubs return `''` for a string by default).
#[derive(Debug, Default)]
pub struct StubClassMetadata {
    pub name: String,
    /// Calls of `getFieldNames()` (a mock's expectation).
    pub get_field_names_calls: AtomicUsize,
}

impl StubClassMetadata {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn named(name: &str) -> Arc<Self> {
        Arc::new(Self {
            name: name.to_owned(),
            ..Self::default()
        })
    }
}

impl PartialEq for StubClassMetadata {
    /// `assertEquals()`: the same configured values.
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl ClassMetadata for StubClassMetadata {
    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_identifier(&self) -> Vec<String> {
        Vec::new()
    }

    fn get_reflection_class(&self) -> Result<ReflectionClass, Thrown> {
        ReflectionClass::new(&self.name)
    }

    fn is_identifier(&self, _field_name: &str) -> bool {
        false
    }

    fn has_field(&self, _field_name: &str) -> bool {
        false
    }

    fn has_association(&self, _field_name: &str) -> bool {
        false
    }

    fn is_single_valued_association(&self, _field_name: &str) -> bool {
        false
    }

    fn is_collection_valued_association(&self, _field_name: &str) -> bool {
        false
    }

    fn get_field_names(&self) -> Vec<String> {
        self.get_field_names_calls.fetch_add(1, Ordering::SeqCst);
        Vec::new()
    }

    fn get_identifier_field_names(&self) -> Vec<String> {
        Vec::new()
    }

    fn get_association_names(&self) -> Vec<String> {
        Vec::new()
    }

    fn get_type_of_field(&self, _field_name: &str) -> Result<Option<String>, Thrown> {
        Ok(None)
    }

    fn get_association_target_class(&self, _assoc_name: &str) -> Result<Option<String>, Thrown> {
        Ok(None)
    }

    fn is_association_inverse_side(&self, _assoc_name: &str) -> bool {
        false
    }

    fn get_association_mapped_by_target_field(&self, _assoc_name: &str) -> Result<String, Thrown> {
        Ok(String::new())
    }

    fn get_identifier_values(&self, _object: &Object) -> Result<PhpArray, Thrown> {
        Ok(PhpArray::new())
    }
}

/// `Doctrine\Tests\Persistence\Mapping\Fixtures\TestClassMetadata`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestClassMetadata {
    class_name: String,
}

impl TestClassMetadata {
    pub fn new(class_name: &str) -> Self {
        Self {
            class_name: class_name.to_owned(),
        }
    }
}

fn not_implemented() -> Thrown {
    Thrown::new("LogicException", "Not implemented", 0)
}

impl ClassMetadata for TestClassMetadata {
    fn get_name(&self) -> String {
        self.class_name.clone()
    }

    fn get_identifier(&self) -> Vec<String> {
        vec!["id".to_owned()]
    }

    fn get_reflection_class(&self) -> Result<ReflectionClass, Thrown> {
        ReflectionClass::new(&self.get_name())
    }

    fn is_identifier(&self, _field_name: &str) -> bool {
        false
    }

    fn has_field(&self, _field_name: &str) -> bool {
        false
    }

    fn has_association(&self, _field_name: &str) -> bool {
        false
    }

    fn is_single_valued_association(&self, _field_name: &str) -> bool {
        false
    }

    fn is_collection_valued_association(&self, _field_name: &str) -> bool {
        false
    }

    fn get_field_names(&self) -> Vec<String> {
        Vec::new()
    }

    fn get_identifier_field_names(&self) -> Vec<String> {
        Vec::new()
    }

    fn get_association_names(&self) -> Vec<String> {
        Vec::new()
    }

    fn get_type_of_field(&self, _field_name: &str) -> Result<Option<String>, Thrown> {
        Err(not_implemented())
    }

    fn get_association_target_class(&self, _assoc_name: &str) -> Result<Option<String>, Thrown> {
        Err(not_implemented())
    }

    fn is_association_inverse_side(&self, _assoc_name: &str) -> bool {
        false
    }

    fn get_association_mapped_by_target_field(&self, _assoc_name: &str) -> Result<String, Thrown> {
        Err(not_implemented())
    }

    fn get_identifier_values(&self, _object: &Object) -> Result<PhpArray, Thrown> {
        Ok(PhpArray::new())
    }
}

/// `$this->createMock(MappingDriver::class)` / `createStub()`: answers
/// from the configured closures (a stub's defaults otherwise: `false`,
/// `[]`, nothing) and counts the calls.
#[derive(Default)]
pub struct MockMappingDriver {
    pub is_transient: Option<Box<dyn Fn(&str) -> bool + Send + Sync>>,
    pub all_class_names: Vec<String>,
    pub is_transient_calls: Mutex<Vec<String>>,
    pub load_metadata_calls: Mutex<Vec<String>>,
    pub get_all_class_names_calls: AtomicUsize,
}

impl MockMappingDriver {
    pub fn stub() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn is_transient_calls(&self) -> Vec<String> {
        self.is_transient_calls.lock().unwrap().clone()
    }

    pub fn load_metadata_calls(&self) -> Vec<String> {
        self.load_metadata_calls.lock().unwrap().clone()
    }
}

impl MappingDriver for MockMappingDriver {
    fn load_metadata_for_class(
        &self,
        class_name: &str,
        _metadata: &dyn ClassMetadata,
    ) -> Result<(), Thrown> {
        self.load_metadata_calls
            .lock()
            .unwrap()
            .push(class_name.to_owned());
        Ok(())
    }

    fn get_all_class_names(&self) -> Result<Vec<String>, Thrown> {
        self.get_all_class_names_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(self.all_class_names.clone())
    }

    fn is_transient(&self, class_name: &str) -> Result<bool, Thrown> {
        self.is_transient_calls
            .lock()
            .unwrap()
            .push(class_name.to_owned());
        Ok(self.is_transient.as_ref().is_some_and(|f| f(class_name)))
    }
}

/// `Doctrine\Tests\Persistence\Mapping\TestClassMetadataFactory`.
pub struct TestClassMetadataFactory<M: ClassMetadata> {
    state: ClassMetadataFactoryState<M>,
    pub fallback_callback: Mutex<Option<Box<dyn Fn() -> Option<Arc<M>> + Send + Sync>>>,
    pub driver: Arc<dyn MappingDriver>,
    pub metadata: Arc<M>,
}

impl<M: ClassMetadata> TestClassMetadataFactory<M> {
    pub fn new(driver: Arc<dyn MappingDriver>, metadata: Arc<M>) -> Self {
        Self {
            state: ClassMetadataFactoryState::new(),
            fallback_callback: Mutex::new(None),
            driver,
            metadata,
        }
    }

    pub fn set_fallback_callback(
        &self,
        callback: impl Fn() -> Option<Arc<M>> + Send + Sync + 'static,
    ) {
        *self.fallback_callback.lock().unwrap() = Some(Box::new(callback));
    }
}

impl<M: ClassMetadata> AbstractClassMetadataFactory for TestClassMetadataFactory<M> {
    type Metadata = M;

    fn factory_state(&self) -> &ClassMetadataFactoryState<M> {
        &self.state
    }

    fn do_load_metadata(
        &self,
        _class: &Arc<M>,
        _parent: Option<&Arc<M>>,
        _root_entity_found: bool,
        _non_superclass_parents: &[String],
    ) -> Result<(), Thrown> {
        Ok(())
    }

    fn initialize(&self) -> Result<(), Thrown> {
        Ok(())
    }

    fn new_class_metadata_instance(&self, _class_name: &str) -> Result<Arc<M>, Thrown> {
        Ok(Arc::clone(&self.metadata))
    }

    fn get_driver(&self) -> Result<Arc<dyn MappingDriver>, Thrown> {
        Ok(Arc::clone(&self.driver))
    }

    fn wakeup_reflection(
        &self,
        _class: &Arc<M>,
        _refl: &dyn ReflectionService,
    ) -> Result<(), Thrown> {
        Ok(())
    }

    fn initialize_reflection(
        &self,
        _class: &Arc<M>,
        _refl: &dyn ReflectionService,
    ) -> Result<(), Thrown> {
        Ok(())
    }

    fn is_entity(&self, _class: &M) -> bool {
        true
    }

    fn on_not_found_metadata(&self, _class_name: &str) -> Result<Option<Arc<M>>, Thrown> {
        Ok(self
            .fallback_callback
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|callback| callback()))
    }

    fn is_transient(&self, class_name: &str) -> Result<bool, Thrown> {
        Ok(class_name != self.metadata.get_name())
    }

    fn get_cache_key(&self, real_class_name: &str) -> String {
        abstract_factory::get_cache_key(self, real_class_name)
    }
}

/// A method call a [`MockObjectManager`] got: the method and its
/// arguments.
#[derive(Debug, Clone)]
pub struct Call {
    pub method: &'static str,
    pub class_name: Option<String>,
    pub arguments: Vec<Value>,
}

/// `$this->createMock(ObjectManager::class)`: records the calls and
/// answers with the configured values.
#[derive(Default)]
pub struct MockObjectManager {
    pub calls: Mutex<Vec<Call>>,
    pub find_result: Mutex<Option<Object>>,
    pub repository: Mutex<Option<Arc<dyn ObjectRepository>>>,
    pub class_metadata: Mutex<Option<Arc<dyn ClassMetadata>>>,
    pub metadata_factory: Mutex<Option<Arc<dyn ClassMetadataFactory>>>,
    pub contains_result: Mutex<bool>,
    pub is_uninitialized_result: Mutex<bool>,
}

impl MockObjectManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn record(&self, method: &'static str, class_name: Option<&str>, arguments: Vec<Value>) {
        self.calls.lock().unwrap().push(Call {
            method,
            class_name: class_name.map(str::to_owned),
            arguments,
        });
    }

    /// The calls of `method`.
    pub fn calls_of(&self, method: &str) -> Vec<Call> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|c| c.method == method)
            .cloned()
            .collect()
    }
}

impl ObjectManager for MockObjectManager {
    fn find(&self, class_name: &str, id: &Value) -> Result<Option<Object>, Thrown> {
        self.record("find", Some(class_name), vec![id.clone()]);
        Ok(self.find_result.lock().unwrap().clone())
    }

    fn persist(&self, object: &Object) -> Result<(), Thrown> {
        self.record("persist", None, vec![Value::Object(Arc::clone(object))]);
        Ok(())
    }

    fn remove(&self, object: &Object) -> Result<(), Thrown> {
        self.record("remove", None, vec![Value::Object(Arc::clone(object))]);
        Ok(())
    }

    fn clear(&self) -> Result<(), Thrown> {
        self.record("clear", None, Vec::new());
        Ok(())
    }

    fn detach(&self, object: &Object) -> Result<(), Thrown> {
        self.record("detach", None, vec![Value::Object(Arc::clone(object))]);
        Ok(())
    }

    fn refresh(&self, object: &Object) -> Result<(), Thrown> {
        self.record("refresh", None, vec![Value::Object(Arc::clone(object))]);
        Ok(())
    }

    fn flush(&self) -> Result<(), Thrown> {
        self.record("flush", None, Vec::new());
        Ok(())
    }

    fn get_repository(&self, class_name: &str) -> Result<Arc<dyn ObjectRepository>, Thrown> {
        self.record("getRepository", Some(class_name), Vec::new());
        Ok(self
            .repository
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| StubObjectRepository::shared()))
    }

    fn get_class_metadata(&self, class_name: &str) -> Result<Arc<dyn ClassMetadata>, Thrown> {
        self.record("getClassMetadata", Some(class_name), Vec::new());
        Ok(self
            .class_metadata
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| StubClassMetadata::new()))
    }

    fn get_metadata_factory(&self) -> Arc<dyn ClassMetadataFactory> {
        self.record("getMetadataFactory", None, Vec::new());
        self.metadata_factory
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| {
                Arc::new(TestClassMetadataFactory::new(
                    MockMappingDriver::stub(),
                    StubClassMetadata::new(),
                ))
            })
    }

    fn initialize_object(&self, obj: &Object) -> Result<(), Thrown> {
        self.record(
            "initializeObject",
            None,
            vec![Value::Object(Arc::clone(obj))],
        );
        Ok(())
    }

    fn is_uninitialized_object(&self, value: &Value) -> bool {
        self.record("isUninitializedObject", None, vec![value.clone()]);
        *self.is_uninitialized_result.lock().unwrap()
    }

    fn contains(&self, object: &Object) -> bool {
        self.record("contains", None, vec![Value::Object(Arc::clone(object))]);
        *self.contains_result.lock().unwrap()
    }
}

/// `self::createStub(ObjectRepository::class)`.
#[derive(Debug, Default)]
pub struct StubObjectRepository;

impl StubObjectRepository {
    pub fn shared() -> Arc<dyn ObjectRepository> {
        Arc::new(Self)
    }
}

impl ObjectRepository for StubObjectRepository {
    fn find(&self, _id: &Value) -> Result<Option<Object>, Thrown> {
        Ok(None)
    }

    fn find_all(&self) -> Result<Vec<Object>, Thrown> {
        Ok(Vec::new())
    }

    fn find_by(
        &self,
        _criteria: &PhpArray,
        _order_by: Option<&PhpArray>,
        _limit: Option<i64>,
        _offset: Option<i64>,
    ) -> Result<Vec<Object>, Thrown> {
        Ok(Vec::new())
    }

    fn find_one_by(&self, _criteria: &PhpArray) -> Result<Option<Object>, Thrown> {
        Ok(None)
    }

    fn get_class_name(&self) -> String {
        String::new()
    }
}

/// `$a === $b` for shared objects.
pub fn same<T: ?Sized>(a: &Arc<T>, b: &Arc<T>) -> bool {
    std::ptr::addr_eq(Arc::as_ptr(a), Arc::as_ptr(b))
}
