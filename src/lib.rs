mod error;
mod eval;
mod lex;
mod parser;
mod span;
pub mod value;

pub use error::Error;
pub use lex::{Atom, Ident, Punct};
pub use span::Span;
pub type Result<T> = std::result::Result<T, Error>;
pub use eval::Env;
pub use eval::PureFunction;
pub use eval::eval;
pub use parser::Node;
pub use parser::parse_expr;
pub use value::Value;
