use std::{
    collections::HashMap,
    fmt::{Display, Write},
    rc::Rc,
};

use crate::{
    Function, Punct,
    eval::{OpError, RuntimeError, closure::Closure},
    parser::fmt_comma_separated,
};

mod from;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Object(pub HashMap<String, Value>);

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Array(pub Vec<Value>);

impl Display for Array {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        fmt_comma_separated(f, &self.0, ']')
    }
}

impl Array {
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Object {
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Display for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char('{')?;

        for (i, (k, v)) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{k}: {v}")?;
        }
        f.write_char('}')
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Number {
    Float(f64),
    Int(i64),
}

impl Default for Number {
    fn default() -> Self {
        Self::Int(0)
    }
}

impl Number {
    pub fn as_f64(&self) -> f64 {
        match self {
            Number::Float(f) => *f,
            Number::Int(i) => *i as f64,
        }
    }
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Float(l0), Self::Float(r0)) => l0 == r0,
            (Self::Int(l0), Self::Int(r0)) => l0 == r0,
            _ => self.as_f64() == other.as_f64(),
        }
    }
}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Self::Int(l), Self::Int(r)) => l.partial_cmp(r),
            _ => self.as_f64().partial_cmp(&other.as_f64()),
        }
    }
}

macro_rules! impl_op {
    ($method: ident, $checked: ident, $sign: tt) => {
        /// Integer overflow and division by zero are errors, not panics.
        pub fn $method(self, rhs: Self) -> std::result::Result<Self, crate::eval::RuntimeError> {
            Ok(match (self, rhs) {
                (Number::Int(lhs), Number::Int(rhs)) => {
                    Number::Int(lhs.$checked(rhs).ok_or_else(|| {
                        $crate::eval::RuntimeError::IntegerOverflow
                    })?)
                }
                (lhs, rhs) => Number::Float(lhs.as_f64() $sign rhs.as_f64()),
            })
        }
    };
}

#[allow(clippy::should_implement_trait)]
impl Number {
    impl_op!(add, checked_add, +);
    impl_op!(sub, checked_sub, -);
    impl_op!(mul, checked_mul, *);

    pub fn div(self, rhs: Self) -> std::result::Result<Self, RuntimeError> {
        if let Number::Int(0) | Number::Float(0.) = rhs {
            return Err(RuntimeError::DivisionByZero);
        }
        Ok(match (self, rhs) {
            (Number::Int(lhs), Number::Int(rhs)) => {
                Number::Int(lhs.checked_div(rhs).ok_or(RuntimeError::IntegerOverflow)?)
            }
            (lhs, rhs) => Number::Float(lhs.as_f64() / rhs.as_f64()),
        })
    }
}

impl Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Number::Float(n) => {
                write!(f, "{n:?}")
            }
            Number::Int(i) => write!(f, "{i}"),
        }
    }
}

#[derive(Debug)]
pub enum CallableValue {
    Closure(Closure),
    ExternalFunction(Rc<Function>),
}

impl crate::Callable for CallableValue {
    fn call(
        &self,
        ctx: &mut crate::RuntimeEnv<'_>,
        args: crate::Args,
    ) -> Result<Value, RuntimeError> {
        match self {
            CallableValue::Closure(closure) => closure.call(ctx, args).map_err(|e| e.inner),
            CallableValue::ExternalFunction(function) => function.call(ctx, args),
        }
    }

    fn arity(&self) -> usize {
        match self {
            CallableValue::Closure(closure) => closure.args.len(),
            CallableValue::ExternalFunction(function) => function.arity(),
        }
    }
}

/// Runtime value
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Value {
    String(String),
    Number(Number),
    Bool(bool),
    Object(Object),
    Array(Array),
    Function(Rc<Function>),
    Closure(Closure),
    Null,
    #[default]
    Void,
}

/// All possible value types
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ValueType {
    String,
    Integer,
    Float,
    Bool,
    Object,
    Array,
    Function,
    Null,
    #[default]
    Void,
}

impl ValueType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::String => "str",
            Self::Integer => "integer",
            Self::Float => "float",
            Self::Bool => "bool",
            Self::Object => "object",
            Self::Array => "array",
            Self::Function => "function",
            Self::Null => "null",
            Self::Void => "void",
        }
    }
}

