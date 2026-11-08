#![doc=include_str!("../README.md")]

pub mod engine;
pub mod env;
pub mod error;
pub mod facts;
pub mod fns;
pub mod lex;
pub mod parser;
pub mod span;
mod std_fns;
pub mod traverse;
pub mod value;
pub mod vm;

#[cfg(test)]
mod tests;

pub use value::Value;

pub type Result<T> = std::result::Result<T, error::Error>;

/// Parses and evaluates `s` with the tree-walking evaluator.
pub fn eval_str(s: &str, env: &env::Env) -> Result<Value> {
    let node = parser::parse_expr(s)?;
    let mut runtime = traverse::TraverseRuntime::new(env, node.span);
    Ok(runtime.eval(&node)?)
}
