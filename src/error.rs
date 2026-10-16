use crate::{
    eval::RuntimeError,
    lex::LexerError,
    parser::ParserError,
    span::{Span, Spanned},
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