impl Display for ValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "\"{s}\""),
            Value::Number(n) => write!(f, "{n}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Object(obj) => write!(f, "{obj}"),
            Value::Array(arr) => write!(f, "{arr}"),
            Value::Function(fun) => write!(f, "{fun}"),
            Value::Closure(closure) => {
                write!(f, "anynomyous closure/{}", closure.args.len())
            }
            Value::Null => write!(f, "null"),
            Value::Void => write!(f, "void"),
        }
    }
}

#[allow(clippy::should_implement_trait)]
impl Value {
    pub fn add(&self, other: &Self) -> Result<Value, RuntimeError> {
        Ok(match (self, other) {
            (Value::String(lhs), Value::String(rhs)) => Value::String(lhs.to_owned() + rhs),
            (Value::Number(lhs), Value::Number(rhs)) => Value::Number(lhs.add(*rhs)?),
            _ => {
                return Err(RuntimeError::OperationError(OpError::UnsupportedOperands(
                    Punct::Add,
                    self.value_type(),
                    other.value_type(),
                )));
            }
        })
    }

    pub fn sub(&self, other: &Self) -> Result<Value, RuntimeError> {
        Ok(match (self, other) {
            (Value::Number(lhs), Value::Number(rhs)) => Value::Number(lhs.sub(*rhs)?),
            _ => {
                return Err(RuntimeError::OperationError(OpError::UnsupportedOperands(
                    Punct::Sub,
                    self.value_type(),
                    other.value_type(),
                )));
            }
        })
    }

    pub fn mul(&self, other: &Self) -> Result<Value, RuntimeError> {
        Ok(match (self, other) {
            (Value::Number(lhs), Value::Number(rhs)) => Value::Number(lhs.mul(*rhs)?),
            _ => {
                return Err(RuntimeError::OperationError(OpError::UnsupportedOperands(
                    Punct::Mul,
                    self.value_type(),
                    other.value_type(),
                )));
            }
        })
    }

    pub fn div(&self, other: &Self) -> Result<Value, RuntimeError> {
        Ok(match (self, other) {
            (Value::Number(lhs), Value::Number(rhs)) => Value::Number(lhs.div(*rhs)?),
            _ => {
                return Err(RuntimeError::OperationError(OpError::UnsupportedOperands(
                    Punct::Div,
                    self.value_type(),
                    other.value_type(),
                )));
            }
        })
    }

    pub fn truthy(&self) -> bool {
        match self {
            Value::String(s) => !s.is_empty(),
            Value::Number(Number::Int(n)) => *n != 0,
            Value::Number(Number::Float(n)) => *n != 0.,
            Value::Bool(b) => *b,
            Value::Null | Value::Void => false,
            Self::Object(Object(obj)) => !obj.is_empty(),
            Self::Array(Array(arr)) => !arr.is_empty(),
            Self::Function(_) => true,
            Self::Closure(_) => true,
        }
    }

    pub fn mt(&self, other: &Value) -> Result<Value, RuntimeError> {
        Ok(Value::Bool(match (self, other) {
            (Value::String(s), Value::String(o)) => s > o,
            (Value::Number(s), Value::Number(o)) => s > o,
            (Value::Bool(s), Value::Bool(o)) => s & !o,
            _ => {
                return Err(RuntimeError::OperationError(OpError::UnsupportedOperands(
                    Punct::More,
                    self.value_type(),
                    other.value_type(),
                )));
            }
        }))
    }

    pub fn mte(&self, other: &Value) -> Result<Value, RuntimeError> {
        Ok(Value::Bool(match (self, other) {
            (Value::String(s), Value::String(o)) => s >= o,
            (Value::Number(s), Value::Number(o)) => s >= o,
            (Value::Bool(s), Value::Bool(o)) => s >= o,
            _ => {
                return Err(RuntimeError::OperationError(OpError::UnsupportedOperands(
                    Punct::MoreOrEq,
                    self.value_type(),
                    other.value_type(),
                )));
            }
        }))
    }

    pub fn lt(&self, other: &Value) -> Result<Value, RuntimeError> {
        Ok(Value::Bool(match (self, other) {
            (Value::String(s), Value::String(o)) => s < o,
            (Value::Number(s), Value::Number(o)) => s < o,
            (Value::Bool(s), Value::Bool(o)) => !s & o,
            _ => {
                return Err(RuntimeError::OperationError(OpError::UnsupportedOperands(
                    Punct::Less,
                    self.value_type(),
                    other.value_type(),
                )));
            }
        }))
    }

