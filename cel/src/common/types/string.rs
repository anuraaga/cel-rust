use crate::common::traits::{self, Adder, Comparer, Sizer, Zeroer};
use super::str_ref::{str_concat, StrRef};
use crate::common::types::{CelBool, CelDouble, CelInt, CelUInt, Kind, Type};
#[cfg(feature = "chrono")]
use crate::common::types::{CelDuration, CelTimestamp};
use crate::common::value::Val;
use crate::ExecutionError;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::ops::Deref;
use std::string::String as StdString;

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct String(StdString);

impl String {
    pub fn into_inner(self) -> StdString {
        self.0
    }

    pub fn inner(&self) -> &str {
        &self.0
    }
}

impl Deref for String {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}

impl Val for String {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn get_type(&self) -> &Type {
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
        Some(self.inner())
    }

    fn equals(&self, other: &dyn Val) -> bool {
        StrRef(self.inner()).equals(other)
    }

    fn clone_as_boxed(&self) -> Box<dyn Val> {
        Box::new(String(self.0.clone()))
    }
}

impl Adder for String {
    fn add<'a>(&'a self, rhs: &dyn Val) -> Result<Cow<'a, dyn Val>, ExecutionError> {
        str_concat(self.inner(), rhs).map(Cow::<dyn Val>::Owned)
    }
}

impl Comparer for String {
    fn compare(&self, rhs: &dyn Val) -> Result<Ordering, ExecutionError> {
        StrRef(self.inner()).compare(rhs)
    }
}

impl Sizer for String {
    fn size(&self) -> CelInt {
        StrRef(self.inner()).size()
    }
}

impl Zeroer for String {
    fn is_zero_value(&self) -> bool {
        StrRef(self.inner()).is_zero_value()
    }
}

impl From<StdString> for String {
    fn from(v: StdString) -> Self {
        Self(v)
    }
}

impl From<String> for StdString {
    fn from(v: String) -> Self {
        v.0
    }
}

impl From<&str> for String {
    fn from(value: &str) -> Self {
        Self(StdString::from(value))
    }
}

impl TryFrom<Box<dyn Val>> for StdString {
    type Error = Box<dyn Val>;

    fn try_from(value: Box<dyn Val>) -> Result<Self, Self::Error> {
        super::cast_boxed::<String>(value).map(|s| s.into_inner())
    }
}

impl<'a> TryFrom<&'a dyn Val> for &'a str {
    type Error = &'a dyn Val;
    fn try_from(value: &'a dyn Val) -> Result<Self, Self::Error> {
        value.as_str_ref().ok_or(value)
    }
}

fn string_contains<'a>(args: Vec<Cow<'a, dyn Val>>) -> Result<Cow<'a, dyn Val>, ExecutionError> {
    let target = &args[0];
    let arg = &args[1];
    match target.as_str_ref() {
        None => Err(ExecutionError::UnexpectedType {
            got: target.get_type().name().to_string(),
            want: super::STRING_TYPE.name().to_string(),
        }),
        Some(s) => match arg.as_str_ref() {
            None => Err(ExecutionError::UnexpectedType {
                got: arg.get_type().name().to_string(),
                want: super::STRING_TYPE.name().to_string(),
            }),
            Some(needle) => Ok(Cow::<dyn Val>::Owned(Box::new(CelBool::from(
                s.contains(needle),
            )))),
        },
    }
}

fn ends_with_string<'a>(args: Vec<Cow<'a, dyn Val>>) -> Result<Cow<'a, dyn Val>, ExecutionError> {
    let target = &args[0];
    let needle = &args[1];
    match (target.as_str_ref(), needle.as_str_ref()) {
        (Some(t), Some(n)) => Ok(Cow::<dyn Val>::Owned(Box::new(CelBool::from(t.ends_with(n))))),
        (None, _) => Err(ExecutionError::UnexpectedType {
            got: target.get_type().name().to_string(),
            want: super::STRING_TYPE.name().to_string(),
        }),
        (_, None) => Err(ExecutionError::UnexpectedType {
            got: needle.get_type().name().to_string(),
            want: super::STRING_TYPE.name().to_string(),
        }),
    }
}

fn starts_with_string<'a>(args: Vec<Cow<'a, dyn Val>>) -> Result<Cow<'a, dyn Val>, ExecutionError> {
    let target = &args[0];
    let needle = &args[1];
    match (target.as_str_ref(), needle.as_str_ref()) {
        (Some(t), Some(n)) => Ok(Cow::<dyn Val>::Owned(Box::new(CelBool::from(t.starts_with(n))))),
        (None, _) => Err(ExecutionError::UnexpectedType {
            got: target.get_type().name().to_string(),
            want: super::STRING_TYPE.name().to_string(),
        }),
        (_, None) => Err(ExecutionError::UnexpectedType {
            got: needle.get_type().name().to_string(),
            want: super::STRING_TYPE.name().to_string(),
        }),
    }
}

