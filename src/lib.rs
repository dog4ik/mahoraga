mod error;
mod eval;
pub mod facts;
mod lex;
mod parser;
mod span;
pub mod value;

pub use error::ErrorKind;
pub use eval::{ArgConversionError, FunctionCallError, OpError, RuntimeError};
pub use lex::{Atom, Ident, Punct};
pub use lex::{LexerError, NumLiteralError};
pub use parser::ParserError;
pub use span::Span;
pub use span::Spanned;
pub type Result<T> = std::result::Result<T, ErrorKind>;
pub use eval::Env;
pub use eval::eval;
pub use eval::fns::{Args, Callable, Function};
pub use parser::NodeKind;
pub use parser::parse_expr;
pub use value::Value;

pub fn eval_str(s: &str, env: &Env) -> crate::Result<Value> {
    Ok(eval(&parse_expr(s)?, env)?)
}
