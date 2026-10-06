use crate::{
    eval::RuntimeError,
    lex::LexerError,
    parser::ParserError,
    span::{Span, Spanned},
};

#[derive(Debug, thiserror::Error)]
pub enum ErrorKind {
    #[error("lexer error: {}", .0.inner)]
    Lexer(Spanned<LexerError>),
    #[error("parser error: {}", .0.inner)]
    Parser(Spanned<ParserError>),
    #[error("runtime error: {}", .0.inner)]
    Runtime(Spanned<RuntimeError>),
}

impl ErrorKind {
    pub fn help(&self) -> Option<String> {
        match self {
            ErrorKind::Lexer(spanned) => spanned.inner.help(),
            ErrorKind::Parser(spanned) => spanned.inner.help(),
            ErrorKind::Runtime(spanned) => spanned.inner.help(),
        }
    }
}

macro_rules! impl_from_spanned {
    ($($variant:ident($err:ty)),*) => {$(
        impl From<Spanned<$err>> for ErrorKind {
            fn from(value: Spanned<$err>) -> Self {
                ErrorKind::$variant(value)
            }
        }
    )*};
}

impl_from_spanned!(
    Lexer(LexerError),
    Parser(ParserError),
    Runtime(RuntimeError)
);

impl ErrorKind {
    pub fn span(&self) -> Span {
        match self {
            ErrorKind::Lexer(spanned) => spanned.span,
            ErrorKind::Parser(spanned) => spanned.span,
            ErrorKind::Runtime(spanned) => spanned.span,
        }
    }
}

#[cfg(feature = "miette")]
impl miette::Diagnostic for ErrorKind {
    fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        Some(Box::new(match self {
            ErrorKind::Lexer(_) => "mahoraga::lexer",
            ErrorKind::Parser(_) => "mahoraga::parser",
            ErrorKind::Runtime(_) => "mahoraga::runtime",
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
