use std::fmt::{Debug, Display};

use crate::{Error, Value};

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
    pub name: &'static str,
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
    pub fn call(&self, args: Args) -> crate::Result<Value> {
        let arity = self.f.arity();
        if args.len() != arity {
            let plural = if arity == 1 { "argument" } else { "arguments" };
            return Err(Error::new(format!(
                "{} takes {arity} {plural}, got {}",
                self.name,
                args.len()
            )));
        }
        self.f
            .call(args)
            .map_err(|e| Error::new_from_parts(format!("{}: {}", self.name, e.message), e.span))
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
    fn call(&self, args: Args) -> crate::Result<Value>;
    /// Counting the piped input, which a pipe supplies as the first argument.
    fn arity(&self) -> usize;
}

macro_rules! impl_callable_with_args {
    () => {};
    ($(($($types:ident),*)),*) => {
        $(
            impl<$($types: TryFrom<crate::Value> + 'static),*> Callable for fn($($types,)*) -> crate::Result<crate::Value>
            where
                $(crate::Error: From<<$types as TryFrom<crate::Value>>::Error>),*
            {
                fn call(&self, args: Args) -> crate::Result<Value> {
                    let mut args = args.0.into_iter();

                    (self)(
                        $(
                            $types::try_from(args.next().ok_or_else(|| crate::Error::new("missing argument"))?)?,
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
