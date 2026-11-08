use crate::{
    fns::ArgConversionError,
    lex::{LexerError, Punct},
    parser::ParserError,
    span::{Span, Spanned},
    value::{OpError, ValueType},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("lexer error: {}", .0.inner)]
    Lexer(Spanned<LexerError>),
    #[error("parser error: {}", .0.inner)]
    Parser(Spanned<ParserError>),
    #[error("runtime error: {}", .0.inner)]
    Runtime(Spanned<RuntimeError>),
}

impl Error {
    pub fn help(&self) -> Option<String> {
        match self {
            Error::Lexer(spanned) => spanned.inner.help(),
            Error::Parser(spanned) => spanned.inner.help(),
            Error::Runtime(spanned) => spanned.inner.help(),
        }
    }
}

macro_rules! impl_from_spanned {
    ($($variant:ident($err:ty)),*) => {$(
        impl From<Spanned<$err>> for Error {
            fn from(value: Spanned<$err>) -> Self {
                Error::$variant(value)
            }
        }
    )*};
}

impl_from_spanned!(
    Lexer(LexerError),
    Parser(ParserError),
    Runtime(RuntimeError)
);

impl Error {
    pub fn span(&self) -> Span {
        match self {
            Error::Lexer(spanned) => spanned.span,
            Error::Parser(spanned) => spanned.span,
            Error::Runtime(spanned) => spanned.span,
        }
    }
}

#[cfg(feature = "miette")]
impl miette::Diagnostic for Error {
    fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        Some(Box::new(match self {
            Error::Lexer(_) => "mahoraga::lexer",
            Error::Parser(_) => "mahoraga::parser",
            Error::Runtime(_) => "mahoraga::runtime",
        }))
    }

    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        match self.help() {
            Some(v) => Some(Box::new(v)),
            None => None,
        }
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        Some(Box::new(std::iter::once(
            miette::LabeledSpan::new_with_span(Some("here".into()), self.span()),
        )))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("{0}")]
    OperationError(#[from] OpError),
    #[error("object can be indexed only by strings, got {}", .0)]
    InvalidObjectKey(ValueType),
    #[error("array can be indexed only by integers, got {}", .0)]
    InvalidArrayIndex(ValueType),
    #[error("only array or object are allowed to be indexed, got {}", .0)]
    InvalidIndexOperand(ValueType),
    #[error("unexpected value type, got {got}, expected {expected}")]
    UnexpectedType { got: ValueType, expected: ValueType },
    #[error("function call failed: {0}")]
    FunctionCallError(#[from] FunctionCallError),
    #[error("Integer overflow")]
    IntegerOverflow,
    #[error("division by zero")]
    DivisionByZero,
    #[error("non-nulish assertion violated")]
    NonNullishAssertion,
    #[error("evaluation depth limit reached")]
    DepthLimitReached,
}

impl RuntimeError {
    pub fn help(&self) -> Option<String> {
        match self {
            RuntimeError::OperationError(op_error) => match op_error {
                OpError::UnsupportedOperands(Punct::Add, ValueType::String, _) => {
                    Some("string can be added only to another string, consider using `to_s` to convert the second operand to string".to_string())
                }
                _ => None,
            },
            RuntimeError::InvalidObjectKey(_) => Some("object can be only keys by string values".to_string()),
            RuntimeError::InvalidArrayIndex(_) => Some("array can only be keyed by integers or rounded floats e.g. 3 or 3.0".to_string()),
            RuntimeError::InvalidIndexOperand(_) => None,
            RuntimeError::UnexpectedType { .. } => None,
            RuntimeError::FunctionCallError(_) => None,
            RuntimeError::IntegerOverflow => None,
            RuntimeError::DivisionByZero => None,
            RuntimeError::NonNullishAssertion => Some("accesses annotated with non nullish operator must not be void or null".to_string()),
            RuntimeError::DepthLimitReached => None,
        }
    }

    pub fn with_span(self, span: Span) -> Spanned<Self> {
        Spanned { inner: self, span }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FunctionCallError {
    #[error("function {name} not found")]
    NotFound { name: String },
    #[error("unexpected number of arguments for function {name}, got {got}, expected {expected}")]
    InvalidArity {
        name: String,
        got: usize,
        expected: usize,
    },
    #[error("only functions can be called, got {0}")]
    UncallableType(ValueType),
    #[error("invalid function argument in pos {argument_pos}, {err}")]
    InvalidArgument {
        argument_pos: usize,
        err: ArgConversionError,
    },
    /// Raised by a function implemented outside of mahoraga, e.g. a host callback.
    #[error("{0}")]
    External(String),
}
