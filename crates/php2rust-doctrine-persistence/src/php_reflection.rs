//! The part of PHP's class table and Reflection API that doctrine/persistence
//! uses: `class_exists()`, `class_parents()`, `method_exists()`,
//! `ReflectionClass` (names, parents, interfaces, anonymous classes,
//! default properties, methods), `ReflectionProperty` (visibility, type,
//! default value, doc comment, attributes, reading and writing a value of
//! any visibility) and the enum cases `BackedEnum::from()` looks up.
//!
//! PHP knows a class once its file is loaded; a Rust program declares the
//! classes it maps to PHP once, with [`register_class`] and a
//! [`ClassInfo`]. A class that is not registered does not exist here
//! (`class_exists()` is false, `new ReflectionClass()` throws). `stdClass`,
//! the exception classes persistence throws and persistence's own
//! interfaces are registered from the start.
//!
//! Values are read and written on objects (`php_value::PhpObject`), which
//! list their public properties only. A class whose non-public properties
//! reflection reads or writes implements [`ReflectionObject`] and
//! registers it, once, with [`register_reflection_object`]; that also
//! registers it with collections' `reflection::register_raw_properties()`,
//! so `Criteria` matching reads the same properties. Properties are found
//! by name: a private property that shadows a parent's private property of
//! the same name is the object's business.
//!
//! Upstream: none

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, PoisonError, RwLock};

use php_value::object::{GenericObject, StdClass};
use php_value::{Key, Object, PhpArray, PhpObject, Thrown, Value};
use php2rust_doctrine_collections::reflection::{RawProperties, register_raw_properties};

use crate::mapping::ClassMetadata;
use crate::proxy::Proxy;

/// `ReflectionProperty::IS_PUBLIC`.
pub const IS_PUBLIC: i64 = 1;
/// `ReflectionProperty::IS_PROTECTED`.
pub const IS_PROTECTED: i64 = 2;
/// `ReflectionProperty::IS_PRIVATE`.
pub const IS_PRIVATE: i64 = 4;
/// `ReflectionProperty::IS_STATIC`.
pub const IS_STATIC: i64 = 16;
/// `ReflectionProperty::IS_READONLY`.
pub const IS_READONLY: i64 = 128;

/// A member's visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    #[default]
    Public,
    Protected,
    Private,
}

/// What a declared name is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClassKind {
    #[default]
    Class,
    Interface,
    Trait,
    Enum,
}

/// A property's declared type, as `ReflectionNamedType` gives it: the
/// type's name (`Suit` for `Suit|null`, `array` for `?array`) and whether
/// it allows `null`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyType {
    name: String,
    allows_null: bool,
}

impl PropertyType {
    pub fn new(name: impl Into<String>, allows_null: bool) -> Self {
        Self {
            name: name.into(),
            allows_null,
        }
    }

    /// `ReflectionNamedType::getName()`.
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// `ReflectionType::allowsNull()`.
    pub fn allows_null(&self) -> bool {
        self.allows_null
    }
}

/// A declared property.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyInfo {
    name: String,
    visibility: Visibility,
    is_static: bool,
    is_readonly: bool,
    ty: Option<PropertyType>,
    default: Option<Value>,
    doc_comment: Option<String>,
    attributes: Vec<String>,
}

impl PropertyInfo {
    /// `public $name;`: an untyped public property, whose default is
    /// `null` as PHP gives untyped properties.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            visibility: Visibility::Public,
            is_static: false,
            is_readonly: false,
            ty: None,
            default: Some(Value::Null),
            doc_comment: None,
            attributes: Vec::new(),
        }
    }

    /// `public Type $name;`: a typed property without a default value
    /// (until [`PropertyInfo::default_value`]).
    pub fn typed(name: impl Into<String>, ty: &str, allows_null: bool) -> Self {
        Self {
            ty: Some(PropertyType::new(ty, allows_null)),
            default: None,
            ..Self::new(name)
        }
    }

    #[must_use]
    pub fn visibility(mut self, visibility: Visibility) -> Self {
        self.visibility = visibility;
        self
    }

    #[must_use]
    pub fn protected(self) -> Self {
        self.visibility(Visibility::Protected)
    }

    #[must_use]
    pub fn private(self) -> Self {
        self.visibility(Visibility::Private)
    }

    #[must_use]
    pub fn static_property(mut self) -> Self {
        self.is_static = true;
        self
    }

    #[must_use]
    pub fn readonly(mut self) -> Self {
        self.is_readonly = true;
        // A readonly property has no default value.
        self.default = None;
        self
    }

    /// The declared default value (`= $value`).
    #[must_use]
    pub fn default_value(mut self, value: Value) -> Self {
        self.default = Some(value);
        self
    }

    /// No default value: an untyped property declared without one still
    /// defaults to `null`, so this is for typed properties.
    #[must_use]
    pub fn no_default(mut self) -> Self {
        self.default = None;
        self
    }

    #[must_use]
    pub fn doc_comment(mut self, comment: impl Into<String>) -> Self {
        self.doc_comment = Some(comment.into());
        self
    }

    /// An attribute (`#[Name]`) on the property, by class name.
    #[must_use]
    pub fn attribute(mut self, class: impl Into<String>) -> Self {
        self.attributes.push(class.into());
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// `Class::loadMetadata(ClassMetadata $metadata)`, the static method
/// `StaticPHPDriver` calls.
pub type LoadMetadataFn = Arc<dyn Fn(&dyn ClassMetadata) -> Result<(), Thrown> + Send + Sync>;

/// Makes a blank instance of a class without running its constructor:
/// what `ReflectionClass::newInstanceWithoutConstructor()` gives (and,
/// for an internal final class, what `unserialize()` of an empty payload
/// gives, or what it throws).
pub type NewInstanceFn = Arc<dyn Fn() -> Result<Object, Thrown> + Send + Sync>;

/// A declared method.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodInfo {
    name: String,
    visibility: Visibility,
    is_static: bool,
    required_parameters: usize,
}

