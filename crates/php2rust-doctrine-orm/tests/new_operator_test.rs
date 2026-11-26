//! Functional/NewOperatorTest.php on `SQLite` (`tests/common/orm.rs`).
//!
//! The DTO classes are declared from `tests/data/query_models.json`
//! (their reflection, which the parser's checks read) and construct
//! `php_value` generic objects: each registers its constructor for the
//! hydrator (`register_new_object_class_with_arguments()`), which binds
//! the positional and named arguments and coerces them to the parameter
//! types as PHP does for a call from `ReflectionClass::newInstanceArgs()`
//! (coercive typing). `ClassWithTooMuchArgs` and
//! `ClassWithPrivateConstructor` are declared here. A data provider's rows
//! run in one test.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/NewOperatorTest.php
//! Upstream: doctrine/orm tests/Tests/Models/CMS/CmsUserDTO.php tests/Tests/Models/CMS/CmsAddressDTO.php
//! Upstream: doctrine/orm tests/Tests/Models/CMS/CmsAddressDTONamedArgs.php tests/Tests/Models/CMS/CmsDumbDTO.php
//! Upstream: doctrine/orm tests/Tests/Models/CMS/CmsUserDTONamedArgs.php tests/Tests/Models/CMS/CmsUserDTOVariadicArg.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use std::fmt::Write as _;
use std::sync::{Arc, Once};

use php_value::compare::loose_equals;
use php_value::object::GenericObject;
use php_value::{Key, Object, PhpArray, Thrown, Value};
use php2rust_doctrine_orm::collection_object::downcast;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::internal::hydration::register_new_object_class_with_arguments;
use php2rust_doctrine_orm::object::EntityObject;
use php2rust_doctrine_orm::query::context::{HYDRATE_ARRAY, HYDRATE_OBJECT};
use php2rust_doctrine_persistence::php_reflection::{
    ClassInfo, MethodInfo, Visibility, register_class,
};

use orm::{
    CMS_ADDRESS, CMS_MODELS, CMS_USER, add_phonenumber, cms_address, cms_email, cms_phonenumber,
    cms_user, get, s, set, set_address, set_email,
};

const CMS_USER_DTO: &str = "Doctrine\\Tests\\Models\\CMS\\CmsUserDTO";
const CMS_ADDRESS_DTO: &str = "Doctrine\\Tests\\Models\\CMS\\CmsAddressDTO";
const CMS_ADDRESS_DTO_NAMED_ARGS: &str = "Doctrine\\Tests\\Models\\CMS\\CmsAddressDTONamedArgs";
const CMS_DUMB_DTO: &str = "Doctrine\\Tests\\Models\\CMS\\CmsDumbDTO";
const CMS_USER_DTO_NAMED_ARGS: &str = "Doctrine\\Tests\\Models\\CMS\\CmsUserDTONamedArgs";
const CMS_USER_DTO_VARIADIC_ARG: &str = "Doctrine\\Tests\\Models\\CMS\\CmsUserDTOVariadicArg";
const CLASS_WITH_TOO_MUCH_ARGS: &str = "Doctrine\\Tests\\ORM\\Functional\\ClassWithTooMuchArgs";
const CLASS_WITH_PRIVATE_CONSTRUCTOR: &str =
    "Doctrine\\Tests\\ORM\\Functional\\ClassWithPrivateConstructor";
const QUERY_EXCEPTION: &str = "Doctrine\\ORM\\Query\\QueryException";

// The DTOs

