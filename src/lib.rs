#![doc=include_str!("../README.md")]

pub mod engine;
mod error;
mod eval;
pub mod facts;
mod lex;
mod parser;
mod span;
pub mod value;
mod vm;

pub use error::Error;
pub use eval::{ArgConversionError, FunctionCallError, OpError, RuntimeError};
pub use lex::{Atom, Ident, Punct};
pub use lex::{LexerError, NumLiteralError};
pub use parser::ParserError;
pub use span::Span;
pub use span::Spanned;
pub type Result<T> = std::result::Result<T, Error>;
pub use engine::{Engine, SetEnvError};
pub use eval::Env;
pub use eval::bytecode;
pub use eval::fns::TryFromArgument;
pub use eval::fns::{Args, Callable, Function};
pub use eval::traverse;
pub use parser::parse_expr;
pub use parser::{Node, NodeKind};
pub use value::Value;
pub use eval::ExecutionContext;

pub fn eval_str(s: &str, env: &Env) -> crate::Result<Value> {
    let mut env = traverse::RuntimeEnv::new(env);
    Ok(traverse::eval(&parse_expr(s)?, &mut env)?)
}