impl MethodInfo {
    /// A public method.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            visibility: Visibility::Public,
            is_static: false,
            required_parameters: 0,
        }
    }

    /// Sets `ReflectionMethod::getNumberOfRequiredParameters()`.
    #[must_use]
    pub fn required_parameters(mut self, count: usize) -> Self {
        self.required_parameters = count;
        self
    }

    /// `ReflectionMethod::getNumberOfRequiredParameters()`.
    pub fn get_number_of_required_parameters(&self) -> usize {
        self.required_parameters
    }

    #[must_use]
    pub fn visibility(mut self, visibility: Visibility) -> Self {
        self.visibility = visibility;
        self
    }

    #[must_use]
    pub fn static_method(mut self) -> Self {
        self.is_static = true;
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// `ReflectionMethod::isPublic()`.
    pub fn is_public(&self) -> bool {
        self.visibility == Visibility::Public
    }

    /// `ReflectionMethod::isStatic()`.
    pub fn is_static(&self) -> bool {
        self.is_static
    }
}

/// A declared class, interface, trait or enum.
#[derive(Clone)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "the flags PHP keeps on a class entry"
)]
pub struct ClassInfo {
    name: String,
    kind: ClassKind,
    parent: Option<String>,
    interfaces: Vec<String>,
    anonymous: bool,
    file_name: Option<PathBuf>,
    properties: Vec<PropertyInfo>,
    methods: Vec<MethodInfo>,
    cases: Vec<Object>,
    load_metadata: Option<LoadMetadataFn>,
    is_abstract: bool,
    is_final: bool,
    is_internal: bool,
    new_instance: Option<NewInstanceFn>,
}

impl fmt::Debug for ClassInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClassInfo")
            .field("name", &self.name)
            .field("kind", &self.kind)
            .field("parent", &self.parent)
            .field("interfaces", &self.interfaces)
            .field("anonymous", &self.anonymous)
            .field("file_name", &self.file_name)
            .field("properties", &self.properties)
            .field("methods", &self.methods)
            .field("cases", &self.cases)
            .field("load_metadata", &self.load_metadata.is_some())
            .field("is_abstract", &self.is_abstract)
            .field("is_final", &self.is_final)
            .field("is_internal", &self.is_internal)
            .field("new_instance", &self.new_instance.is_some())
            .finish()
    }
}