#[cfg(feature = "regex")]
fn matches<'a>(args: Vec<Cow<'a, dyn Val>>) -> Result<Cow<'a, dyn Val>, ExecutionError> {
    let this = &args[0];
    let regex = &args[1];
    match (this.as_str_ref(), regex.as_str_ref()) {
        (Some(s), Some(r)) => match regex::Regex::new(r) {
            Ok(re) => Ok(Cow::<dyn Val>::Owned(Box::new(CelBool::from(re.is_match(s))))),
            Err(err) => Err(ExecutionError::FunctionError {
                function: "matches".to_string(),
                message: format!("'{r}' not a valid regex:\n{err}"),
            }),
        },
        (None, _) => Err(ExecutionError::UnexpectedType {
            got: this.get_type().name().to_string(),
            want: super::STRING_TYPE.name().to_string(),
        }),
        (_, None) => Err(ExecutionError::UnexpectedType {
            got: regex.get_type().name().to_string(),
            want: super::STRING_TYPE.name().to_string(),
        }),
    }
}

fn string<'a>(args: Vec<Cow<'a, dyn Val>>) -> Result<Cow<'a, dyn Val>, ExecutionError> {
    let mut args = args;
    let arg = args.remove(0);

    // Fast path: already a string — return without any clone.
    if arg.get_type().kind() == Kind::String {
        return Ok(arg);
    }

    let arg = arg.into_owned();
    let ret: Result<Box<String>, Box<dyn Val>> = match arg.get_type().kind() {
        Kind::Int => match arg.downcast_ref::<CelInt>() {
            Some(arg) => Ok(Box::new(String::from(arg.to_string()))),
            None => Err(arg),
        },
        Kind::UInt => match arg.downcast_ref::<CelUInt>() {
            Some(arg) => Ok(Box::new(String::from(arg.to_string()))),
            None => Err(arg),
        },
        Kind::Double => match arg.downcast_ref::<CelDouble>() {
            Some(arg) => Ok(Box::new(String::from(arg.to_string()))),
            None => Err(arg),
        },
        Kind::Bytes => match arg.as_bytes_ref() {
            Some(bytes) => Ok(Box::new(String::from(
                StdString::from_utf8_lossy(bytes).as_ref(),
            ))),
            None => Err(arg),
        },
        #[cfg(feature = "chrono")]
        Kind::Timestamp => match arg.downcast_ref::<CelTimestamp>() {
            Some(ts) => Ok(Box::new(String::from(ts.inner().to_rfc3339()))),
            None => Err(arg),
        },
        #[cfg(feature = "chrono")]
        Kind::Duration => match arg.downcast_ref::<CelDuration>() {
            Some(arg) => Ok(Box::new(String::from(crate::duration::format_duration(arg.inner())))),
            None => Err(arg),
        },
        _ => Err(arg),
    };
    match ret {
        Ok(ret) => Ok(Cow::<dyn Val>::Owned(ret)),
        Err(arg) => Err(ExecutionError::FunctionError {
            function: "string".to_owned(),
            message: format!("cannot convert {arg:?} to string"),
        }),
    }
}

pub(crate) fn stdlib(env: &mut crate::Env) {
    env.add_overload(
        "string",
        "string_to_string",
        vec![super::STRING_TYPE],
        string,
    )
    .expect("Must be unique id");
    env.add_overload("string", "int64_to_string", vec![super::INT_TYPE], string)
        .expect("Must be unique id");
    env.add_overload("string", "uint64_to_string", vec![super::UINT_TYPE], string)
        .expect("Must be unique id");
    env.add_overload(
        "string",
        "double_to_string",
        vec![super::DOUBLE_TYPE],
        string,
    )
    .expect("Must be unique id");
    env.add_overload("string", "bytes_to_string", vec![super::BYTES_TYPE], string)
        .expect("Must be unique id");

    #[cfg(feature = "chrono")]
    {
        env.add_overload(
            "string",
            "timestamp_to_string",
            vec![super::TIMESTAMP_TYPE],
            string,
        )
        .expect("Must be unique id");
        env.add_overload(
            "string",
            "duration_to_string",
            vec![super::DURATION_TYPE],
            string,
        )
        .expect("Must be unique id");
    }

    env.add_member_overload(
        "contains",
        "contains_string",
        super::STRING_TYPE,
        vec![super::STRING_TYPE],
        string_contains,
    )
    .expect("Must be unique id");
    env.add_member_overload(
        "endsWith",
        "ends_with_string",
        super::STRING_TYPE,
        vec![super::STRING_TYPE],
        ends_with_string,
    )
    .expect("Must be unique id");
    env.add_overload(
        "size",
        "size_string",
        vec![super::STRING_TYPE],
        traits::adapter::sizer_size,
    )
    .expect("Must be unique id");
    env.add_member_overload(
        "size",
        "string_size",
        super::STRING_TYPE,
        vec![],
        traits::adapter::sizer_size,
    )
    .expect("Must be unique id");
    env.add_member_overload(
        "startsWith",
        "starts_with_string",
        super::STRING_TYPE,
        vec![super::STRING_TYPE],
        starts_with_string,
    )
    .expect("Must be unique id");
    #[cfg(feature = "regex")]
    env.add_member_overload(
        "matches",
        "matches",
        super::STRING_TYPE,
        vec![super::STRING_TYPE],
        matches,
    )
    .expect("Must be unique id");
}

#[cfg(test)]
mod tests {
    use super::StdString;
    use super::String;
    use crate::common::value::Val;

    #[test]
    fn test_try_into_string() {
        let str: Box<dyn Val> = Box::new(String::from("cel-rust"));
        assert_eq!(Ok(StdString::from("cel-rust")), str.try_into())
    }

    #[test]
    fn test_try_into_str() {
        let str: Box<dyn Val> = Box::new(String::from("cel-rust"));
        assert_eq!(Ok("cel-rust"), str.as_ref().try_into())
    }
}