/// A constructor parameter's declared type.
#[derive(Clone, Copy)]
enum Ty {
    /// `string|null`.
    String,
    /// `int|null`.
    Int,
    /// `mixed`.
    Mixed,
    /// `SomeClass|string|null`.
    ClassOrString(&'static str),
    /// `SomeClass|null`.
    Class(&'static str),
}

fn type_name(ty: Ty) -> String {
    match ty {
        Ty::String => "?string".to_owned(),
        Ty::Int => "?int".to_owned(),
        Ty::Mixed => "mixed".to_owned(),
        Ty::ClassOrString(class) => format!("{class}|string|null"),
        Ty::Class(class) => format!("?{class}"),
    }
}

fn instance_of(value: &Value, class: &str) -> bool {
    value
        .as_object()
        .is_some_and(|o| o.class() == class || o.instance_of(class))
}

/// Coercive typing of an argument (`ReflectionClass::newInstanceArgs()`
/// calls the constructor from internal code).
fn coerce(class: &str, n: usize, name: &str, ty: Ty, value: Value) -> Result<Value, Thrown> {
    let type_error = |value: &Value| {
        Thrown::new(
            "TypeError",
            format!(
                "{class}::__construct(): Argument #{n} (${name}) must be of type {}, {} given",
                type_name(ty),
                value.value_name()
            ),
            0,
        )
    };
    let to_string = |value: Value| -> Result<Value, Thrown> {
        match &value {
            Value::Null | Value::String(_) => Ok(value),
            Value::Int(_) | Value::Float(_) | Value::Bool(_) => {
                Ok(Value::String(value.to_php_string().unwrap_or_default()))
            }
            _ => Err(type_error(&value)),
        }
    };
    match ty {
        Ty::Mixed => Ok(value),
        Ty::String => to_string(value),
        Ty::Int => match &value {
            Value::Null | Value::Int(_) => Ok(value),
            Value::Bool(b) => Ok(Value::Int(i64::from(*b))),
            Value::String(text) => text
                .to_string_lossy()
                .trim()
                .parse::<i64>()
                .map(Value::Int)
                .map_err(|_| type_error(&value)),
            _ => Err(type_error(&value)),
        },
        Ty::ClassOrString(expected) => {
            if instance_of(&value, expected) {
                Ok(value)
            } else {
                to_string(value)
            }
        }
        Ty::Class(expected) => {
            if value.is_null() || instance_of(&value, expected) {
                Ok(value)
            } else {
                Err(type_error(&value))
            }
        }
    }
}

/// Binds `newInstanceArgs($args)` to the parameters: the positional
/// arguments first, then the named ones; the rest goes to the variadic
/// parameter when there is one.
fn bind(
    class: &str,
    params: &[(&str, Ty)],
    args: &PhpArray,
    variadic: bool,
) -> Result<(Vec<Value>, PhpArray), Thrown> {
    let mut bound: Vec<Option<Value>> = vec![None; params.len()];
    let mut rest = PhpArray::new();
    let mut position = 0;
    let mut named = false;
    for (key, value) in args.iter() {
        match key {
            Key::Int(_) => {
                if named {
                    return Err(Thrown::new(
                        "Error",
                        "Cannot use positional argument after named argument during unpacking",
                        0,
                    ));
                }
                if position < params.len() {
                    bound[position] = Some(value.clone());
                } else if variadic {
                    rest.push(value.clone()).map_err(|_| {
                        Thrown::new(
                            "Error",
                            "Cannot add element to the array as the next element is already occupied",
                            0,
                        )
                    })?;
                }
                position += 1;
            }
            Key::Str(name) => {
                named = true;
                let name = name.to_string_lossy().into_owned();
                match params.iter().position(|(p, _)| *p == name) {
                    Some(at) => {
                        if bound[at].is_some() {
                            return Err(Thrown::new(
                                "Error",
                                format!("Named parameter ${name} overwrites previous argument"),
                                0,
                            ));
                        }
                        bound[at] = Some(value.clone());
                    }
                    None if variadic => {
                        rest.insert(name.as_str(), value.clone());
                    }
                    None => {
                        return Err(Thrown::new(
                            "Error",
                            format!("Unknown named parameter ${name}"),
                            0,
                        ));
                    }
                }
            }
        }
    }
    let mut values = Vec::with_capacity(params.len());
    for (n, ((name, ty), value)) in params.iter().zip(bound).enumerate() {
        // Every parameter defaults to null.
        values.push(coerce(
            class,
            n + 1,
            name,
            *ty,
            value.unwrap_or(Value::Null),
        )?);
    }
    Ok((values, rest))
}

/// A DTO class whose constructor promotes its parameters.
fn register_dto(class: &'static str, params: &'static [(&'static str, Ty)]) {
    register_new_object_class_with_arguments(
        class,
        Arc::new(move |args| {
            let (values, _) = bind(class, params, args, false)?;
            let mut properties = PhpArray::new();
            for ((name, _), value) in params.iter().zip(values) {
                properties.insert(*name, value);
            }
            Ok(Value::Object(Arc::new(GenericObject::new(
                class, properties,
            ))))
        }),
    );
}

fn register_dtos() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        models::register_query_models();
        register_dto(
            CMS_USER_DTO,
            &[
                ("name", Ty::String),
                ("email", Ty::String),
                ("address", Ty::ClassOrString(CMS_ADDRESS_DTO)),
                ("phonenumbers", Ty::Int),
            ],
        );
        register_dto(
            CMS_ADDRESS_DTO,
            &[
                ("country", Ty::String),
                ("city", Ty::String),
                ("zip", Ty::String),
                ("address", Ty::String),
                ("other", Ty::Class(CMS_DUMB_DTO)),
            ],
        );
        register_dto(
            CMS_ADDRESS_DTO_NAMED_ARGS,
            &[
                ("country", Ty::String),
                ("city", Ty::String),
                ("zip", Ty::String),
                ("address", Ty::ClassOrString(CMS_ADDRESS_DTO)),
            ],
        );
        register_dto(
            CMS_DUMB_DTO,
            &[
                ("val1", Ty::Mixed),
                ("val2", Ty::Mixed),
                ("val3", Ty::Mixed),
                ("val4", Ty::Mixed),
                ("val5", Ty::Mixed),
            ],
        );
        register_dto(
            CMS_USER_DTO_NAMED_ARGS,
            &[
                ("name", Ty::String),
                ("email", Ty::String),
                ("address", Ty::String),
                ("phonenumbers", Ty::Int),
                ("addressDto", Ty::Class(CMS_ADDRESS_DTO)),
                ("addressDtoNamedArgs", Ty::Class(CMS_ADDRESS_DTO_NAMED_ARGS)),
                ("dumb", Ty::Class(CMS_DUMB_DTO)),
                ("addressEntity", Ty::Class(CMS_ADDRESS)),
            ],
        );
        // `__construct(...$args)`.
        register_new_object_class_with_arguments(
            CMS_USER_DTO_VARIADIC_ARG,
            Arc::new(|args| {
                let (_, args) = bind(CMS_USER_DTO_VARIADIC_ARG, &[], args, true)?;
                let arg = |name: &str| args.get(&Key::from(name)).cloned().unwrap_or_default();
                let mut properties = PhpArray::new();
                properties.insert("name", arg("name"));
                properties.insert("email", arg("email"));
                properties.insert("address", arg("address"));
                properties.insert("phonenumbers", arg("phonenumbers"));
                properties.insert("otherProperties", Value::Array(args.clone()));
                Ok(Value::Object(Arc::new(GenericObject::new(
                    CMS_USER_DTO_VARIADIC_ARG,
                    properties,
                ))))
            }),
        );
        // The classes the test file declares.
        register_class(
            ClassInfo::class(CLASS_WITH_TOO_MUCH_ARGS)
                .method(MethodInfo::new("__construct").required_parameters(2)),
        );
        register_class(
            ClassInfo::class(CLASS_WITH_PRIVATE_CONSTRUCTOR).method(
                MethodInfo::new("__construct")
                    .required_parameters(1)
                    .visibility(Visibility::Private),
            ),
        );
    });
}

// The fixtures

/// What the assertions read of a fixture user.
#[derive(Debug, Clone)]
struct Fixture {
    id: Value,
    name: Value,
    username: Value,
    status: Value,
    email: Value,
    city: Value,
    country: Value,
    zip: Value,
    address_id: Value,
    phonenumbers: i64,
}

async fn set_up() -> (Arc<EntityManager>, Vec<Fixture>) {
    let em = orm::set_up(&CMS_MODELS).await;
    register_dtos();
    let fixtures = load_fixtures(&em).await;
    (em, fixtures)
}

fn user_with(
    name: &str,
    username: &str,
    email: &str,
    (zip, city, country): (&str, &str, &str),
    phonenumbers: &[&str],
) -> Object {
    let user = cms_user();
    set_email(&user, &cms_email(email));
    set_address(&user, &cms_address(country, zip, city));
    for number in phonenumbers {
        add_phonenumber(&user, &cms_phonenumber(number));
    }
    set(&user, "name", s(name));
    set(&user, "username", s(username));
    set(&user, "status", s("developer"));
    user
}

async fn load_fixtures(em: &Arc<EntityManager>) -> Vec<Fixture> {
    let u1 = user_with(
        "Test 1",
        "1test",
        "email@test1.com",
        ("111111111", "Some City 1", "Some Country 2"),
        &["(11) 1111-1111"],
    );
    let u2 = user_with(
        "Test 2",
        "2test",
        "email@test2.com",
        ("222222222", "Some City 2", "Some Country 2"),
        &["(22) 1111-1111", "(22) 2222-2222"],
    );
    let u3 = user_with(
        "Test 3",
        "3test",
        "email@test3.com",
        ("33333333", "Some City 3", "Some Country 3"),
        &["(33) 1111-1111", "(33) 2222-2222", "(33) 3333-3333"],
    );

    em.persist(&u1).unwrap();
    em.persist(&u2).unwrap();
    em.persist(&u3).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    [(u1, 1), (u2, 2), (u3, 3)]
        .into_iter()
        .map(|(user, phonenumbers)| {
            let email = orm::get_object(&user, "email").unwrap();
            let address = orm::get_object(&user, "address").unwrap();
            Fixture {
                id: get(&user, "id"),
                name: get(&user, "name"),
                username: get(&user, "username"),
                status: get(&user, "status"),
                email: get(&email, "email"),
                city: get(&address, "city"),
                country: get(&address, "country"),
                zip: get(&address, "zip"),
                address_id: get(&address, "id"),
                phonenumbers,
            }
        })
        .collect()
}

// Reading the results

/// `$value[$key]`.
fn at(value: &Value, key: impl Into<Key>) -> Value {
    let key = key.into();
    match value {
        Value::Array(array) => array
            .get(&key)
            .cloned()
            .unwrap_or_else(|| panic!("no key {key:?} in {array:?}")),
        other => panic!("not an array: {other:?}"),
    }
}

/// A list index.
fn idx(i: usize) -> Key {
    Key::Int(i64::try_from(i).unwrap())
}

/// `$value->$name`.
fn prop(value: &Value, name: &str) -> Value {
    let Value::Object(object) = value else {
        panic!("not an object: {value:?}");
    };
    if let Some(entity) = downcast::<EntityObject>(object) {
        return entity.get(name).unwrap_or(Value::Null);
    }
    object
        .properties()
        .get(&Key::from(name))
        .cloned()
        .unwrap_or_else(|| panic!("no property {name} in {object:?}"))
}

fn assert_instance_of(class: &str, value: &Value) {
    assert!(
        value.as_object().is_some_and(|o| o.class() == class),
        "{value:?} is not a {class}"
    );
}

/// `assertEquals()`.
#[track_caller]
fn assert_equals(expected: &Value, actual: &Value) {
    assert!(
        loose_equals(expected, actual),
        "{expected:?} does not equal {actual:?}"
    );
}

async fn get_result(em: &Arc<EntityManager>, dql: &str, hydration_mode: i64) -> Value {
    em.create_query(dql)
        .get_result(hydration_mode)
        .await
        .unwrap()
}

fn assert_count(expected: usize, result: &Value) {
    assert_eq!(result.as_array().unwrap().len(), expected);
}

fn int(i: i64) -> Value {
    Value::Int(i)
}

fn add(a: &Value, b: &Value) -> Value {
    php_value::ops::add(a, b).unwrap()
}

/// `sprintf('%s %s %s', $country, $city, $zip)`.
fn country_city_zip(f: &Fixture) -> Value {
    let text = |v: &Value| v.to_php_string().unwrap().to_string_lossy().into_owned();
    s(&format!(
        "{} {} {}",
        text(&f.country),
        text(&f.city),
        text(&f.zip)
    ))
}

/// The provider `provideDataForHydrationMode()`.
const HYDRATION_MODES: [i64; 2] = [HYDRATE_ARRAY, HYDRATE_OBJECT];

const BASIC_USAGE_DQL: &str = "
            SELECT
                new Doctrine\\Tests\\Models\\CMS\\CmsUserDTO(
                    u.name,
                    e.email,
                    a.city
                )
            FROM
                Doctrine\\Tests\\Models\\CMS\\CmsUser u
            JOIN
                u.email e
            JOIN
                u.address a
            ORDER BY
                u.name";

fn assert_user_dtos(result: &Value, fixtures: &[Fixture]) {
    assert_count(3, result);
    for i in 0..fixtures.len() {
        assert_instance_of(CMS_USER_DTO, &at(result, idx(i)));
    }
    for (i, f) in fixtures.iter().enumerate() {
        let dto = at(result, idx(i));
        assert_equals(&f.name, &prop(&dto, "name"));
        assert_equals(&f.email, &prop(&dto, "email"));
        assert_equals(&f.city, &prop(&dto, "address"));
    }
}

#[tokio::test]
async fn testShouldSupportsBasicUsage() {
    let (em, fixtures) = set_up().await;
    for hydration_mode in HYDRATION_MODES {
        let result = get_result(&em, BASIC_USAGE_DQL, hydration_mode).await;
        assert_user_dtos(&result, &fixtures);
    }
}

#[tokio::test]
async fn testShouldIgnoreAliasesForSingleObject() {
    let (em, fixtures) = set_up().await;
    let dql = "
            SELECT
                new Doctrine\\Tests\\Models\\CMS\\CmsUserDTO(
                    u.name,
                    e.email,
                    a.city
                ) as cmsUser
            FROM
                Doctrine\\Tests\\Models\\CMS\\CmsUser u
            JOIN
                u.email e
            JOIN
                u.address a
            ORDER BY
                u.name";
    for hydration_mode in HYDRATION_MODES {
        let result = get_result(&em, dql, hydration_mode).await;
        assert_user_dtos(&result, &fixtures);
    }
}

#[tokio::test]
async fn testShouldAssumeFromEntityNamespaceWhenNotGiven() {
    let (em, _fixtures) = set_up().await;
    let dql = "
            SELECT
                new CmsUserDTO(u.name, e.email, a.city)
            FROM
                Doctrine\\Tests\\Models\\CMS\\CmsUser u
            JOIN
                u.email e
            JOIN
                u.address a
            ORDER BY
                u.name";

    let result = get_result(&em, dql, HYDRATE_OBJECT).await;

    assert_count(3, &result);

    for i in 0..3 {
        assert_instance_of(CMS_USER_DTO, &at(&result, i));
    }
}

#[tokio::test]
async fn testShouldSupportLiteralExpression() {
    let (em, fixtures) = set_up().await;
    let dql = "
            SELECT
                new Doctrine\\Tests\\Models\\CMS\\CmsUserDTO(
                    u.name,
                    'fabio.bat.silva@gmail.com',
                    FALSE,
                    123
                )
            FROM
                Doctrine\\Tests\\Models\\CMS\\CmsUser u
            JOIN
                u.email e
            JOIN
                u.address a
            JOIN
                u.phonenumbers p
            GROUP BY
                u, e, a
            ORDER BY
                u.name";

    let result = get_result(&em, dql, HYDRATE_OBJECT).await;

    assert_count(3, &result);

    for (i, f) in fixtures.iter().enumerate() {
        let dto = at(&result, idx(i));
        assert_instance_of(CMS_USER_DTO, &dto);
        assert_equals(&f.name, &prop(&dto, "name"));
        assert_equals(&s("fabio.bat.silva@gmail.com"), &prop(&dto, "email"));
        // Note that the type hints on the DTO model convert false -> ''
        assert_equals(&s(""), &prop(&dto, "address"));
        assert_equals(&int(123), &prop(&dto, "phonenumbers"));
    }
}

#[tokio::test]
async fn testShouldSupportNullLiteralExpression() {
    let (em, fixtures) = set_up().await;
    let dql = "
            SELECT
                new Doctrine\\Tests\\Models\\CMS\\CmsUserDTO(
                    u.name,
                    'fabio.bat.silva@gmail.com',
                    null,
                    123
                )
            FROM
                Doctrine\\Tests\\Models\\CMS\\CmsUser u
            ORDER BY
                u.name";

    let result = get_result(&em, dql, HYDRATE_OBJECT).await;

    assert_count(3, &result);

    for (i, f) in fixtures.iter().enumerate() {
        let dto = at(&result, idx(i));
        assert_instance_of(CMS_USER_DTO, &dto);
        assert_equals(&f.name, &prop(&dto, "name"));
        assert_equals(&s("fabio.bat.silva@gmail.com"), &prop(&dto, "email"));
        assert!(prop(&dto, "address").is_null());
        assert_equals(&int(123), &prop(&dto, "phonenumbers"));
    }
}

#[tokio::test]
async fn testShouldSupportCaseExpression() {
    let (em, fixtures) = set_up().await;
    let dql = "
            SELECT
                new Doctrine\\Tests\\Models\\CMS\\CmsUserDTO(
                    u.name,
                    CASE WHEN (e.email = 'email@test1.com') THEN 'TEST1' ELSE 'OTHER_TEST' END
                )
            FROM
                Doctrine\\Tests\\Models\\CMS\\CmsUser u
            JOIN
                u.email e
            JOIN
                u.address a
            JOIN
                u.phonenumbers p
            GROUP BY
                u, e, a
            ORDER BY
                u.name";

    let result = get_result(&em, dql, HYDRATE_OBJECT).await;

    assert_count(3, &result);

    let emails = ["TEST1", "OTHER_TEST", "OTHER_TEST"];
    for (i, f) in fixtures.iter().enumerate() {
        let dto = at(&result, idx(i));
        assert_instance_of(CMS_USER_DTO, &dto);
        assert_equals(&f.name, &prop(&dto, "name"));
        assert_equals(&s(emails[i]), &prop(&dto, "email"));
    }
}

/// The tests whose fourth argument is an expression of the fixture.
async fn fourth_argument(fourth: &str, expected: impl Fn(&Fixture) -> Value) {
    let (em, fixtures) = set_up().await;
    let dql = format!(
        "
            SELECT
                new Doctrine\\Tests\\Models\\CMS\\CmsUserDTO(
                    u.name,
                    e.email,
                    a.city,
                    {fourth}
                )
            FROM
                Doctrine\\Tests\\Models\\CMS\\CmsUser u
            JOIN
                u.email e
            JOIN
                u.address a
            JOIN
                u.phonenumbers p
            GROUP BY
                u, e, a
            ORDER BY
                u.name"
    );

    let result = get_result(&em, &dql, HYDRATE_OBJECT).await;

    assert_count(3, &result);

    for (i, f) in fixtures.iter().enumerate() {
        let dto = at(&result, idx(i));
        assert_instance_of(CMS_USER_DTO, &dto);
        assert_equals(&f.name, &prop(&dto, "name"));
        assert_equals(&f.email, &prop(&dto, "email"));
        assert_equals(&f.city, &prop(&dto, "address"));
        assert_equals(&expected(f), &prop(&dto, "phonenumbers"));
    }
}

#[tokio::test]
async fn testShouldSupportSimpleArithmeticExpression() {
    fourth_argument("a.id + u.id", |f| add(&f.address_id, &f.id)).await;
}

#[tokio::test]
async fn testShouldSupportAggregateFunctions() {
    fourth_argument("COUNT(p)", |f| int(f.phonenumbers)).await;
}

#[tokio::test]
async fn testShouldSupportArithmeticExpression() {
    fourth_argument("COUNT(p) + u.id", |f| add(&int(f.phonenumbers), &f.id)).await;
}

/// The tests that select a user DTO, an address DTO and scalars: the
/// aliases of each (`None` for none) and where the results are.
async fn multiple_new_operators(
    user_alias: Option<&str>,
    address_alias: Option<&str>,
    scalars: &[(&str, Option<&str>)],
) {
    let (em, fixtures) = set_up().await;
    let alias = |a: Option<&str>| a.map(|a| format!(" as {a}")).unwrap_or_default();
    let mut select = format!(
        "
                new CmsUserDTO(
                    u.name,
                    e.email
                ){},
                new CmsAddressDTO(
                    a.country,
                    a.city
                ){}",
        alias(user_alias),
        alias(address_alias)
    );
    for (field, field_alias) in scalars {
        write!(
            select,
            ",\n                u.{field}{}",
            alias(*field_alias)
        )
        .unwrap();
    }
    let dql = format!(
        "
            SELECT{select}
            FROM
                Doctrine\\Tests\\Models\\CMS\\CmsUser u
            JOIN
                u.email e
            JOIN
                u.address a
            ORDER BY
                u.name"
    );

    let result = get_result(&em, &dql, HYDRATE_OBJECT).await;

    assert_count(3, &result);

    // The objects without an alias are at 0, 1, ...
    let mut position = 0;
    let mut key = |a: Option<&str>| -> Key {
        if let Some(a) = a {
            Key::from(a)
        } else {
            position += 1;
            Key::Int(position - 1)
        }
    };
    let user_key = key(user_alias);
    let address_key = key(address_alias);

    for (i, f) in fixtures.iter().enumerate() {
        let row = at(&result, idx(i));
        let user = at(&row, user_key.clone());
        let address = at(&row, address_key.clone());
        assert_instance_of(CMS_USER_DTO, &user);
        assert_instance_of(CMS_ADDRESS_DTO, &address);
        assert_equals(&f.name, &prop(&user, "name"));
        assert_equals(&f.email, &prop(&user, "email"));
        assert_equals(&f.city, &prop(&address, "city"));
        assert_equals(&f.country, &prop(&address, "country"));
        for (field, field_alias) in scalars {
            let expected = match *field {
                "status" => &f.status,
                _ => &f.username,
            };
            assert_equals(expected, &at(&row, field_alias.unwrap_or(field)));
        }
    }
}

#[tokio::test]
async fn testShouldSupportMultipleNewOperators() {
    multiple_new_operators(None, None, &[]).await;
}

#[tokio::test]
async fn testShouldSupportMultipleNewOperatorsWithAliases() {
    multiple_new_operators(Some("cmsUser"), Some("cmsAddress"), &[]).await;
}

#[tokio::test]
async fn testShouldSupportMultipleNewOperatorsWithAndWithoutAliases() {
    multiple_new_operators(Some("cmsUser"), None, &[]).await;
}

#[tokio::test]
async fn testShouldSupportMultipleNewOperatorsAndSingleScalar() {
    multiple_new_operators(None, None, &[("status", None)]).await;
}

#[tokio::test]
async fn testShouldSupportMultipleNewOperatorsAndSingleScalarWithAliases() {
    multiple_new_operators(
        Some("cmsUser"),
        Some("cmsAddress"),
        &[("status", Some("cmsUserStatus"))],
    )
    .await;
}

#[tokio::test]
async fn testShouldSupportMultipleNewOperatorsAndSingleScalarWithAndWithoutAliases() {
    multiple_new_operators(Some("cmsUser"), None, &[("status", None)]).await;
}

#[tokio::test]
async fn testShouldSupportMultipleNewOperatorsAndMultipleScalars() {
    multiple_new_operators(None, None, &[("status", None), ("username", None)]).await;
}

#[tokio::test]
async fn testShouldSupportMultipleNewOperatorsAndMultipleScalarsWithAliases() {
    multiple_new_operators(
        Some("cmsUser"),
        Some("cmsAddress"),
        &[
            ("status", Some("cmsUserStatus")),
            ("username", Some("cmsUserUsername")),
        ],
    )
    .await;
}

#[tokio::test]
async fn testShouldSupportMultipleNewOperatorsAndMultipleScalarsWithAndWithoutAliases() {
    multiple_new_operators(
        Some("cmsUser"),
        None,
        &[("status", None), ("username", Some("cmsUserUsername"))],
    )
    .await;
}

async fn expect_query_exception(dql: &str, message: &str) {
    let (em, _fixtures) = set_up().await;
    let e = em
        .create_query(dql)
        .get_result(HYDRATE_OBJECT)
        .await
        .unwrap_err();
    assert_eq!(e.class, QUERY_EXCEPTION);
    assert_eq!(orm::message(&e), message);
}

#[tokio::test]
async fn testInvalidClassException() {
    expect_query_exception(
        "SELECT new \\InvalidClass(u.name) FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u",
        "[Semantical Error] line 0, col 11 near '\\InvalidClass(u.name)': Error: Class \"\\InvalidClass\" is not defined.",
    )
    .await;
}

#[tokio::test]
async fn testInvalidClassConstructorException() {
    expect_query_exception(
        "SELECT new \\stdClass(u.name) FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u",
        "[Semantical Error] line 0, col 11 near '\\stdClass(u.name)': Error: Class \"\\stdClass\" has not a valid constructor.",
    )
    .await;
}

#[tokio::test]
async fn testInvalidClassWithoutConstructorException() {
    expect_query_exception(
        "SELECT new Doctrine\\Tests\\ORM\\Functional\\ClassWithTooMuchArgs(u.name) FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u",
        "[Semantical Error] line 0, col 11 near 'Doctrine\\Tests\\ORM\\Functional\\ClassWithTooMuchArgs(u.name)': Error: Number of arguments does not match with \"Doctrine\\Tests\\ORM\\Functional\\ClassWithTooMuchArgs\" constructor declaration.",
    )
    .await;
}

#[tokio::test]
async fn testClassCantBeInstantiatedException() {
    expect_query_exception(
        "SELECT new Doctrine\\Tests\\ORM\\Functional\\ClassWithPrivateConstructor(u.name) FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u",
        "[Semantical Error] line 0, col 11 near 'Doctrine\\Tests\\ORM\\Functional\\ClassWithPrivateConstructor(u.name)': Error: Class \"Doctrine\\Tests\\ORM\\Functional\\ClassWithPrivateConstructor\" can not be instantiated.",
    )
    .await;
}

/// `assertSame()`.
#[track_caller]
fn assert_same(expected: &Value, actual: &Value) {
    assert_eq!(expected, actual);
}

const FROM_USER_EMAIL_ADDRESS: &str = "
            FROM
                Doctrine\\Tests\\Models\\CMS\\CmsUser u
            JOIN
                u.email e
            JOIN
                u.address a
            ORDER BY
                u.name";

const FROM_USER_LEFT_EMAIL_ADDRESS: &str = "
            FROM
                Doctrine\\Tests\\Models\\CMS\\CmsUser u
            LEFT JOIN
                u.email e
            LEFT JOIN
                u.address a
            ORDER BY
                u.name";

#[tokio::test]
async fn testShouldSupportNestedNewOperators() {
    let (em, fixtures) = set_up().await;
    let dql = format!(
        "
            SELECT
                new CmsUserDTO(
                    u.name,
                    e.email,
                    new CmsAddressDTO(
                        a.country,
                        a.city,
                        a.zip,
                        'Abbey Road',
                        new CmsDumbDTO(
                            a.country,
                            a.city,
                            a.zip
                        )
                    )
                ) as user,
                u.status,
                u.username as cmsUserUsername{FROM_USER_EMAIL_ADDRESS}"
    );

    let result = get_result(&em, &dql, HYDRATE_OBJECT).await;

    assert_count(3, &result);

    for (i, f) in fixtures.iter().enumerate() {
        let row = at(&result, idx(i));
        let user = at(&row, "user");
        assert_instance_of(CMS_USER_DTO, &user);
        let address = prop(&user, "address");
        assert_instance_of(CMS_ADDRESS_DTO, &address);
        assert_same(&f.name, &prop(&user, "name"));
        assert_same(&f.email, &prop(&user, "email"));
        assert_same(&f.city, &prop(&address, "city"));
        assert_same(&f.country, &prop(&address, "country"));
        let other = prop(&address, "other");
        assert_instance_of(CMS_DUMB_DTO, &other);
        assert_same(&f.country, &prop(&other, "val1"));
        assert_same(&f.city, &prop(&other, "val2"));
        assert_same(&f.status, &at(&row, "status"));
        assert_same(&f.username, &at(&row, "cmsUserUsername"));
    }
}

#[tokio::test]
async fn testShouldSupportNestedNewOperatorsWithDtoFirst() {
    let (em, fixtures) = set_up().await;
    let dql = format!(
        "
            SELECT
                new CmsUserDTO(
                    u.name,
                    e.email,
                    new CmsAddressDTO(
                        a.country,
                        a.city,
                        a.zip
                    ),
                    555812452
                ) as user,
                u.status,
                u.username as cmsUserUsername{FROM_USER_EMAIL_ADDRESS}"
    );

    let result = get_result(&em, &dql, HYDRATE_OBJECT).await;

    assert_count(3, &result);

    for (i, f) in fixtures.iter().enumerate() {
        let row = at(&result, idx(i));
        let user = at(&row, "user");
        assert_instance_of(CMS_USER_DTO, &user);
        let address = prop(&user, "address");
        assert_instance_of(CMS_ADDRESS_DTO, &address);
        assert_same(&f.name, &prop(&user, "name"));
        assert_same(&f.email, &prop(&user, "email"));
        assert_same(&f.city, &prop(&address, "city"));
        assert_same(&f.country, &prop(&address, "country"));
        assert_same(&int(555_812_452), &prop(&user, "phonenumbers"));
        assert_same(&f.status, &at(&row, "status"));
        assert_same(&f.username, &at(&row, "cmsUserUsername"));
    }
}

#[tokio::test]
async fn testOnlyObjectInObject() {
    let (em, fixtures) = set_up().await;
    let dql = format!(
        "
            SELECT
                new CmsDumbDTO(
                    new CmsDumbDTO(
                        u.name,
                        e.email
                    ),
                    new CmsAddressDTO(
                        a.country,
                        a.city,
                        a.zip
                    )
                ) as user{FROM_USER_EMAIL_ADDRESS}"
    );

    let result = get_result(&em, &dql, HYDRATE_OBJECT).await;

    assert_count(3, &result);

    for (i, f) in fixtures.iter().enumerate() {
        let dto = at(&result, idx(i));
        assert_instance_of(CMS_DUMB_DTO, &dto);
        let val1 = prop(&dto, "val1");
        assert_instance_of(CMS_DUMB_DTO, &val1);
        assert_same(&f.name, &prop(&val1, "val1"));
        assert_same(&f.email, &prop(&val1, "val2"));
        let val2 = prop(&dto, "val2");
        assert_instance_of(CMS_ADDRESS_DTO, &val2);
        assert_same(&f.country, &prop(&val2, "country"));
        assert_same(&f.city, &prop(&val2, "city"));
        assert_same(&f.zip, &prop(&val2, "zip"));
    }
}

#[tokio::test]
async fn testEntityInDtoWithRoot() {
    let (em, fixtures) = set_up().await;
    let dql = format!(
        "
            SELECT
                new CmsDumbDTO(
                    u.id,
                    u,
                    a,
                    e.email
                ){FROM_USER_LEFT_EMAIL_ADDRESS}"
    );

    let result = get_result(&em, &dql, HYDRATE_OBJECT).await;

    assert_count(3, &result);

    for (i, f) in fixtures.iter().enumerate() {
        let dto = at(&result, idx(i));
        assert_instance_of(CMS_DUMB_DTO, &dto);
        let user = prop(&dto, "val2");
        assert_instance_of(CMS_USER, &user);
        assert_same(&f.name, &prop(&user, "name"));
        assert_same(&f.username, &prop(&user, "username"));
        assert_same(&f.status, &prop(&user, "status"));
        let address = prop(&dto, "val3");
        assert_instance_of(CMS_ADDRESS, &address);
        assert_same(&f.city, &prop(&address, "city"));
        assert_same(&f.country, &prop(&address, "country"));
        assert_same(&f.email, &prop(&dto, "val4"));
    }
}

#[tokio::test]
async fn testEntityInDtoWithoutRoot() {
    let (em, fixtures) = set_up().await;
    let dql = format!(
        "
            SELECT
                new CmsDumbDTO(
                    u.id,
                    u.name,
                    a,
                    e.email
                ){FROM_USER_LEFT_EMAIL_ADDRESS}"
    );

    let result = get_result(&em, &dql, HYDRATE_OBJECT).await;

    assert_count(3, &result);

    for (i, f) in fixtures.iter().enumerate() {
        let dto = at(&result, idx(i));
        assert_instance_of(CMS_DUMB_DTO, &dto);
        assert_same(&f.name, &prop(&dto, "val2"));
        let address = prop(&dto, "val3");
        assert_instance_of(CMS_ADDRESS, &address);
        assert_same(&f.city, &prop(&address, "city"));
        assert_same(&f.country, &prop(&address, "country"));
        assert_same(&f.email, &prop(&dto, "val4"));
    }
}

#[tokio::test]
async fn testOnlyObjectInDto() {
    let (em, fixtures) = set_up().await;
    let dql = format!(
        "
            SELECT
                new CmsDumbDTO(
                    a,
                    new CmsDumbDTO(
                        u.name,
                        e.email
                    )
                ){FROM_USER_LEFT_EMAIL_ADDRESS}"
    );

    let result = get_result(&em, &dql, HYDRATE_OBJECT).await;

    assert_count(3, &result);

    for (i, f) in fixtures.iter().enumerate() {
        let dto = at(&result, idx(i));
        assert_instance_of(CMS_DUMB_DTO, &dto);
        let address = prop(&dto, "val1");
        assert_instance_of(CMS_ADDRESS, &address);
        assert_same(&f.city, &prop(&address, "city"));
        assert_same(&f.country, &prop(&address, "country"));
        let val2 = prop(&dto, "val2");
        assert_instance_of(CMS_DUMB_DTO, &val2);
        assert_same(&f.name, &prop(&val2, "val1"));
        assert_same(&f.email, &prop(&val2, "val2"));
    }
}

/// `testOnlyObjectInNamedDto()` and `testOnlyObjectInNamedDtoWithAlias()`.
async fn only_object_in_named_dto(entity: &str, key: &str) {
    let (em, fixtures) = set_up().await;
    let dql = format!(
        "
            SELECT
                new named CmsUserDTOVariadicArg(
                    {entity},
                    new CmsDumbDTO(
                        u.name,
                        e.email
                    ) as dumb
                ){FROM_USER_LEFT_EMAIL_ADDRESS}"
    );

    let result = get_result(&em, &dql, HYDRATE_OBJECT).await;

    assert_count(3, &result);

    for (i, f) in fixtures.iter().enumerate() {
        let dto = at(&result, idx(i));
        assert_instance_of(CMS_USER_DTO_VARIADIC_ARG, &dto);
        let other_properties = prop(&dto, "otherProperties");
        let address = at(&other_properties, key);
        assert_instance_of(CMS_ADDRESS, &address);
        assert_same(&f.city, &prop(&address, "city"));
        assert_same(&f.country, &prop(&address, "country"));
        let dumb = at(&other_properties, "dumb");
        assert_instance_of(CMS_DUMB_DTO, &dumb);
        assert_same(&f.name, &prop(&dumb, "val1"));
        assert_same(&f.email, &prop(&dumb, "val2"));
    }
}

#[tokio::test]
async fn testOnlyObjectInNamedDto() {
    only_object_in_named_dto("a", "a").await;
}

#[tokio::test]
async fn testOnlyObjectInNamedDtoWithAlias() {
    only_object_in_named_dto("a as addr", "addr").await;
}

#[tokio::test]
async fn testOnlyObjectInNamedDtoWithSameNameAsTheProperties() {
    let (em, fixtures) = set_up().await;
    let dql = "
            SELECT
                new named CmsUserDTONamedArgs(
                    addressEntity,
                    new CmsDumbDTO(
                        u.name,
                        e.email
                    ) as dumb
                )
            FROM
                Doctrine\\Tests\\Models\\CMS\\CmsUser u
            LEFT JOIN
                u.email e
            LEFT JOIN
                u.address addressEntity
            ORDER BY
                u.name";

    let result = get_result(&em, dql, HYDRATE_OBJECT).await;

    assert_count(3, &result);

    for (i, f) in fixtures.iter().enumerate() {
        let dto = at(&result, idx(i));
        assert_instance_of(CMS_USER_DTO_NAMED_ARGS, &dto);
        let address = prop(&dto, "addressEntity");
        assert_instance_of(CMS_ADDRESS, &address);
        assert_same(&f.city, &prop(&address, "city"));
        assert_same(&f.country, &prop(&address, "country"));
        let dumb = prop(&dto, "dumb");
        assert_instance_of(CMS_DUMB_DTO, &dumb);
        assert_same(&f.name, &prop(&dumb, "val1"));
        assert_same(&f.email, &prop(&dumb, "val2"));
    }
}

/// `testNamedArguments()` and `testVariadicArgument()`.
async fn named_arguments(class: &str, arguments: &str) {
    let (em, fixtures) = set_up().await;
    let dql = format!(
        "
            SELECT
                new named {}(
                    {arguments}
                ) as user,
                u.status,
                u.username as cmsUserUsername{FROM_USER_EMAIL_ADDRESS}",
        class.rsplit('\\').next().unwrap()
    );

    let result = get_result(&em, &dql, HYDRATE_OBJECT).await;

    assert_count(3, &result);

    for (i, f) in fixtures.iter().enumerate() {
        let row = at(&result, idx(i));
        let user = at(&row, "user");
        assert_instance_of(class, &user);
        assert_same(&f.name, &prop(&user, "name"));
        assert_same(&f.email, &prop(&user, "email"));
        assert_same(&country_city_zip(f), &prop(&user, "address"));
        assert_same(&f.status, &at(&row, "status"));
        assert_same(&f.username, &at(&row, "cmsUserUsername"));
    }
}

#[tokio::test]
async fn testNamedArguments() {
    named_arguments(
        CMS_USER_DTO_NAMED_ARGS,
        "e.email,
                    u.name,
                    CONCAT(a.country, ' ', a.city, ' ', a.zip) AS address",
    )
    .await;
}

#[tokio::test]
async fn testShouldSupportNestedNewOperatorsWithNestedDtoNotLast() {
    let (em, fixtures) = set_up().await;
    let dql = format!(
        "
            SELECT
                new CmsUserDTO(
                    u.name,
                    e.email,
                    new CmsAddressDTO(
                        a.country,
                        a.city,
                        a.zip,
                        'Abbey Road',
                        new CmsDumbDTO(
                            a.country,
                            a.city,
                            new CmsDumbDTO(
                                a.zip,
                                456
                            ),
                            a.zip
                        )
                    ),
                    555812452
                ) as user,
                u.status,
                u.username as cmsUserUsername{FROM_USER_EMAIL_ADDRESS}"
    );

    let result = get_result(&em, &dql, HYDRATE_OBJECT).await;

    assert_count(3, &result);

    for (i, f) in fixtures.iter().enumerate() {
        let row = at(&result, idx(i));
        let user = at(&row, "user");
        assert_instance_of(CMS_USER_DTO, &user);
        assert_same(&f.name, &prop(&user, "name"));
        assert_same(&f.email, &prop(&user, "email"));
        let address = prop(&user, "address");
        assert_instance_of(CMS_ADDRESS_DTO, &address);
        assert_same(&f.country, &prop(&address, "country"));
        assert_same(&f.city, &prop(&address, "city"));
        let other = prop(&address, "other");
        assert_instance_of(CMS_DUMB_DTO, &other);
        assert_same(&f.country, &prop(&other, "val1"));
        assert_same(&f.city, &prop(&other, "val2"));
        let val3 = prop(&other, "val3");
        assert_instance_of(CMS_DUMB_DTO, &val3);
        assert_same(&f.zip, &prop(&val3, "val1"));
        assert_same(&int(456), &prop(&val3, "val2"));
        assert_same(&f.zip, &prop(&other, "val4"));
        assert_same(&int(555_812_452), &prop(&user, "phonenumbers"));
        assert_same(&f.status, &at(&row, "status"));
        assert_same(&f.username, &at(&row, "cmsUserUsername"));
    }
}

#[tokio::test]
async fn testVariadicArgument() {
    named_arguments(
        CMS_USER_DTO_VARIADIC_ARG,
        "CONCAT(a.country, ' ', a.city, ' ', a.zip) AS address,
                    e.email,
                    u.name",
    )
    .await;
}

#[tokio::test]
async fn testShouldSupportNestedNewOperatorsAndNamedArguments() {
    let (em, fixtures) = set_up().await;
    let dql = format!(
        "
            SELECT
                new named CmsUserDTONamedArgs(
                    e.email,
                    u.name as name,
                    new CmsAddressDTO(
                        a.country,
                        a.city,
                        a.zip
                    ) as addressDto
                ) as user,
                u.status,
                u.username as cmsUserUsername{FROM_USER_EMAIL_ADDRESS}"
    );

    let result = get_result(&em, &dql, HYDRATE_OBJECT).await;

    assert_count(3, &result);

    for (i, f) in fixtures.iter().enumerate() {
        let row = at(&result, idx(i));
        let user = at(&row, "user");
        assert_instance_of(CMS_USER_DTO_NAMED_ARGS, &user);
        assert!(prop(&user, "address").is_null());
        let address_dto = prop(&user, "addressDto");
        assert_instance_of(CMS_ADDRESS_DTO, &address_dto);
        assert_same(&f.name, &prop(&user, "name"));
        assert_same(&f.email, &prop(&user, "email"));
        assert_same(&f.city, &prop(&address_dto, "city"));
        assert_same(&f.country, &prop(&address_dto, "country"));
        assert_same(&f.status, &at(&row, "status"));
        assert_same(&f.username, &at(&row, "cmsUserUsername"));
    }
}

#[tokio::test]
async fn testShouldSupportNestedNamedArguments() {
    let (em, fixtures) = set_up().await;
    let dql = format!(
        "
            SELECT
                new named CmsUserDTONamedArgs(
                    e.email,
                    u.name as name,
                    new named CmsAddressDTONamedArgs(
                        a.zip,
                        a.city,
                        a.country
                    ) as addressDtoNamedArgs
                ) as user,
                u.status,
                u.username as cmsUserUsername{FROM_USER_EMAIL_ADDRESS}"
    );

    let result = get_result(&em, &dql, HYDRATE_OBJECT).await;

    assert_count(3, &result);

    for (i, f) in fixtures.iter().enumerate() {
        let row = at(&result, idx(i));
        let user = at(&row, "user");
        assert_instance_of(CMS_USER_DTO_NAMED_ARGS, &user);
        assert!(prop(&user, "address").is_null());
        assert!(prop(&user, "addressDto").is_null());
        let address = prop(&user, "addressDtoNamedArgs");
        assert_instance_of(CMS_ADDRESS_DTO_NAMED_ARGS, &address);
        assert_same(&f.name, &prop(&user, "name"));
        assert_same(&f.email, &prop(&user, "email"));
        assert_same(&f.city, &prop(&address, "city"));
        assert_same(&f.country, &prop(&address, "country"));
        assert_same(&f.status, &at(&row, "status"));
        assert_same(&f.username, &at(&row, "cmsUserUsername"));
    }
}

#[tokio::test]
async fn testExceptionIfTwoAliases() {
    let (em, _fixtures) = set_up().await;
    let dql = "
            SELECT
                new named Doctrine\\Tests\\Models\\CMS\\CmsUserDTO(
                    u.name,
                    u.username AS name
                )
            FROM
                Doctrine\\Tests\\Models\\CMS\\CmsUser u";

    let e = em
        .create_query(dql)
        .get_result(HYDRATE_OBJECT)
        .await
        .unwrap_err();
    assert_eq!(e.class, "Doctrine\\ORM\\Exception\\DuplicateFieldException");
    assert_eq!(
        orm::message(&e),
        "Name \"name\" for \"u.username AS name\" already in use."
    );
}

#[tokio::test]
async fn testExceptionIfFunctionHasNoAlias() {
    let (em, _fixtures) = set_up().await;
    let dql = "
            SELECT
                new named Doctrine\\Tests\\Models\\CMS\\CmsUserDTO(
                    u.name,
                    CASE WHEN (e.email = 'email@test1.com') THEN 'TEST1' ELSE 'OTHER_TEST' END
                )
            FROM
                Doctrine\\Tests\\Models\\CMS\\CmsUser u
            JOIN
                u.email e";

    let e = em
        .create_query(dql)
        .get_result(HYDRATE_OBJECT)
        .await
        .unwrap_err();
    assert_eq!(
        e.class,
        "Doctrine\\ORM\\Exception\\NoMatchingPropertyException"
    );
    assert_eq!(
        orm::message(&e),
        "Column name \"CASE WHEN (e.email = 'email@test1.com') THEN 'TEST1' ELSE 'OTHER_TEST' END\" does not match any property name. Consider aliasing it to the name of an existing property."
    );
}
