/// Zero-copy borrowed string value for use in a [`crate::Context`].
///
/// `StrRef<'a>` wraps a `&'a str` and implements [`Val`] without allocating.
/// It reports the same CEL type as [`super::string::String`] (`Kind::String`) and is
/// interoperable with `CelString` in all string operations: comparisons, concatenation,
/// `contains`/`starts_with`/`ends_with`, etc.
///
/// Because `'a` is not `'static`, `StrRef` **cannot** go through the `Any`-based
/// `downcast_ref` path.  All dispatch is handled through the `as_str_ref()` accessor
/// added to [`Val`].
///
/// # Example
/// ```ignore
/// use cel_interpreter::{Context, common::types::StrRef};
///
/// let greeting = String::from("hello, world");
/// let mut ctx = Context::default();
/// ctx.add_variable_borrowed("greeting", StrRef::new(&greeting));
/// // `greeting` must outlive `ctx`; no heap copy of the string data is made.
/// ```
use crate::common::traits::{Adder, Comparer, Sizer, Zeroer};
use crate::common::types::{CelInt, CelString};
use crate::common::value::Val;
use crate::ExecutionError;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt;

/// A zero-copy, borrowed string CEL value.
#[derive(Debug)]
pub struct StrRef<'a>(pub &'a str);

impl<'a> StrRef<'a> {
    #[inline]
    pub fn new(s: &'a str) -> Self {
        StrRef(s)
    }

    #[inline]
    pub fn inner(&self) -> &'a str {
        self.0
    }
}

impl fmt::Display for StrRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl<'a> Val for StrRef<'a> {
    // Note: no `as_any` — `StrRef<'a>` is not `'static` so it cannot implement `Any`.

    fn get_type(&self) -> &crate::common::types::Type {
        &super::STRING_TYPE
    }

    fn as_adder(&self) -> Option<&dyn Adder> {
        Some(self)
    }

    fn as_comparer(&self) -> Option<&dyn Comparer> {
        Some(self)
    }

    fn as_sizer(&self) -> Option<&dyn Sizer> {
        Some(self)
    }

    fn as_zeroer(&self) -> Option<&dyn Zeroer> {
        Some(self)
    }

    fn as_str_ref(&self) -> Option<&str> {
        Some(self.0)
    }

    fn equals(&self, other: &dyn Val) -> bool {
        other.as_str_ref().is_some_and(|s| s == self.0)
    }

    /// Cloning a `StrRef` necessarily allocates — the result is an owned `CelString`.
    fn clone_as_boxed(&self) -> Box<dyn Val> {
        Box::new(CelString::from(self.0))
    }
}

impl<'s> Adder for StrRef<'s> {
    fn add<'a>(&'a self, rhs: &dyn Val) -> Result<Cow<'a, dyn Val>, ExecutionError> {
        if let Some(rhs_str) = rhs.as_str_ref() {
            let mut s = String::with_capacity(self.0.len() + rhs_str.len());
            s.push_str(self.0);
            s.push_str(rhs_str);
            Ok(Cow::<dyn Val>::Owned(Box::new(CelString::from(s))))
        } else {
            Err(ExecutionError::UnsupportedBinaryOperator(
                "add",
                (self as &dyn Val).try_into().unwrap_or(crate::Value::Null),
                rhs.try_into().unwrap_or(crate::Value::Null),
            ))
        }
    }
}

impl<'s> Comparer for StrRef<'s> {
    fn compare(&self, rhs: &dyn Val) -> Result<Ordering, ExecutionError> {
        if let Some(rhs_str) = rhs.as_str_ref() {
            Ok(self.0.cmp(rhs_str))
        } else {
            Err(ExecutionError::NoSuchOverload)
        }
    }
}

impl<'s> Sizer for StrRef<'s> {
    fn size(&self) -> CelInt {
        (self.0.len() as i64).into()
    }
}

impl<'s> Zeroer for StrRef<'s> {
    fn is_zero_value(&self) -> bool {
        self.0.is_empty()
    }
}
