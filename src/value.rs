use std::{collections::HashMap, fmt::Display};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Object(pub HashMap<String, Value>);

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Array(pub Vec<Value>);

#[derive(Debug, Clone, PartialEq)]
pub enum Number {
    Float(f64),
    Int(i64),
}

impl Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Number::Float(n) => write!(f, "{n}"),
            Number::Int(i) => write!(f, "{i}"),
        }
    }
}

/// Runtime value
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    Object(Object),
    Array(Array),
    Null,
    #[default]
    Void,
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "\"{s}\""),
            Value::Number(n) => write!(f, "{n}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Object(obj) => write!(f, "{obj:?}"),
            Value::Array(arr) => write!(f, "{arr:?}"),
            Value::Null => write!(f, "null"),
            Value::Void => write!(f, "void"),
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
            Value::Number(n) => *n != 0.,
            Value::Bool(b) => *b,
            Value::Null => false,
            Value::Void => false,
            Self::Object(Object(obj)) => !obj.is_empty(),
            Self::Array(Array(arr)) => !arr.is_empty(),
        }
    }

    pub fn mt(&self, other: Value) -> Result<Value, crate::Error> {
        Ok(Value::Bool(match (self, other) {
            (Value::String(s), Value::String(o)) => *s > o,
            (Value::Number(s), Value::Number(o)) => *s > o,
            (Value::Bool(s), Value::Bool(o)) => *s & !o,
            _ => return Err(crate::Error::new("unsupported mt operands")),
        }))
    }

    pub fn mte(&self, other: Value) -> Result<Value, crate::Error> {
        Ok(Value::Bool(match (self, other) {
            (Value::String(s), Value::String(o)) => *s >= o,
            (Value::Number(s), Value::Number(o)) => *s >= o,
            (Value::Bool(s), Value::Bool(o)) => *s >= o,
            _ => return Err(crate::Error::new("unsupported mte operands")),
        }))
    }

    pub fn lt(&self, other: Value) -> Result<Value, crate::Error> {
        Ok(Value::Bool(match (self, other) {
            (Value::String(s), Value::String(o)) => *s < o,
            (Value::Number(s), Value::Number(o)) => *s < o,
            (Value::Bool(s), Value::Bool(o)) => !*s & o,
            _ => return Err(crate::Error::new("unsupported lt operands")),
        }))
    }

    pub fn lte(&self, other: Value) -> Result<Value, crate::Error> {
        Ok(Value::Bool(match (self, other) {
            (Value::String(s), Value::String(o)) => *s <= o,
            (Value::Number(s), Value::Number(o)) => *s <= o,
            (Value::Bool(s), Value::Bool(o)) => *s <= o,
            _ => return Err(crate::Error::new("unsupported lte operands")),
        }))
    }

    pub fn eq(&self, other: Value) -> Result<Value, crate::Error> {
        Ok(Value::Bool(match (self, other) {
            (Value::String(s), Value::String(o)) => *s == o,
            (Value::Number(s), Value::Number(o)) => *s == o,
            (Value::Bool(s), Value::Bool(o)) => *s == o,
            (Value::Null, Value::Null) => true,
            (Value::Array(Array(s)), Value::Array(Array(o))) => *s == o,
            _ => false,
        }))
    }

    pub fn nullish(&self) -> bool {
        matches!(self, Value::Void | Value::Null)
    }

    pub fn blank(&self) -> bool {
        match self {
            Value::String(s) if s.is_empty() => true,
            Value::Null => true,
            Value::Void => true,
            _ => false,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::String(_) => "str",
            Value::Number(_) => "number",
            Value::Bool(_) => "bool",
            Value::Object(_) => "object",
            Value::Array(_) => "array",
            Value::Null => "null",
            Value::Void => "void",
        }
    }
}

#[cfg(any(feature = "serde_json", test))]
mod from_serde_json {
    use super::*;

    impl From<serde_json::Value> for Value {
        fn from(value: serde_json::Value) -> Self {
            match value {
                serde_json::Value::Bool(b) => Self::Bool(b),
                serde_json::Value::Number(number) => Self::Number(
                    number
                        .as_f64()
                        .expect("each number should be convertible to f64"),
                ),
                serde_json::Value::String(s) => Self::String(s),
                serde_json::Value::Array(values) => {
                    Self::Array(Array(values.into_iter().map(|v| Self::from(v)).collect()))
                }
                serde_json::Value::Object(map) => Self::Object(Object::from(map)),
                serde_json::Value::Null => Self::Null,
            }
        }
    }

    impl From<serde_json::Map<String, serde_json::Value>> for Object {
        fn from(value: serde_json::Map<String, serde_json::Value>) -> Self {
            Object(HashMap::from_iter(
                value.into_iter().map(|(k, v)| (k, Value::from(v))),
            ))
        }
    }
}