impl ClassInfo {
    /// `class Name {}`.
    pub fn class(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: ClassKind::Class,
            parent: None,
            interfaces: Vec::new(),
            anonymous: false,
            file_name: None,
            properties: Vec::new(),
            methods: Vec::new(),
            cases: Vec::new(),
            load_metadata: None,
            is_abstract: false,
            is_final: false,
            is_internal: false,
            new_instance: None,
        }
    }

    /// `interface Name {}` (its parent interfaces through
    /// [`ClassInfo::implements`]).
    pub fn interface(name: impl Into<String>) -> Self {
        Self {
            kind: ClassKind::Interface,
            ..Self::class(name)
        }
    }

    /// `trait Name {}`.
    pub fn trait_(name: impl Into<String>) -> Self {
        Self {
            kind: ClassKind::Trait,
            ..Self::class(name)
        }
    }

    /// `enum Name: type { case ... }`, with its cases
    /// (`php_value::object::EnumCase` objects, in declaration order).
    pub fn enumeration(name: impl Into<String>, cases: Vec<Object>) -> Self {
        Self {
            kind: ClassKind::Enum,
            cases,
            interfaces: vec!["UnitEnum".to_owned(), "BackedEnum".to_owned()],
            ..Self::class(name)
        }
    }

    /// `new class {}`: an anonymous class, whose generated name
    /// (`class@anonymous` then a NUL byte, the file, line and a counter)
    /// is `name`.
    pub fn anonymous(name: impl Into<String>) -> Self {
        Self {
            anonymous: true,
            ..Self::class(name)
        }
    }

    #[must_use]
    pub fn extends(mut self, parent: impl Into<String>) -> Self {
        self.parent = Some(parent.into());
        self
    }

    #[must_use]
    pub fn implements(mut self, interface: impl Into<String>) -> Self {
        self.interfaces.push(interface.into());
        self
    }

    /// The file that declares the class (`ReflectionClass::getFileName()`),
    /// as `realpath()` gives it.
    #[must_use]
    pub fn file(mut self, file_name: impl Into<PathBuf>) -> Self {
        self.file_name = Some(file_name.into());
        self
    }

    #[must_use]
    pub fn property(mut self, property: PropertyInfo) -> Self {
        self.properties.push(property);
        self
    }

    #[must_use]
    pub fn method(mut self, method: MethodInfo) -> Self {
        self.methods.push(method);
        self
    }

    /// `public static function loadMetadata(ClassMetadata $metadata)`.
    #[must_use]
    pub fn load_metadata(mut self, load: LoadMetadataFn) -> Self {
        self.methods
            .push(MethodInfo::new("loadMetadata").static_method());
        self.load_metadata = Some(load);
        self
    }

    /// `abstract class Name {}`.
    #[must_use]
    pub fn abstract_class(mut self) -> Self {
        self.is_abstract = true;
        self
    }

    /// `final class Name {}`.
    #[must_use]
    pub fn final_class(mut self) -> Self {
        self.is_final = true;
        self
    }

    /// A class PHP itself (or an extension) declares
    /// (`ReflectionClass::isInternal()`).
    #[must_use]
    pub fn internal(mut self) -> Self {
        self.is_internal = true;
        self
    }

    /// How to make a blank instance without the constructor (see
    /// [`NewInstanceFn`]). Without one, a blank instance is a
    /// `php_value::object::GenericObject` with the public non-static
    /// default properties (`stdClass` gives a `StdClass`).
    #[must_use]
    pub fn new_instance(mut self, new_instance: NewInstanceFn) -> Self {
        self.new_instance = Some(new_instance);
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_abstract(&self) -> bool {
        self.is_abstract
    }

    pub fn is_final(&self) -> bool {
        self.is_final
    }

    pub fn is_internal(&self) -> bool {
        self.is_internal
    }

    pub fn kind(&self) -> ClassKind {
        self.kind
    }

    pub fn parent(&self) -> Option<&str> {
        self.parent.as_deref()
    }

    pub fn is_anonymous(&self) -> bool {
        self.anonymous
    }

    pub fn file_name(&self) -> Option<&PathBuf> {
        self.file_name.as_ref()
    }

    pub fn properties(&self) -> &[PropertyInfo] {
        &self.properties
    }

    pub fn methods(&self) -> &[MethodInfo] {
        &self.methods
    }

    /// The enum's cases.
    pub fn cases(&self) -> &[Object] {
        &self.cases
    }
}

struct ClassTable {
    /// Lowercased name → class.
    by_name: HashMap<String, Arc<ClassInfo>>,
    /// Declaration order (`get_declared_classes()`).
    order: Vec<Arc<ClassInfo>>,
}

impl ClassTable {
    fn add(&mut self, info: ClassInfo) -> bool {
        let key = info.name.to_ascii_lowercase();
        if self.by_name.contains_key(&key) {
            return false;
        }
        let info = Arc::new(info);
        self.by_name.insert(key, Arc::clone(&info));
        self.order.push(info);
        true
    }
}

const PERSISTENCE: &str = "Doctrine\\Persistence\\";

