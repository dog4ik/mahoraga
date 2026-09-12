use std::fmt::Display;

use crate::lex::Tok;

pub enum S {
    Atom(Tok),
    Cons(Tok, Vec<S>),
}

impl Display for S {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            S::Atom(a) => write!(f, "{a}"),
            S::Cons(head, rest) => {
                write!(f, "({head}: ")?;
                for s in rest {
                    write!(f, " {}", s)?
                }
                write!(f, ")")
            }
        }
    }
}
