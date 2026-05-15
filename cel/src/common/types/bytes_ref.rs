/// Zero-copy borrowed bytes value for use in a [`crate::Context`].
///
/// `BytesRef<'a>` wraps a `&'a [u8]` and implements [`Val`] without allocating.
/// It reports the same CEL type as [`super::bytes::Bytes`] (`Kind::Bytes`) and is
/// interoperable with `CelBytes` in all bytes operations: comparisons, concatenation, etc.
///
/// Like [`super::str_ref::StrRef`], dispatch is handled through `as_bytes_ref()` on [`Val`]
/// rather than the `Any`-based `downcast_ref` path.
use crate::common::traits::{Adder, Comparer, Sizer, Zeroer};
use crate::common::types::{CelBytes, CelInt};
use crate::common::value::Val;
use crate::ExecutionError;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt;

/// A zero-copy, borrowed bytes CEL value.
#[derive(Debug)]
pub struct BytesRef<'a>(pub &'a [u8]);

impl<'a> BytesRef<'a> {
    #[inline]
    pub fn new(b: &'a [u8]) -> Self {
        BytesRef(b)
    }

    #[inline]
    pub fn inner(&self) -> &'a [u8] {
        self.0
    }
}

impl fmt::Display for BytesRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl<'a> Val for BytesRef<'a> {
    // Note: no `as_any` — `BytesRef<'a>` is not `'static` so it cannot implement `Any`.

    fn get_type(&self) -> &crate::common::types::Type {
        &super::BYTES_TYPE
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

    fn as_bytes_ref(&self) -> Option<&[u8]> {
        Some(self.0)
    }

    fn equals(&self, other: &dyn Val) -> bool {
        other.as_bytes_ref().is_some_and(|b| b == self.0)
    }

    /// Cloning a `BytesRef` necessarily allocates — the result is an owned `CelBytes`.
    fn clone_as_boxed(&self) -> Box<dyn Val> {
        Box::new(CelBytes::from(self.0.to_vec()))
    }
}

impl<'s> Adder for BytesRef<'s> {
    fn add<'a>(&'a self, other: &dyn Val) -> Result<Cow<'a, dyn Val>, ExecutionError> {
        if let Some(rhs) = other.as_bytes_ref() {
            let mut v = Vec::with_capacity(self.0.len() + rhs.len());
            v.extend_from_slice(self.0);
            v.extend_from_slice(rhs);
            Ok(Cow::<dyn Val>::Owned(Box::new(CelBytes::from(v))))
        } else {
            Err(ExecutionError::UnsupportedBinaryOperator(
                "add",
                (self as &dyn Val).try_into().unwrap_or(crate::Value::Null),
                other.try_into().unwrap_or(crate::Value::Null),
            ))
        }
    }
}

impl<'s> Comparer for BytesRef<'s> {
    fn compare(&self, other: &dyn Val) -> Result<Ordering, ExecutionError> {
        if let Some(rhs) = other.as_bytes_ref() {
            Ok(self.0.cmp(rhs))
        } else {
            Err(ExecutionError::NoSuchOverload)
        }
    }
}

impl<'s> Sizer for BytesRef<'s> {
    fn size(&self) -> CelInt {
        (self.0.len() as i64).into()
    }
}

impl<'s> Zeroer for BytesRef<'s> {
    fn is_zero_value(&self) -> bool {
        self.0.is_empty()
    }
}