static CLASSES: LazyLock<RwLock<ClassTable>> = LazyLock::new(|| {
    let mut table = ClassTable {
        by_name: HashMap::new(),
        order: Vec::new(),
    };
    for info in [
        ClassInfo::class("stdClass").internal(),
        ClassInfo::interface("Throwable")
            .internal()
            .implements("Stringable"),
        ClassInfo::interface("Stringable").internal(),
        ClassInfo::interface("UnitEnum").internal(),
        ClassInfo::interface("BackedEnum")
            .internal()
            .implements("UnitEnum"),
        ClassInfo::class("Exception")
            .internal()
            .implements("Throwable"),
        ClassInfo::class("Error").internal().implements("Throwable"),
        ClassInfo::class("TypeError").internal().extends("Error"),
        ClassInfo::class("ValueError").internal().extends("Error"),
        ClassInfo::class("AssertionError")
            .internal()
            .extends("Error"),
        ClassInfo::class("ErrorException")
            .internal()
            .extends("Exception"),
        ClassInfo::class("LogicException")
            .internal()
            .extends("Exception"),
        ClassInfo::class("InvalidArgumentException")
            .internal()
            .extends("LogicException"),
        ClassInfo::class("RuntimeException")
            .internal()
            .extends("Exception"),
        ClassInfo::class("UnexpectedValueException")
            .internal()
            .extends("RuntimeException"),
        ClassInfo::class("ReflectionException")
            .internal()
            .extends("Exception"),
        ClassInfo::class(format!("{PERSISTENCE}Mapping\\MappingException")).extends("Exception"),
        ClassInfo::interface(format!("{PERSISTENCE}Proxy")),
        ClassInfo::interface(format!("{PERSISTENCE}ObjectManager")),
        ClassInfo::interface(format!("{PERSISTENCE}ObjectRepository")),
        ClassInfo::interface(format!("{PERSISTENCE}ConnectionRegistry")),
        ClassInfo::interface(format!("{PERSISTENCE}ManagerRegistry"))
            .implements(format!("{PERSISTENCE}ConnectionRegistry")),
        ClassInfo::interface(format!("{PERSISTENCE}NotifyPropertyChanged")),
        ClassInfo::interface(format!("{PERSISTENCE}PropertyChangedListener")),
        ClassInfo::interface(format!("{PERSISTENCE}Mapping\\ClassMetadata")),
        ClassInfo::interface(format!("{PERSISTENCE}Mapping\\ClassMetadataFactory")),
        ClassInfo::interface(format!("{PERSISTENCE}Mapping\\ReflectionService")),
        ClassInfo::interface(format!("{PERSISTENCE}Mapping\\ProxyClassNameResolver")),
        ClassInfo::interface(format!("{PERSISTENCE}Mapping\\Driver\\MappingDriver")),
        ClassInfo::interface(format!("{PERSISTENCE}Mapping\\Driver\\FileLocator")),
        ClassInfo::interface(format!("{PERSISTENCE}Mapping\\Driver\\ClassLocator")),
        ClassInfo::class(format!("{PERSISTENCE}Mapping\\RuntimeReflectionService"))
            .implements(format!("{PERSISTENCE}Mapping\\ReflectionService")),
    ] {
        table.add(info);
    }
    RwLock::new(table)
});

/// Declares a class. A class that exists already stays as it is (PHP
/// cannot redeclare one); the result says whether this declared it.
pub fn register_class(info: ClassInfo) -> bool {
    CLASSES
        .write()
        .unwrap_or_else(PoisonError::into_inner)
        .add(info)
}

/// The declared class, interface, trait or enum `name` (without case, as
/// PHP looks names up; a leading `\` is not part of a name).
pub fn class_info(name: &str) -> Option<Arc<ClassInfo>> {
    CLASSES
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .by_name
        .get(&name.to_ascii_lowercase())
        .cloned()
}

/// The declared names, in declaration order (`get_declared_classes()`
/// with the interfaces, traits and enums).
pub fn declared() -> Vec<Arc<ClassInfo>> {
    CLASSES
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .order
        .clone()
}

/// `trait_exists($name)`.
pub fn trait_exists(name: &str) -> bool {
    class_info(name).is_some_and(|c| c.kind == ClassKind::Trait)
}

/// `enum_exists($name)`.
pub fn enum_exists(name: &str) -> bool {
    class_info(name).is_some_and(|c| c.kind == ClassKind::Enum)
}

/// `class_exists($name)`: a class or an enum (not an interface or trait).
pub fn class_exists(name: &str) -> bool {
    class_info(name).is_some_and(|c| matches!(c.kind, ClassKind::Class | ClassKind::Enum))
}

/// `interface_exists($name)`.
pub fn interface_exists(name: &str) -> bool {
    class_info(name).is_some_and(|c| c.kind == ClassKind::Interface)
}

/// `class_parents($name)`: the parent classes, nearest first; `None` when
/// the class does not exist.
pub fn class_parents(name: &str) -> Option<Vec<String>> {
    let mut info = class_info(name)?;
    let mut parents = Vec::new();
    while let Some(parent) = info.parent.as_deref().and_then(class_info) {
        if parents
            .iter()
            .any(|p: &String| p.eq_ignore_ascii_case(&parent.name))
        {
            break;
        }
        parents.push(parent.name.clone());
        info = parent;
    }
    Some(parents)
}

/// `is_a($class, $parent, true)` on names: the class itself, a parent
/// class or an implemented interface (of the class or a parent, and the
/// interfaces those extend).
pub fn is_a(class: &str, parent: &str) -> bool {
    let mut pending = vec![class.to_owned()];
    let mut seen: Vec<String> = Vec::new();
    while let Some(name) = pending.pop() {
        if name.eq_ignore_ascii_case(parent) {
            return true;
        }
        if seen.iter().any(|s| s.eq_ignore_ascii_case(&name)) {
            continue;
        }
        seen.push(name.clone());
        if let Some(info) = class_info(&name) {
            pending.extend(info.parent.iter().cloned());
            pending.extend(info.interfaces.iter().cloned());
        }
    }
    false
}

/// `$thrown instanceof $class`: the thrown class or one of its parents.
pub fn thrown_is_a(thrown: &Thrown, class: &str) -> bool {
    is_a(&thrown.class, class)
}

/// `method_exists($class, $method)`, through the parent classes.
pub fn method_exists(class: &str, method: &str) -> bool {
    find_method(class, method).is_some()
}

