//! `Expr\Base`, the parts list its subclasses share, and the macro that
//! declares each subclass.
//!
//! Upstream: doctrine/orm src/Query/Expr/Base.php

use std::sync::{Mutex, PoisonError};

use php_value::{Key, PhpArray, PhpString, Thrown, Value};

use super::php;

/// What a subclass of `Base` declares: its separators and the classes it
/// accepts as parts.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BaseKind {
    pub(crate) class: &'static str,
    pub(crate) pre_separator: &'static str,
    pub(crate) separator: &'static str,
    pub(crate) post_separator: &'static str,
    pub(crate) allowed_classes: &'static [&'static str],
    /// A subclass of `Composite`.
    pub(crate) composite: bool,
}

/// Abstract base Expr class for building DQL parts: the parts, behind a
/// lock so that an expression stored as a `php_value` object (a handle,
/// as in PHP) can still be added to.
#[derive(Debug)]
pub struct Base {
    kind: BaseKind,
    parts: Mutex<Vec<Value>>,
}

impl Clone for Base {
    fn clone(&self) -> Self {
        Self {
            kind: self.kind,
            parts: Mutex::new(self.parts()),
        }
    }
}

impl PartialEq for Base {
    fn eq(&self, other: &Self) -> bool {
        self.kind.class == other.kind.class && {
            let (a, b) = (self.parts(), other.parts());
            a.len() == b.len()
                && a.iter()
                    .zip(&b)
                    .all(|(x, y)| php_value::compare::loose_equals(x, y))
        }
    }
}

impl Base {
    /// `__construct($args)`: an array whose element 0 is an array stands
    /// for that array.
    pub(crate) fn new(kind: BaseKind, args: Value) -> Result<Self, Thrown> {
        let base = Self {
            kind,
            parts: Mutex::new(Vec::new()),
        };
        let args = match &args {
            Value::Array(array) => match array.get(&Key::from(0)) {
                Some(first @ Value::Array(_)) => first.clone(),
                _ => args,
            },
            _ => args,
        };
        base.add_multiple(&args)?;
        Ok(base)
    }

    /// `addMultiple($args)`: each element of `(array) $args`.
    pub(crate) fn add_multiple(&self, args: &Value) -> Result<(), Thrown> {
        if !matches!(args, Value::Array(_) | Value::String(_) | Value::Object(_)) {
            return Err(Thrown::new(
                "TypeError",
                format!(
                    "Doctrine\\ORM\\Query\\Expr\\Base::addMultiple(): Argument #1 ($args) must be of type object|array|string, {} given",
                    args.value_name()
                ),
                0,
            ));
        }
        for arg in php::to_array(args).values() {
            self.add(arg.clone())?;
        }
        Ok(())
    }

    /// `add($arg)`: `null` and an empty expression are skipped; anything
    /// but a string or an object of an allowed class is refused.
    pub(crate) fn add(&self, arg: Value) -> Result<(), Thrown> {
        let empty_expression = arg
            .as_object()
            .and_then(|o| base_of(o.as_ref()))
            .is_some_and(|b| b.count() == 0);
        if arg.is_null() || empty_expression {
            return Ok(());
        }
        let allowed = match &arg {
            Value::String(_) => true,
            Value::Object(o) => self.kind.allowed_classes.contains(&o.class()),
            _ => false,
        };
        if !allowed {
            return Err(php::invalid_argument(format!(
                "Expression of type '{}' not allowed in this context.",
                arg.debug_type()
            )));
        }
        self.parts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(arg);
        Ok(())
    }

    /// `count()`.
    pub(crate) fn count(&self) -> usize {
        self.parts.lock().unwrap_or_else(PoisonError::into_inner).len()
    }

