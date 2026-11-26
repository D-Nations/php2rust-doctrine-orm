//! CursorTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! `testCursorIsFinal` is n/a: a Rust struct has no subclasses.
//! `Cursor::fromEncodedString()` takes the `mixed` value as a `Value`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Tools/Pagination/CursorTest.php

#![allow(clippy::unwrap_used, reason = "a panic is a test failure")]
#![allow(non_snake_case)]

use php_value::json::encode;
use php_value::{PhpArray, Value, base64};
use php2rust_doctrine_orm::tools::pagination::{Cursor, InvalidCursor};

fn params(entries: &[(&str, Value)]) -> PhpArray {
    let mut array = PhpArray::new();
    for (key, value) in entries {
        array.insert(*key, value.clone());
    }
    array
}

/// `json_encode($value)`.
fn json_encode(value: &Value) -> String {
    encode(value, 0, 512).unwrap().json.unwrap()
}

/// `rtrim(strtr(base64_encode($text), '+/', '-_'), '=')`.
fn url_safe(text: &str) -> Value {
    Value::from(
        base64::encode(text.as_bytes())
            .replace('+', "-")
            .replace('/', "_")
            .trim_end_matches('=')
            .to_owned(),
    )
}

#[test]
fn testEncodeAndDecode() {
    let parameters = params(&[("id", Value::Int(10)), ("name", Value::from("test"))]);
    let cursor = Cursor::new(parameters.clone(), true);

    let decoded = Cursor::from_encoded_string(&Value::from(cursor.encode_to_string())).unwrap();

    assert_eq!(&parameters, decoded.get_parameters());
    assert!(decoded.is_next());
}

#[test]
fn testEncodeAndDecodeWithPreviousCursor() {
    let cursor = Cursor::new(params(&[("id", Value::Int(10))]), false);

    let decoded = Cursor::from_encoded_string(&Value::from(cursor.encode_to_string())).unwrap();

    assert!(decoded.is_previous());
}

#[test]
fn testEncodeProducesUrlSafeString() {
    let cursor = Cursor::new(
        params(&[
            ("id", Value::Int(123_456_789)),
            ("foo", Value::from("bar/baz+test")),
        ]),
        true,
    );
    let encoded = cursor.encode_to_string();

    assert!(!encoded.contains('+'));
    assert!(!encoded.contains('/'));
    assert!(!encoded.contains('='));
}

#[test]
fn testFromEncodedStringThrowsForInvalidBase64() {
    let e = Cursor::from_encoded_string(&Value::from("not-valid-base64!@#$")).unwrap_err();
    assert_eq!(InvalidCursor::CLASS, php_value::Thrown::from(e).class);
}

#[test]
fn testFromEncodedStringThrowsForInvalidJson() {
    let e = Cursor::from_encoded_string(&url_safe("not-json")).unwrap_err();
    assert_eq!(InvalidCursor::CLASS, php_value::Thrown::from(e).class);
}

#[test]
fn testFromEncodedStringChainsPreviousExceptionForInvalidJson() {
    match Cursor::from_encoded_string(&url_safe("not-json")) {
        Ok(_) => panic!("Expected InvalidCursor to be thrown."),
        Err(e) => assert_eq!("JsonException", e.get_previous().unwrap().class),
    }
}

#[test]
fn testFromEncodedStringDefaultsToNextWhenIsNextMissing() {
    let json = json_encode(&Value::Array(params(&[("id", Value::Int(10))])));
    let encoded = url_safe(&json);

    let cursor = Cursor::from_encoded_string(&encoded).unwrap();

    assert!(cursor.is_next());
}

#[test]
fn testToArray() {
    let cursor = Cursor::new(params(&[("id", Value::Int(1))]), true);

    assert_eq!(
        params(&[("id", Value::Int(1)), ("_isNext", Value::Bool(true))]),
        cursor.to_array()
    );
}

#[test]
fn testFromEncodedStringThrowsForNonStringInput() {
    let e =
        Cursor::from_encoded_string(&Value::Array(params(&[("injected", Value::from("array"))])))
            .unwrap_err();
    assert_eq!(InvalidCursor::CLASS, php_value::Thrown::from(e).class);
}

#[test]
fn testFromEncodedStringThrowsForNonScalarParameterValue() {
    let payload = url_safe(&json_encode(&Value::Array(params(&[
        (
            "p.id",
            Value::Array(PhpArray::list([
                Value::Int(1),
                Value::Int(2),
                Value::Int(3),
            ])),
        ),
        ("_isNext", Value::Bool(true)),
    ]))));
    let e = Cursor::from_encoded_string(&payload).unwrap_err();
    assert_eq!(InvalidCursor::CLASS, php_value::Thrown::from(e).class);
}

#[test]
fn testFromEncodedStringCastsIsNextToBool() {
    let payload = url_safe(&json_encode(&Value::Array(params(&[
        ("id", Value::Int(10)),
        ("_isNext", Value::Int(0)),
    ]))));
    let cursor = Cursor::from_encoded_string(&payload).unwrap();

    assert!(!cursor.is_next());
    assert!(cursor.is_previous());
}

#[test]
fn testFromEncodedStringThrowsForNonArrayJson() {
    let payload = url_safe(&json_encode(&Value::from("just-a-string")));
    let e = Cursor::from_encoded_string(&payload).unwrap_err();
    assert_eq!(InvalidCursor::CLASS, php_value::Thrown::from(e).class);
}
