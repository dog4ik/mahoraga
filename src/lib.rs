mod error;
pub mod eval;
pub mod lex;
pub mod parser;
pub mod s;
pub mod span;
pub mod value;

pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;