fn find_method(class: &str, method: &str) -> Option<MethodInfo> {
    let mut current = class_info(class);
    let mut depth = 0;
    while let Some(info) = current {
        if let Some(m) = info
            .methods
            .iter()
            .find(|m| m.name.eq_ignore_ascii_case(method))
        {
            return Some(m.clone());
        }
        depth += 1;
        if depth > 64 {
            return None;
        }
        current = info.parent.as_deref().and_then(class_info);
    }
    None
}

/// `$class::loadMetadata($metadata)`: `None` when the class (and its
/// parents) declare no such static method.
pub fn call_load_metadata(class: &str, metadata: &dyn ClassMetadata) -> Option<Result<(), Thrown>> {
    let mut current = class_info(class);
    while let Some(info) = current {
        if let Some(load) = &info.load_metadata {
            return Some(load(metadata));
        }
        current = info.parent.as_deref().and_then(class_info);
    }
    None
}

fn reflection_exception(message: String) -> Thrown {
    Thrown::new("ReflectionException", message, 0)
}

/// `new ReflectionClass($name)`'s exception for a class that does not
/// exist.
pub fn class_does_not_exist(name: &str) -> Thrown {
    Thrown::new(
        "ReflectionException",
        format!("Class \"{name}\" does not exist"),
        -1,
    )
}

/// `ReflectionClass`.
#[derive(Debug, Clone)]
pub struct ReflectionClass {
    info: Arc<ClassInfo>,
}

impl ReflectionClass {
    /// `new ReflectionClass($name)`.
    pub fn new(name: &str) -> Result<Self, Thrown> {
        class_info(name)
            .map(|info| Self { info })
            .ok_or_else(|| class_does_not_exist(name))
    }

    /// `new ReflectionClass($object)`.
    pub fn of(object: &dyn PhpObject) -> Result<Self, Thrown> {
        Self::new(object.class())
    }

    pub fn info(&self) -> &ClassInfo {
        &self.info
    }

    /// `getName()`.
    pub fn get_name(&self) -> &str {
        &self.info.name
    }

    /// `getShortName()`: the name after the last `\`.
    pub fn get_short_name(&self) -> &str {
        let name = &self.info.name;
        name.rfind('\\')
            .map_or(name.as_str(), |pos| &name[pos + 1..])
    }

    /// `getNamespaceName()`: the name before the last `\` (empty in the
    /// global namespace).
    pub fn get_namespace_name(&self) -> &str {
        let name = &self.info.name;
        name.rfind('\\').map_or("", |pos| &name[..pos])
    }

    /// `isAnonymous()`.
    pub fn is_anonymous(&self) -> bool {
        self.info.anonymous
    }

    /// `isInterface()`.
    pub fn is_interface(&self) -> bool {
        self.info.kind == ClassKind::Interface
    }

    /// `isEnum()`.
    pub fn is_enum(&self) -> bool {
        self.info.kind == ClassKind::Enum
    }

    /// `isAbstract()` (an interface is abstract too, as in PHP).
    pub fn is_abstract(&self) -> bool {
        self.info.is_abstract || self.info.kind == ClassKind::Interface
    }

    /// `isFinal()` (an enum is final, as in PHP).
    pub fn is_final(&self) -> bool {
        self.info.is_final || self.info.kind == ClassKind::Enum
    }

    /// `isInternal()`.
    pub fn is_internal(&self) -> bool {
        self.info.is_internal
    }

    /// `newInstanceWithoutConstructor()`: the class's [`NewInstanceFn`],
    /// or a `GenericObject` with the public non-static defaults; PHP's
    /// `Error` for an interface, trait, enum or abstract class, and its
    /// `ReflectionException` for an internal final class.
    pub fn new_instance_without_constructor(&self) -> Result<Object, Thrown> {
        let info = &self.info;
        let kind = match info.kind {
            ClassKind::Interface => Some("interface"),
            ClassKind::Trait => Some("trait"),
            ClassKind::Enum => Some("enum"),
            ClassKind::Class if info.is_abstract => Some("abstract class"),
            ClassKind::Class => None,
        };
        if let Some(kind) = kind {
            return Err(Thrown::new(
                "Error",
                format!("Cannot instantiate {kind} {}", info.name),
                0,
            ));
        }
        if info.is_internal && info.is_final {
            return Err(reflection_exception(format!(
                "Class {} is an internal class marked as final that cannot be instantiated without invoking its constructor",
                info.name
            )));
        }
        self.blank_instance()
    }

