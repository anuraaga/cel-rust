use crate::common::value::Val;
use crate::magic::{Function, FunctionRegistry, IntoFunction};
use crate::objects::{TryIntoValue, Value};
use crate::parser::Expression;
use crate::{Env, ExecutionError};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;

pub enum StoredValue<'a> {
    Owned(Box<dyn Val>),
    Borrowed(&'a dyn Val),
    /// An owned, non-`'static` `Val` — e.g. `StrRef<'a>` or `BytesRef<'a>`.
    /// Unlike `Owned`, the boxed value may borrow from data that lives as long as `'a`.
    OwnedBorrowing(Box<dyn Val + 'a>),
}

/// Context is a collection of variables and functions that can be used
/// by the interpreter to resolve expressions.
///
/// The context can be either a parent context, or a child context. A
/// parent context is created by default and contains all of the built-in
/// functions. A child context can be created by calling `.new_inner_scope()`. The
/// child context has it's own variables (which can be added to), but it
/// will also reference the parent context. This allows for variables to
/// be overridden within the child context while still being able to
/// resolve variables in the child's parents. You can have theoretically
/// have an infinite number of child contexts that reference each-other.
///
/// So why is this important? Well some CEL-macros such as the `.map` macro
/// declare intermediate user-specified identifiers that should only be
/// available within the macro, and should not override variables in the
/// parent context. The `.map` macro can create a child context from the parent, add the
/// intermediate identifier to the child context, and then evaluate the
/// map expression.
///
/// Intermediate variable stored in child context
///               ↓
/// [1, 2, 3].map(x, x * 2) == [2, 4, 6]
///                  ↑
/// Only in scope for the duration of the map expression
///
pub enum Context<'a> {
    Root {
        functions: FunctionRegistry,
        variables: BTreeMap<String, StoredValue<'a>>,
        resolver: Option<&'a dyn VariableResolver>,
        env: Arc<Env>,
    },
    Child {
        parent: &'a Context<'a>,
        variables: BTreeMap<String, StoredValue<'a>>,
        resolver: Option<&'a dyn VariableResolver>,
    },
}

impl<'a> Context<'a> {
    pub fn add_variable<S, V>(
        &mut self,
        name: S,
        value: V,
    ) -> Result<(), <V as TryIntoValue>::Error>
    where
        S: Into<String>,
        V: TryIntoValue,
    {
        match self {
            Context::Root { variables, .. } => {
                let value = value.try_into_value()?;
                let value: Box<dyn Val> = value.try_into().unwrap();
                variables.insert(name.into(), StoredValue::Owned(value));
            }
            Context::Child { variables, .. } => {
                let value = value.try_into_value()?;
                let value: Box<dyn Val> = value.try_into().unwrap();
                variables.insert(name.into(), StoredValue::Owned(value));
            }
        }
        Ok(())
    }

    pub fn add_variable_from_value<S, V>(&mut self, name: S, value: V)
    where
        S: Into<String>,
        V: Into<Value>,
    {
        match self {
            Context::Root { variables, .. } => {
                let value = value.into();
                let value: Box<dyn Val> = value.try_into().unwrap();
                variables.insert(name.into(), StoredValue::Owned(value));
            }
            Context::Child { variables, .. } => {
                let value = value.into();
                let value: Box<dyn Val> = value.try_into().unwrap();
                variables.insert(name.into(), StoredValue::Owned(value));
            }
        }
    }

