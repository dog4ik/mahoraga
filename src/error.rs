use std::fmt::Display;

use crate::span::Span;

#[derive(Debug, Clone)]
pub struct Error {
    span: Option<Span>,
    message: String,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.span {
            Some(pos) => write!(f, "{} in pos: {}", self.message, pos),
            None => write!(f, "{}", self.message),
        }
    }
}

impl Error {
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            span: None,
        }
    }

    pub fn new_with_span(msg: impl Into<String>, span: Span) -> Self {
        Self {
            message: msg.into(),
            span: Some(span),
        }
    }
}
