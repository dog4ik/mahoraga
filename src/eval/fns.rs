use std::{
    fmt::{Debug, Display},
    rc::Rc,
};

use crate::{
    RuntimeError, Value,
    eval::{ExecutionContext, FunctionCallError},
};

/// Native function input arguments value list
#[derive(Debug)]
pub struct Args(pub Vec<Value>);

impl Args {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Rust native functions representation
pub struct Function {
    pub name: Rc<str>,
    pub help: Option<&'static str>,
    pub f: Box<dyn Callable>,
}

impl Callable for Function {
    fn call(&self, ctx: &mut dyn ExecutionContext, args: Args) -> Result<Value, RuntimeError> {
        self.f.call(ctx, args)
    }

    fn arity(&self) -> usize {
        self.f.arity()
    }
}

/// Functions are values, so they need an equality: two handles are equal when they point at the
/// same declaration.
impl PartialEq for Function {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fn {}/{}", self.name, self.f.arity())
    }
}

impl Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Function")
            .field("name", &self.name)
            .field("help", &self.help)
            .field("f", &format_args!("fn({} args)", self.f.arity()))
            .finish()
    }
}

pub trait TryFromArgument: Sized {
    fn parse_argument(value: Value) -> Result<Self, crate::ArgConversionError>;
}

pub trait Callable {
    fn call(&self, ctx: &mut dyn ExecutionContext, args: Args) -> Result<Value, RuntimeError>;
    /// Counting the piped input, which a pipe supplies as the first argument.
    fn arity(&self) -> usize;
}

macro_rules! impl_callable_with_args {
    () => {};
    ($(($($types:ident),*)),*) => {
        $(
            impl<$($types: $crate::TryFromArgument),*> Callable for fn(&mut dyn ExecutionContext, $($types,)*) -> std::result::Result<$crate::Value, $crate::eval::RuntimeError> {
                fn call(&self, ctx: &mut dyn ExecutionContext, args: Args) -> std::result::Result<$crate::Value, $crate::eval::RuntimeError> {
                    let mut args = args.0.into_iter();
                    let args_len = args.len();
                    (self)(
                        ctx,
                        $(
                            $types::parse_argument(args.next().unwrap_or($crate::Value::Void)).map_err(|err|
                                $crate::eval::FunctionCallError::InvalidArgument {
                                    err: err.into(),
                                    argument_pos: args_len - args.len()}
                            )?,
                        )*
                    )
                }
                fn arity(&self) -> usize {
                    [$(stringify!($types)),*].len()
                }
            }
        )*
    };
}

impl_callable_with_args! {
    (A),
    (A, B),
    (A, B, C),
    (A, B, C, D),
    (A, B, C, D, E),
    (A, B, C, D, E, F),
    (A, B, C, D, E, F, G),
    (A, B, C, D, E, F, G, H),
    (A, B, C, D, E, F, G, H, I),
    (A, B, C, D, E, F, G, H, I, J),
    (A, B, C, D, E, F, G, H, I, J, K),
    (A, B, C, D, E, F, G, H, I, J, K, L)
}
