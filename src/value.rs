use std::fmt::Display;

/// Runtime value
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "\"{s}\""),
            Value::Number(n) => write!(f, "{n}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Null => write!(f, "null"),
        }
    }
}

#[allow(clippy::should_implement_trait)]
impl Value {
    pub fn add(self, other: Self) -> crate::Result<Self> {
        Ok(match (self, other) {
            (Value::String(lhs), Value::String(rhs)) => Value::String(lhs + &rhs),
            (Value::Number(lhs), Value::Number(rhs)) => Value::Number(lhs + rhs),
            _ => return Err(crate::Error::new("unsupported add operands")),
        })
    }

    pub fn sub(self, other: Self) -> crate::Result<Self> {
        Ok(match (self, other) {
            (Value::Number(lhs), Value::Number(rhs)) => Value::Number(lhs - rhs),
            _ => return Err(crate::Error::new("unsupported sub operands")),
        })
    }

    pub fn mul(self, other: Self) -> crate::Result<Self> {
        Ok(match (self, other) {
            (Value::Number(lhs), Value::Number(rhs)) => Value::Number(lhs * rhs),
            _ => return Err(crate::Error::new("unsupported mul operands")),
        })
    }

    pub fn div(self, other: Self) -> crate::Result<Self> {
        Ok(match (self, other) {
            (Value::Number(lhs), Value::Number(rhs)) => Value::Number(lhs / rhs),
            _ => return Err(crate::Error::new("unsupported div operands")),
        })
    }

    pub fn truthy(&self) -> bool {
        match self {
            Value::String(s) => !s.is_empty(),
            Value::Number(n) => *n > 0.,
            Value::Bool(b) => *b,
            Value::Null => false,
        }
    }
}
