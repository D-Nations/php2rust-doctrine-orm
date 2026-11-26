//! `Cursor`, `CursorItem` and `InvalidCursor`: a cursor-based pagination
//! position, an item with its cursor, and the error of a cursor that does
//! not decode.
//!
//! `InvalidCursor` keeps the `JsonException` it chains (`getPrevious()`)
//! next to the `Thrown` it converts to, which has no previous exception.
//!
//! Upstream: doctrine/orm src/Tools/Pagination/Cursor.php src/Tools/Pagination/CursorItem.php src/Tools/Pagination/Exception/InvalidCursor.php

use std::sync::Arc;

use php_value::json::{decode_assoc, encode};
use php_value::{Key, PhpArray, PhpObject, PhpString, Thrown, Value, base64};

/// `Exception\InvalidCursor` (an `InvalidArgumentException`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidCursor {
    thrown: Thrown,
    previous: Option<Thrown>,
}

impl InvalidCursor {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Tools\\Pagination\\Exception\\InvalidCursor";

    /// `new InvalidCursor($cursor, $previous)`.
    pub fn new(cursor: &[u8], previous: Option<Thrown>) -> Self {
        let mut message = b"The cursor \"".to_vec();
        message.extend_from_slice(cursor);
        message.extend_from_slice(b"\" could not be decoded.");
        Self {
            thrown: Thrown::new(Self::CLASS, PhpString::from(message), 0),
            previous,
        }
    }

    pub fn get_message(&self) -> &PhpString {
        &self.thrown.message
    }

    /// `getPrevious()`.
    pub fn get_previous(&self) -> Option<&Thrown> {
        self.previous.as_ref()
    }
}

impl From<InvalidCursor> for Thrown {
    fn from(e: InvalidCursor) -> Self {
        e.thrown
    }
}

/// `strtr($text, $from, $to)` for two single-byte pairs.
fn swap_bytes(text: &[u8], from: [u8; 2], to: [u8; 2]) -> Vec<u8> {
    text.iter()
        .map(|&b| {
            if b == from[0] {
                to[0]
            } else if b == from[1] {
                to[1]
            } else {
                b
            }
        })
        .collect()
}

/// Represents a cursor for cursor-based pagination.
///
/// A cursor contains the parameters needed to fetch the next or previous page of results.
#[derive(Debug, Clone)]
pub struct Cursor {
    parameters: PhpArray,
    is_next: bool,
}

impl Cursor {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Tools\\Pagination\\Cursor";

    /// `new Cursor($parameters, $isNext)` (`$isNext` is `true` by default).
    pub fn new(parameters: PhpArray, is_next: bool) -> Self {
        Self {
            parameters,
            is_next,
        }
    }

    /// `getParameters()` (`@internal`).
    pub fn get_parameters(&self) -> &PhpArray {
        &self.parameters
    }

    /// Returns whether the cursor is for navigating to the next page.
    pub fn is_next(&self) -> bool {
        self.is_next
    }

    /// Returns whether the cursor is for navigating to the previous page.
    pub fn is_previous(&self) -> bool {
        !self.is_next
    }

    /// `toArray()`: `[...$this->parameters, '_isNext' => $this->isNext]`.
    pub fn to_array(&self) -> PhpArray {
        // Spreading renumbers the integer keys, as `[...$array]` does.
        let mut array = PhpArray::new();
        for (key, value) in self.parameters.iter() {
            match key {
                Key::Int(_) => {
                    // The next index of a new array cannot be occupied.
                    let _ = array.push(value.clone());
                }
                Key::Str(_) => {
                    array.insert(key.clone(), value.clone());
                }
            }
        }
        array.insert("_isNext", Value::Bool(self.is_next));
        array
    }

    /// Encodes the cursor to a URL-safe Base64 JSON string.
    pub fn encode_to_string(&self) -> String {
        // `(string) json_encode(...)`: false is "".
        let json = encode(&Value::Array(self.to_array()), 0, 512)
            .ok()
            .and_then(|encoded| encoded.json)
            .unwrap_or_default();
        let encoded = base64::encode(json.as_bytes());
        let swapped = swap_bytes(encoded.as_bytes(), *b"+/", *b"-_");
        String::from_utf8_lossy(&swapped)
            .trim_end_matches('=')
            .to_owned()
    }

    /// Decodes a cursor from an encoded string.
    ///
    /// See CursorWalker::buildCursorCondition() for the security model around
    /// cursor manipulation.
    pub fn from_encoded_string(encoded_string: &Value) -> Result<Self, InvalidCursor> {
        let Value::String(encoded_string) = encoded_string else {
            return Err(InvalidCursor::new(b"(non-string value)", None));
        };
        let encoded_string = encoded_string.as_bytes();

        let Some(decoded) = base64::decode_strict(&swap_bytes(encoded_string, *b"-_", *b"+/"))
        else {
            return Err(InvalidCursor::new(encoded_string, None));
        };

        let parameters = decode_assoc(&decoded, 2, false).map_err(|e| {
            InvalidCursor::new(
                encoded_string,
                Some(Thrown::new("JsonException", e.message(), e.code())),
            )
        })?;

        let Value::Array(mut parameters) = parameters else {
            return Err(InvalidCursor::new(encoded_string, None));
        };

        let is_next = match parameters.get(&Key::from("_isNext")) {
            None | Some(Value::Null) => true,
            Some(value) => value.is_truthy(),
        };

        parameters.remove(&Key::from("_isNext"));

        for value in parameters.values() {
            if matches!(value, Value::Array(_) | Value::Object(_)) {
                return Err(InvalidCursor::new(encoded_string, None));
            }
        }

        Ok(Self::new(parameters, is_next))
    }
}

impl PhpObject for Cursor {
    fn class(&self) -> &str {
        Self::CLASS
    }

    fn properties(&self) -> PhpArray {
        // The properties are private.
        PhpArray::new()
    }
}

impl From<Cursor> for Value {
    fn from(cursor: Cursor) -> Self {
        Value::Object(Arc::new(cursor))
    }
}

/// Represents a paginated item with its associated cursor.
#[derive(Debug, Clone)]
pub struct CursorItem {
    value: Value,
    cursor: Cursor,
}

impl CursorItem {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Tools\\Pagination\\CursorItem";

    pub fn new(value: Value, cursor: Cursor) -> Self {
        Self { value, cursor }
    }

    pub fn get_value(&self) -> &Value {
        &self.value
    }

    pub fn get_cursor(&self) -> &Cursor {
        &self.cursor
    }
}