    /// A blank instance through the class's [`NewInstanceFn`] (what
    /// `unserialize()` of an empty payload gives for a class that
    /// reflection cannot instantiate), or the default `GenericObject`.
    pub fn blank_instance(&self) -> Result<Object, Thrown> {
        if let Some(new_instance) = &self.info.new_instance {
            return new_instance();
        }
        if self.info.name.eq_ignore_ascii_case(StdClass::CLASS) {
            return Ok(Arc::new(StdClass::default()));
        }
        let mut properties = PhpArray::new();
        // PHP lays a parent's properties out before the child's.
        let mut chain = vec![self.clone()];
        while let Some(parent) = chain.last().and_then(ReflectionClass::get_parent_class) {
            if chain.len() > 64 {
                break;
            }
            chain.push(parent);
        }
        let mut public: Vec<String> = Vec::new();
        for c in chain.iter().rev() {
            for p in &c.info.properties {
                if p.visibility == Visibility::Public && !p.is_static && !public.contains(&p.name) {
                    public.push(p.name.clone());
                }
            }
        }
        let defaults = self.get_default_properties();
        for name in public {
            let key = Key::from(name.as_str());
            if let Some(value) = defaults.get(&key) {
                properties.insert(key, value.clone());
            }
        }
        Ok(Arc::new(GenericObject::new(
            self.info.name.clone(),
            properties,
        )))
    }

    /// `getParentClass()`: `None` for `false`.
    pub fn get_parent_class(&self) -> Option<ReflectionClass> {
        self.info
            .parent
            .as_deref()
            .and_then(class_info)
            .map(|info| Self { info })
    }

    /// `implementsInterface($interface)`: the class is, extends or
    /// implements it; the `ReflectionException`s for a name that is not
    /// an interface.
    pub fn implements_interface(&self, interface: &str) -> Result<bool, Thrown> {
        let Some(target) = class_info(interface) else {
            return Err(reflection_exception(format!(
                "Interface \"{interface}\" does not exist"
            )));
        };
        if target.kind != ClassKind::Interface {
            return Err(reflection_exception(format!(
                "{} is not an interface",
                target.name
            )));
        }
        Ok(is_a(&self.info.name, &target.name))
    }

    /// `isSubclassOf($class)`.
    pub fn is_subclass_of(&self, class: &str) -> bool {
        !self.info.name.eq_ignore_ascii_case(class) && is_a(&self.info.name, class)
    }

    /// `getFileName()`: `None` for `false`.
    pub fn get_file_name(&self) -> Option<&PathBuf> {
        self.info.file_name.as_ref()
    }

    /// `hasMethod($name)`.
    pub fn has_method(&self, name: &str) -> bool {
        find_method(&self.info.name, name).is_some()
    }

    /// `getMethod($name)`.
    pub fn get_method(&self, name: &str) -> Result<MethodInfo, Thrown> {
        find_method(&self.info.name, name).ok_or_else(|| {
            reflection_exception(format!(
                "Method {}::{name}() does not exist",
                self.info.name
            ))
        })
    }

    /// The property `name` as `new ReflectionProperty($class, $name)`
    /// finds it: declared in the class, or a non-private property of a
    /// parent class; with the class that declares it.
    fn find_property(&self, name: &str) -> Option<(Arc<ClassInfo>, PropertyInfo)> {
        if let Some(p) = self.info.properties.iter().find(|p| p.name == name) {
            return Some((Arc::clone(&self.info), p.clone()));
        }
        let mut current = self.get_parent_class();
        let mut depth = 0;
        while let Some(class) = current {
            if let Some(p) = class
                .info
                .properties
                .iter()
                .find(|p| p.name == name && p.visibility != Visibility::Private)
            {
                return Some((Arc::clone(&class.info), p.clone()));
            }
            depth += 1;
            if depth > 64 {
                return None;
            }
            current = class.get_parent_class();
        }
        None
    }

    /// `hasProperty($name)`.
    pub fn has_property(&self, name: &str) -> bool {
        self.find_property(name).is_some()
    }

    /// `getProperty($name)`.
    pub fn get_property(&self, name: &str) -> Result<ReflectionProperty, Thrown> {
        ReflectionProperty::new(&self.info.name, name)
    }

    /// `getDefaultProperties()`: name → default value of the properties
    /// that have one (typed properties without a default are left out),
    /// static ones included, the class's own first, then the inherited
    /// non-private ones.
    pub fn get_default_properties(&self) -> PhpArray {
        let mut defaults = PhpArray::new();
        let mut class = Some(self.clone());
        let mut own = true;
        let mut depth = 0;
        while let Some(c) = class {
            for p in &c.info.properties {
                if !own && p.visibility == Visibility::Private {
                    continue;
                }
                let key = Key::from(p.name.as_str());
                if defaults.contains_key(&key) {
                    continue;
                }
                if let Some(default) = &p.default {
                    defaults.insert(key, default.clone());
                }
            }
            own = false;
            depth += 1;
            if depth > 64 {
                break;
            }
            class = c.get_parent_class();
        }
        defaults
    }
}

/// A class whose properties reflection reads and writes whatever their
/// visibility (see the module docs). Reading is collections'
/// [`RawProperties::raw_property`]: `Some(Ok(value))` for an initialized
/// property, `Some(Err(...))` for a typed property that is not
/// initialized (or was unset), `None` for no such property.
pub trait ReflectionObject: RawProperties {
    /// `ReflectionProperty::setValue($this, $value)`: `None` when the
    /// class has no such property.
    fn set_raw_property(&self, name: &str, value: Value) -> Option<Result<(), Thrown>>;