    pub fn add_variable_ref<S, V>(&mut self, name: S, value: &'a V)
    where
        S: Into<String>,
        V: Val + 'a,
    {
        match self {
            Context::Root { variables, .. } => {
                variables.insert(name.into(), StoredValue::Borrowed(value));
            }
            Context::Child { variables, .. } => {
                variables.insert(name.into(), StoredValue::Borrowed(value));
            }
        }
    }

    /// Store an owned non-`'static` value (e.g. `StrRef<'a>` or `BytesRef<'a>`) in the context.
    ///
    /// The value type only needs to outlive `'a` — the lifetime of this context's borrows —
    /// rather than `'static`. This is the preferred entry-point for zero-copy string/bytes
    /// variables whose data is owned elsewhere but needs no allocation inside the context.
    ///
    /// ```ignore
    /// let src = String::from("hello");
    /// ctx.add_variable_borrowed("greeting", StrRef::new(&src));
    /// // `src` must outlive `ctx`; no heap copy of the string data is made.
    /// ```
    pub fn add_variable_borrowed<S, V>(&mut self, name: S, value: V)
    where
        S: Into<String>,
        V: Val + 'a,
    {
        let boxed: Box<dyn Val + 'a> = Box::new(value);
        match self {
            Context::Root { variables, .. } => {
                variables.insert(name.into(), StoredValue::OwnedBorrowing(boxed));
            }
            Context::Child { variables, .. } => {
                variables.insert(name.into(), StoredValue::OwnedBorrowing(boxed));
            }
        }
    }

    pub(crate) fn add_variable_as_val<S>(&mut self, name: S, value: Box<dyn Val>)
    where
        S: Into<String>,
    {
        match self {
            Context::Root { variables, .. } => {
                variables.insert(name.into(), StoredValue::Owned(value));
            }
            Context::Child { variables, .. } => {
                variables.insert(name.into(), StoredValue::Owned(value));
            }
        }
    }

    pub fn set_variable_resolver(&mut self, r: &'a dyn VariableResolver) {
        match self {
            Context::Root { resolver, .. } => {
                *resolver = Some(r);
            }
            Context::Child { resolver, .. } => {
                *resolver = Some(r);
            }
        }
    }

    pub fn get_variable<S>(&'a self, name: S) -> Option<Cow<'a, dyn Val>>
    where
        S: AsRef<str>,
    {
        let name = name.as_ref();
        match self {
            Context::Child {
                variables,
                parent,
                resolver,
            } => resolver
                .and_then(|r| r.resolve(name))
                .or_else(|| {
                    variables
                        .get(name)
                        .map(|value| match value {
                            StoredValue::Owned(value) => Cow::<dyn Val>::Borrowed(value.as_ref()),
                            StoredValue::Borrowed(value) => Cow::<dyn Val>::Borrowed(*value),
                            StoredValue::OwnedBorrowing(value) => Cow::<dyn Val>::Borrowed(value.as_ref()),
                        })
                        .or_else(|| parent.get_variable(name))
                }),
            Context::Root {
                variables,
                resolver,
                ..
            } => resolver
                .and_then(|r| r.resolve(name))
                .or_else(|| {
                    variables
                        .get(name)
                        .map(|value| match value {
                            StoredValue::Owned(value) => Cow::<dyn Val>::Borrowed(value.as_ref()),
                            StoredValue::Borrowed(value) => Cow::<dyn Val>::Borrowed(*value),
                            StoredValue::OwnedBorrowing(value) => Cow::<dyn Val>::Borrowed(value.as_ref()),
                        })
                }),
        }
    }

    pub(crate) fn env(&self) -> &Env {
        match self {
            Context::Root { env, .. } => env.as_ref(),
            Context::Child { parent, .. } => parent.env(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn get_function(&self, name: &str) -> Option<&Function> {
        match self {
            Context::Root { functions, .. } => functions.get(name),
            Context::Child { parent, .. } => parent.get_function(name),
        }
    }

    pub fn add_function<T: 'static, F>(&mut self, name: &str, value: F)
    where
        F: IntoFunction<T> + 'static + Send + Sync,
    {
        if let Context::Root { functions, .. } = self {
            functions.add(name, value);
        };
    }

    pub fn resolve(&self, expr: &Expression) -> Result<Value, ExecutionError> {
        Value::resolve(expr, self)
    }

    pub fn resolve_all(&self, exprs: &[Expression]) -> Result<Value, ExecutionError> {
        Value::resolve_all(exprs, self)
    }

    pub fn new_inner_scope(&self) -> Context<'_> {
        Context::Child {
            parent: self,
            variables: Default::default(),
            resolver: None,
        }
    }

    /// Constructs a new empty context with no variables or functions.
    ///
    /// If you're looking for a context that has all the standard methods, functions
    /// and macros already added to the context, use [`Context::default`] instead.
    ///
    /// # Example
    /// ```
    /// use cel::Context;
    /// let mut context = Context::empty();
    /// context.add_function("add", |a: i64, b: i64| a + b);
    /// ```
    pub fn empty() -> Self {
        Context::Root {
            env: Arc::new(Env::default()),
            variables: Default::default(),
            functions: Default::default(),
            resolver: None,
        }
    }

    pub fn with_env(env: Arc<Env>) -> Self {
        Context::Root {
            env,
            variables: Default::default(),
            functions: Default::default(),
            resolver: None,
        }
    }
}

impl Default for Context<'_> {
    fn default() -> Self {
        Context::Root {
            env: Arc::new(Env::stdlib()),
            variables: Default::default(),
            functions: Default::default(),
            resolver: None,
        }
    }
}

/// VariableResolver implements a custom resolver for variables that is consulted before looking at
/// variables added to the context. This allows dynamic variables, or avoiding HashMap lookup/creation.
///
/// Returning [`std::borrow::Cow::Borrowed`] from `resolve` allows zero-copy variable access: the
/// returned reference points directly into the resolver's own data without any allocation.
///
/// # Example — zero-copy borrow
/// ```ignore
/// use std::borrow::Cow;
/// use cel::common::types::CelString;
/// use cel::common::value::Val;
///
/// struct ValueContext {
///     request: CelString,
///     response: CelString,
/// }
///
/// impl cel::context::VariableResolver for ValueContext {
///     fn resolve<'a>(&'a self, variable: &str) -> Option<Cow<'a, dyn Val>> {
///         match variable {
///             "request" => Some(Cow::Borrowed(&self.request as &dyn Val)),
///             "response" => Some(Cow::Borrowed(&self.response as &dyn Val)),
///             _ => None,
///         }
///     }
/// }
/// ```
pub trait VariableResolver: Send + Sync {
    fn resolve<'a>(&'a self, variable: &str) -> Option<Cow<'a, dyn Val>>;
}

impl<T: VariableResolver> VariableResolver for Box<T> {
    fn resolve<'a>(&'a self, variable: &str) -> Option<Cow<'a, dyn Val>> {
        (**self).resolve(variable)
    }
}

impl<T: VariableResolver> VariableResolver for Arc<T> {
    fn resolve<'a>(&'a self, variable: &str) -> Option<Cow<'a, dyn Val>> {
        (**self).resolve(variable)
    }
}

impl<T: VariableResolver> VariableResolver for &T {
    fn resolve<'a>(&'a self, variable: &str) -> Option<Cow<'a, dyn Val>> {
        (**self).resolve(variable)
    }
}

#[cfg(test)]
mod test {
    // A helper function that requires T to implement some traits
    fn assert_send<T: Send>() {}

    #[test]
    fn test_context_is_send() {
        // This line will only compile if assertion passes
        assert_send::<super::Context>();
    }
}
