use crate::common::traits::{Sizer, Zeroer};
use crate::common::types::{CelInt, Type};
use crate::common::value::Val;
use crate::{common::traits, ExecutionError};
use std::borrow::Cow;
use std::ops::Deref;
use super::bytes_ref::{bytes_concat, BytesRef};
use traits::{Adder, Comparer};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bytes(Vec<u8>);

impl Bytes {
    pub fn into_inner(self) -> Vec<u8> {
        self.0
    }

    pub fn inner(&self) -> &[u8] {
        &self.0
    }
}

impl Deref for Bytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}

impl Val for Bytes {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn get_type(&self) -> &Type {
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
        Some(self.inner())
    }

    fn equals(&self, other: &dyn Val) -> bool {
        BytesRef(self.inner()).equals(other)
    }

    fn clone_as_boxed(&self) -> Box<dyn Val> {
        Box::new(Bytes(self.0.clone()))
    }
}

impl Adder for Bytes {
    fn add<'a>(&'a self, other: &dyn Val) -> Result<Cow<'a, dyn Val>, crate::ExecutionError> {
        bytes_concat(self.inner(), other).map(Cow::<dyn Val>::Owned)
    }
}

impl Comparer for Bytes {
    fn compare(&self, other: &dyn Val) -> Result<std::cmp::Ordering, crate::ExecutionError> {
        BytesRef(self.inner()).compare(other)
    }
}

impl Sizer for Bytes {
    fn size(&self) -> CelInt {
        BytesRef(self.inner()).size()
    }
}

impl Zeroer for Bytes {
    fn is_zero_value(&self) -> bool {
        BytesRef(self.inner()).is_zero_value()
    }
}

impl From<Vec<u8>> for Bytes {
    fn from(value: Vec<u8>) -> Self {
        Bytes(value)
    }
}

impl From<Bytes> for Vec<u8> {
    fn from(value: Bytes) -> Self {
        value.0
    }
}

impl TryFrom<Box<dyn Val>> for Vec<u8> {
    type Error = Box<dyn Val>;

    fn try_from(value: Box<dyn Val>) -> Result<Self, Self::Error> {
        super::cast_boxed::<Bytes>(value).map(|b| b.into_inner())
    }
}

impl<'a> TryFrom<&'a dyn Val> for &'a [u8] {
    type Error = &'a dyn Val;

    fn try_from(value: &'a dyn Val) -> Result<Self, Self::Error> {
        value.as_bytes_ref().ok_or(value)
    }
}

fn bytes_to_bytes<'a>(args: Vec<Cow<'a, dyn Val>>) -> Result<Cow<'a, dyn Val>, ExecutionError> {
    let mut args = args;
    Ok(args.remove(0))
}

fn string_to_bytes<'a>(args: Vec<Cow<'a, dyn Val>>) -> Result<Cow<'a, dyn Val>, ExecutionError> {
    let mut args = args;
    let arg = args.remove(0);
    match arg.as_str_ref() {
        Some(s) => {
            let value = s.as_bytes().to_vec();
            Ok(Cow::<dyn Val>::Owned(Box::new(Bytes::from(value))))
        }
        None => Err(ExecutionError::UnexpectedType {
            got: arg.get_type().name().to_owned(),
            want: "Bytes".to_owned(),
        }),
    }
}

pub(crate) fn stdlib(env: &mut crate::Env) {
    env.add_overload(
        "bytes",
        "string_to_bytes",
        vec![super::STRING_TYPE],
        string_to_bytes,
    )
    .expect("Must be unique id");
    env.add_overload(
        "bytes",
        "bytes_to_bytes",
        vec![super::BYTES_TYPE],
        bytes_to_bytes,
    )
    .expect("Must be unique id");
    env.add_overload(
        "size",
        "size_bytes",
        vec![super::BYTES_TYPE],
        traits::adapter::sizer_size,
    )
    .expect("Must be unique id");
    env.add_member_overload(
        "size",
        "bytes_size",
        super::BYTES_TYPE,
        vec![],
        traits::adapter::sizer_size,
    )
    .expect("Must be unique id");
}