    /// `unset($this->$name)` from inside the class: the property becomes
    /// uninitialized. `None` when the class has no such property.
    fn unset_raw_property(&self, name: &str) -> Option<Result<(), Thrown>>;

    /// The object as a `Doctrine\Persistence\Proxy`, when its class
    /// implements it.
    fn as_proxy(&self) -> Option<&dyn Proxy> {
        None
    }
}

type Writer = fn(&dyn PhpObject, &str, Option<Value>) -> Option<Result<(), Thrown>>;

struct Accessors {
    read: fn(&dyn PhpObject, &str) -> Option<Result<Value, Thrown>>,
    write: Writer,
    proxy: for<'a> fn(&'a dyn PhpObject) -> Option<&'a dyn Proxy>,
}

static OBJECTS: RwLock<Vec<(TypeId, Accessors)>> = RwLock::new(Vec::new());

fn read<T: ReflectionObject>(object: &dyn PhpObject, name: &str) -> Option<Result<Value, Thrown>> {
    object.downcast_ref::<T>()?.raw_property(name)
}

fn write<T: ReflectionObject>(
    object: &dyn PhpObject,
    name: &str,
    value: Option<Value>,
) -> Option<Result<(), Thrown>> {
    let object = object.downcast_ref::<T>()?;
    match value {
        Some(value) => object.set_raw_property(name, value),
        None => object.unset_raw_property(name),
    }
}

fn proxy<T: ReflectionObject>(object: &dyn PhpObject) -> Option<&dyn Proxy> {
    object.downcast_ref::<T>()?.as_proxy()
}

/// Makes the properties of `T` readable and writable by reflection (and
/// readable by collections' `ClosureExpressionVisitor`). Registering a
/// class again does nothing.
pub fn register_reflection_object<T: ReflectionObject>() {
    register_raw_properties::<T>();
    let id = TypeId::of::<T>();
    let mut objects = OBJECTS.write().unwrap_or_else(PoisonError::into_inner);
    if objects.iter().all(|(known, _)| *known != id) {
        objects.push((
            id,
            Accessors {
                read: read::<T>,
                write: write::<T>,
                proxy: proxy::<T>,
            },
        ));
    }
}

fn accessors<R>(object: &dyn PhpObject, f: impl FnOnce(&Accessors) -> R) -> Option<R> {
    let id = (object as &dyn Any).type_id();
    let objects = OBJECTS.read().unwrap_or_else(PoisonError::into_inner);
    objects
        .iter()
        .find(|(known, _)| *known == id)
        .map(|(_, a)| f(a))
}

/// The raw value of the property `name` of `object`: `Some(Ok)` when it
/// is initialized, `Some(Err)` when it is not, `None` when the object has
/// no such property. An object whose class is not registered is read
/// through its public properties.
pub fn read_property(object: &dyn PhpObject, name: &str) -> Option<Result<Value, Thrown>> {
    if let Some(reader) = accessors(object, |a| a.read) {
        return reader(object, name);
    }
    object.properties().get(&Key::from(name)).cloned().map(Ok)
}

/// Writes (`Some`) or unsets (`None`) the property `name` of `object`.
pub fn write_property(
    object: &dyn PhpObject,
    name: &str,
    value: Option<Value>,
) -> Result<(), Thrown> {
    let writer = accessors(object, |a| a.write);
    let result = writer.and_then(|w| w(object, name, value));
    result.unwrap_or_else(|| {
        Err(Thrown::new(
            "Error",
            format!(
                "Cannot modify property {}::${name}: the class is not registered with register_reflection_object()",
                object.class()
            ),
            0,
        ))
    })
}

/// `$object instanceof Proxy`, for a registered class.
pub fn as_proxy(object: &dyn PhpObject) -> Option<&dyn Proxy> {
    let get = accessors(object, |a| a.proxy)?;
    get(object)
}

/// PHP's `ReflectionProperty`.
#[derive(Debug, Clone)]
pub struct ReflectionProperty {
    class: Arc<ClassInfo>,
    info: PropertyInfo,
}

impl ReflectionProperty {
    /// `new ReflectionProperty($class, $name)`.
    pub fn new(class: &str, name: &str) -> Result<Self, Thrown> {
        let reflection = ReflectionClass::new(class)?;
        let (declaring, info) = reflection.find_property(name).ok_or_else(|| {
            reflection_exception(format!(
                "Property {}::${name} does not exist",
                reflection.get_name()
            ))
        })?;
        Ok(Self {
            class: declaring,
            info,
        })
    }

    /// `$this->name`.
    pub fn get_name(&self) -> &str {
        &self.info.name
    }

    /// `$this->class`: the class that declares the property.
    pub fn class(&self) -> &str {
        &self.class.name
    }

    pub fn info(&self) -> &PropertyInfo {
        &self.info
    }

    /// `getDeclaringClass()`.
    pub fn get_declaring_class(&self) -> ReflectionClass {
        ReflectionClass {
            info: Arc::clone(&self.class),
        }
    }

