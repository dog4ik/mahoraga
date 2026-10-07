use std::{
    fmt::{Debug, Display},
    rc::Rc,
};

use crate::{Value, eval::FunctionCallError};

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

pub struct Function {
    pub name: Rc<str>,
    pub help: Option<&'static str>,
    pub f: Box<dyn Callable>,
}

impl Function {
    /// How many arguments this function takes. A piped call supplies the first
    /// of them, so `x | f(a)` reaches a function of arity 2.
    pub fn arity(&self) -> usize {
        self.f.arity()
    }

    /// Check arity, then invoke, tagging both failures with the function name.
    pub fn call(&self, args: Args) -> std::result::Result<Value, FunctionCallError> {
        let arity = self.f.arity();
        if args.len() != arity {
            return Err(FunctionCallError::InvalidArity {
                name: self.name.to_string(),
                got: args.len(),
                expected: arity,
            });
        }
        self.f.call(args)
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

pub trait Callable {
    fn call(&self, args: Args) -> Result<Value, FunctionCallError>;
    /// Counting the piped input, which a pipe supplies as the first argument.
    fn arity(&self) -> usize;
}

macro_rules! impl_callable_with_args {
    () => {};
    ($(($($types:ident),*)),*) => {
        $(
            impl<$($types: TryFrom<crate::Value> + 'static),*> Callable for fn($($types,)*) -> std::result::Result<$crate::Value, $crate::eval::FunctionCallError>
            where
                $($crate::eval::ArgConversionError: From<<$types as TryFrom<crate::Value>>::Error>),*
            {
                fn call(&self, args: Args) -> std::result::Result<$crate::Value, $crate::eval::FunctionCallError> {
                    let mut args = args.0.into_iter();
                    let args_len = args.len();
                    (self)(
                        $(
                            $types::try_from(args.next().unwrap_or($crate::Value::Void)).map_err(|err|
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