    /// The parts (`getParts()` of the subclasses that have it).
    pub(crate) fn parts(&self) -> Vec<Value> {
        self.parts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// `(array)`: `preSeparator`, `separator`, `postSeparator`,
    /// `allowedClasses`, `parts`.
    pub(crate) fn array_cast(&self) -> PhpArray {
        let kind = self.kind;
        let mut array = PhpArray::new();
        array.insert(php::protected("preSeparator"), Value::from(kind.pre_separator));
        array.insert(php::protected("separator"), Value::from(kind.separator));
        array.insert(php::protected("postSeparator"), Value::from(kind.post_separator));
        array.insert(
            php::protected("allowedClasses"),
            Value::Array(PhpArray::list(kind.allowed_classes.iter().map(|c| Value::from(*c)))),
        );
        array.insert(php::protected("parts"), Value::Array(PhpArray::list(self.parts())));
        array
    }

    /// `__toString()`: `Base`'s, or `Composite`'s for its subclasses.
    pub(crate) fn try_to_string(&self) -> Result<PhpString, Thrown> {
        let parts = self.parts();
        if let [only] = parts.as_slice() {
            return php::to_string(only);
        }
        let kind = self.kind;
        if kind.composite {
            let mut components = Vec::with_capacity(parts.len());
            for part in &parts {
                components.push(Value::String(self.process_query_part(part)?));
            }
            return php::implode(kind.separator, &components);
        }
        Ok(php::concat(&[
            kind.pre_separator.as_bytes(),
            php::implode(kind.separator, &parts)?.as_bytes(),
            kind.post_separator.as_bytes(),
        ]))
    }

    /// `Composite::processQueryPart()`.
    fn process_query_part(&self, part: &Value) -> Result<PhpString, Thrown> {
        let query_part = php::to_string(part)?;
        let nested_composite = part
            .as_object()
            .and_then(|o| base_of(o.as_ref()))
            .is_some_and(|b| b.kind.composite && b.count() > 1);
        // Fixes DDC-1237: User may have added a where item containing nested expression (with "OR" or "AND")
        if nested_composite || has_spaced_or_and(query_part.as_bytes()) {
            return Ok(php::concat(&[
                self.kind.pre_separator.as_bytes(),
                query_part.as_bytes(),
                self.kind.post_separator.as_bytes(),
            ]));
        }
        Ok(query_part)
    }
}

/// `preg_match('/\s(OR|AND)\s/i', $part)`: `\s` is PCRE's white space
/// without Unicode (tab, line feed, vertical tab, form feed, carriage
/// return, space).
fn has_spaced_or_and(s: &[u8]) -> bool {
    let space = |b: u8| matches!(b, b'\t' | b'\n' | 0x0B | 0x0C | b'\r' | b' ');
    (0..s.len()).any(|i| {
        space(s[i])
            && [&b"or"[..], &b"and"[..]].iter().any(|word| {
                let end = i + 1 + word.len();
                s.len() > end
                    && s[i + 1..end].eq_ignore_ascii_case(word)
                    && space(s[end])
            })
    })
}

/// The `Base` of an expression object, if it is a subclass of `Base`.
pub(crate) fn base_of(object: &dyn php_value::PhpObject) -> Option<&Base> {
    macro_rules! downcast {
        ($($class:ty),*) => {
            $(if let Some(e) = object.downcast_ref::<$class>() {
                return Some(&e.base);
            })*
        };
    }
    downcast!(super::Andx, super::Orx, super::Composite, super::Select, super::GroupBy, super::Literal);
    None
}

/// Declares a subclass of `Base`: its constructor, `add()`,
/// `addMultiple()`, `count()`, `getParts()` (where upstream has it) and
/// `__toString()`, as a `php_value` object.
macro_rules! base_class {
    (
        $(#[$doc:meta])*
        $name:ident, $kind:expr, parents: [$($parent:literal),*], get_parts: $get_parts:tt
    ) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq)]
        pub struct $name {
            pub(crate) base: $crate::query::expr::base::Base,
        }

        impl $name {
            pub const CLASS: &'static str = $kind.class;

            /// `new Class($args)`.
            pub fn new(args: impl Into<php_value::Value>) -> Result<Self, php_value::Thrown> {
                Ok(Self {
                    base: $crate::query::expr::base::Base::new($kind, args.into())?,
                })
            }

            /// `addMultiple($args)`.
            pub fn add_multiple(&self, args: impl Into<php_value::Value>) -> Result<&Self, php_value::Thrown> {
                self.base.add_multiple(&args.into())?;
                Ok(self)
            }

            /// `add($arg)`.
            pub fn add(&self, arg: impl Into<php_value::Value>) -> Result<&Self, php_value::Thrown> {
                self.base.add(arg.into())?;
                Ok(self)
            }

            /// `count()`.
            pub fn count(&self) -> usize {
                self.base.count()
            }

            base_class!(@get_parts $get_parts);

            /// `__toString()`.
            pub fn try_to_string(&self) -> Result<php_value::PhpString, php_value::Thrown> {
                self.base.try_to_string()
            }
        }

        impl $crate::query::expr::DqlExpression for $name {
            fn try_to_string(&self) -> Result<php_value::PhpString, php_value::Thrown> {
                self.base.try_to_string()
            }

            fn array_cast(&self) -> php_value::PhpArray {
                self.base.array_cast()
            }
        }

        impl php_value::PhpObject for $name {
            fn class(&self) -> &str {
                Self::CLASS
            }

            fn properties(&self) -> php_value::PhpArray {
                php_value::PhpArray::new()
            }

            fn to_php_string(&self) -> Option<php_value::PhpString> {
                self.base.try_to_string().ok()
            }

            fn is_stringable(&self) -> bool {
                true
            }

            fn instance_of(&self, class: &str) -> bool {
                [$($parent,)* "Doctrine\\ORM\\Query\\Expr\\Base", "Stringable"]
                    .iter()
                    .any(|parent| parent.eq_ignore_ascii_case(class))
            }
        }

        impl ::core::convert::From<$name> for php_value::Value {
            fn from(expression: $name) -> Self {
                php_value::Value::Object(std::sync::Arc::new(expression))
            }
        }
    };
    (@get_parts true) => {
        /// `getParts()`.
        pub fn get_parts(&self) -> Vec<php_value::Value> {
            self.base.parts()
        }
    };
    (@get_parts false) => {};
}

pub(crate) use base_class;