    /// `getType()`.
    pub fn get_type(&self) -> Option<&PropertyType> {
        self.info.ty.as_ref()
    }

    /// `hasType()`.
    pub fn has_type(&self) -> bool {
        self.info.ty.is_some()
    }

    /// `getModifiers()`.
    pub fn get_modifiers(&self) -> i64 {
        let visibility = match self.info.visibility {
            Visibility::Public => IS_PUBLIC,
            Visibility::Protected => IS_PROTECTED,
            Visibility::Private => IS_PRIVATE,
        };
        let is_static = if self.info.is_static { IS_STATIC } else { 0 };
        let readonly = if self.info.is_readonly {
            IS_READONLY
        } else {
            0
        };
        visibility | is_static | readonly
    }

    pub fn is_public(&self) -> bool {
        self.info.visibility == Visibility::Public
    }

    pub fn is_protected(&self) -> bool {
        self.info.visibility == Visibility::Protected
    }

    pub fn is_private(&self) -> bool {
        self.info.visibility == Visibility::Private
    }

    pub fn is_static(&self) -> bool {
        self.info.is_static
    }

    pub fn is_readonly(&self) -> bool {
        self.info.is_readonly
    }

    /// `getDocComment()`: `None` for `false`.
    pub fn get_doc_comment(&self) -> Option<&str> {
        self.info.doc_comment.as_deref()
    }

    /// `getAttributes()`, by attribute class name.
    pub fn get_attributes(&self) -> &[String] {
        &self.info.attributes
    }

    /// `hasDefaultValue()`.
    pub fn has_default_value(&self) -> bool {
        self.info.default.is_some()
    }

    /// `getDefaultValue()` (`null` when there is none).
    pub fn get_default_value(&self) -> Value {
        self.info.default.clone().unwrap_or_default()
    }

    /// `getValue($object)`: the `Error` for a typed property that is not
    /// initialized.
    pub fn get_value(&self, object: &dyn PhpObject) -> Result<Value, Thrown> {
        match read_property(object, &self.info.name) {
            Some(result) => result,
            None if self.has_type() => Err(self.uninitialized_error()),
            None => Ok(Value::Null),
        }
    }

    /// `isInitialized($object)`.
    pub fn is_initialized(&self, object: &dyn PhpObject) -> bool {
        matches!(read_property(object, &self.info.name), Some(Ok(_)))
    }

    /// `setValue($object, $value)`.
    pub fn set_value(&self, object: &dyn PhpObject, value: Value) -> Result<(), Thrown> {
        write_property(object, &self.info.name, Some(value))
    }

    /// `unset($object->name)` from the declaring class's scope.
    pub fn unset_value(&self, object: &dyn PhpObject) -> Result<(), Thrown> {
        write_property(object, &self.info.name, None)
    }

    fn uninitialized_error(&self) -> Thrown {
        Thrown::new(
            "Error",
            format!(
                "Typed property {}::${} must not be accessed before initialization",
                self.class.name, self.info.name
            ),
            0,
        )
    }
}

/// A reflection property as doctrine/persistence hands it out
/// (`ReflectionService::getAccessibleProperty()`): PHP's
/// `ReflectionProperty` or one of the subclasses that change how values
/// are read and written (`RuntimeReflectionProperty`,
/// `TypedNoDefaultReflectionProperty`, `EnumReflectionProperty`). The
/// subclass is found by downcasting (`(p as &dyn Any).downcast_ref()`).
pub trait PropertyAccessor: Any + Send + Sync + fmt::Debug {
    /// The underlying PHP reflection: name, declaring class, type, ...
    fn reflection(&self) -> &ReflectionProperty;

    /// `getValue($object)`.
    fn get_value(&self, object: &dyn PhpObject) -> Result<Value, Thrown>;

    /// `setValue($object, $value)`.
    fn set_value(&self, object: &dyn PhpObject, value: Value) -> Result<(), Thrown>;

    /// `isInitialized($object)`.
    fn is_initialized(&self, object: &dyn PhpObject) -> bool {
        self.reflection().is_initialized(object)
    }

    /// `$this instanceof RuntimeReflectionProperty` (which
    /// `TypedNoDefaultReflectionProperty` extends).
    fn as_runtime_reflection_property(
        &self,
    ) -> Option<&crate::reflection::RuntimeReflectionProperty> {
        None
    }
}

impl PropertyAccessor for ReflectionProperty {
    fn reflection(&self) -> &ReflectionProperty {
        self
    }

    fn get_value(&self, object: &dyn PhpObject) -> Result<Value, Thrown> {
        Self::get_value(self, object)
    }

    fn set_value(&self, object: &dyn PhpObject, value: Value) -> Result<(), Thrown> {
        Self::set_value(self, object, value)
    }
}

impl dyn PropertyAccessor {
    /// `$this instanceof T`.
    pub fn downcast_ref<T: PropertyAccessor>(&self) -> Option<&T> {
        (self as &dyn Any).downcast_ref()
    }
}