    pub fn lte(&self, other: &Value) -> Result<Value, RuntimeError> {
        Ok(Value::Bool(match (self, other) {
            (Value::String(s), Value::String(o)) => s <= o,
            (Value::Number(s), Value::Number(o)) => s <= o,
            (Value::Bool(s), Value::Bool(o)) => s <= o,
            _ => {
                return Err(RuntimeError::OperationError(OpError::UnsupportedOperands(
                    Punct::LessOrEq,
                    self.value_type(),
                    other.value_type(),
                )));
            }
        }))
    }

    pub fn eq(&self, other: &Value) -> Result<Value, RuntimeError> {
        Ok(Value::Bool(match (self, other) {
            (Value::String(s), Value::String(o)) => s == o,
            (Value::Number(s), Value::Number(o)) => s == o,
            (Value::Bool(s), Value::Bool(o)) => s == o,
            (Value::Null, Value::Null) => true,
            (Value::Array(Array(s)), Value::Array(Array(o))) => s == o,
            (Value::Function(s), Value::Function(o)) => Rc::ptr_eq(s, o),
            (Value::Void, Value::Void) => true,
            _ => false,
        }))
    }

    pub fn neq(&self, other: &Value) -> Result<Value, RuntimeError> {
        match self.eq(other)? {
            Value::Bool(b) => Ok(Value::Bool(!b)),
            _ => unreachable!("eq can evaluate only to bool"),
        }
    }

    pub fn nullish(&self) -> bool {
        matches!(self, Value::Void | Value::Null)
    }

    pub fn blank(&self) -> bool {
        match self {
            Value::String(s) if s.is_empty() => true,
            Value::Null | Value::Void => true,
            _ => false,
        }
    }

    pub fn value_type(&self) -> ValueType {
        match self {
            Value::String(_) => ValueType::String,
            Value::Number(Number::Int(_)) => ValueType::Integer,
            Value::Number(Number::Float(_)) => ValueType::Float,
            Value::Bool(_) => ValueType::Bool,
            Value::Object(_) => ValueType::Object,
            Value::Array(_) => ValueType::Array,
            Value::Function(_) => ValueType::Function,
            Value::Null => ValueType::Null,
            Value::Void => ValueType::Void,
            Value::Closure(_) => ValueType::Function,
        }
    }

    /// Get a useful representation of the value in a string
    ///
    /// Values that can't be useful in string conversion are ignored
    pub fn stringify(&self) -> String {
        match self {
            Value::String(v) => v.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            _ => String::new(),
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
                serde_json::Value::Number(number) => Self::Number(match number.as_i64() {
                    Some(i) => Number::Int(i),
                    // TODO: handle float conversion failure
                    None => Number::Float(number.as_f64().unwrap_or_default()),
                }),
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

    impl Value {
        pub fn into_json(self) -> Option<serde_json::Value> {
            Some(match self {
                Value::String(s) => serde_json::Value::String(s),
                Value::Number(Number::Float(n)) => {
                    serde_json::Value::Number(serde_json::Number::from_f64(n)?)
                }
                Value::Number(Number::Int(n)) => {
                    serde_json::Value::Number(serde_json::Number::from_i128(n as i128)?)
                }
                Value::Bool(b) => serde_json::Value::Bool(b),
                Value::Null => serde_json::Value::Null,
                Value::Array(Array(items)) => serde_json::Value::Array(
                    items.into_iter().filter_map(Value::into_json).collect(),
                ),
                Value::Object(Object(fields)) => {
                    // objects are hash maps, so sort to keep a rendered object deterministic.
                    let mut fields: Vec<_> = fields.into_iter().collect();
                    fields.sort_by(|a, b| a.0.cmp(&b.0));
                    serde_json::Value::Object(
                        fields
                            .into_iter()
                            .filter_map(|(k, v)| Some((k, Value::into_json(v)?)))
                            .collect(),
                    )
                }
                Value::Function(_) | Value::Closure(_) | Value::Void => return None,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::value::Number;

    #[test]
    fn int_cmp() {
        // above 2^53 both values round to the same f64
        let n = 9007199254740993;
        assert!(Number::Int(n) > Number::Int(n - 1));
        assert!(Number::Int(n - 1) < Number::Int(n));
        assert!(Number::Int(n) >= Number::Int(n));
    }
}
