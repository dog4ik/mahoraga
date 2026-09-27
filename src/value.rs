use std::{collections::HashMap, fmt::Display, rc::Rc};

use crate::Function;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Object(pub HashMap<String, Value>);

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Array(pub Vec<Value>);

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

#[derive(Debug, Clone)]
pub enum Number {
    Float(f64),
    Int(i64),
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
        self.as_f64().partial_cmp(&other.as_f64())
    }
}

macro_rules! impl_op {
    ($trait: ident, $method: ident, $sign: tt) => {
        impl std::ops::$trait for Number {
            type Output = Self;

            fn $method(self, rhs: Self) -> Self::Output {
                match (self, rhs) {
                    (Number::Float(lhs), Number::Float(rhs)) => Number::Float(lhs $sign rhs),
                    (Number::Float(lhs), Number::Int(rhs)) => Number::Float(lhs $sign rhs as f64),
                    (Number::Int(lhs), Number::Float(rhs)) => Number::Float(lhs as f64 $sign rhs),
                    (Number::Int(lhs), Number::Int(rhs)) => Number::Int(lhs $sign rhs),
                }
            }
        }
    };
}

impl_op!(Sub, sub, -);
impl_op!(Add, add, +);
impl_op!(Div, div, /);
impl_op!(Mul, mul, *);

// impl Add for Number {
//     type Output = Self;
//
//     fn add(self, rhs: Self) -> Self::Output {
//         match (self, rhs) {
//             (Number::Float(lhs), Number::Float(rhs)) => Number::Float(lhs + rhs),
//             (Number::Float(lhs), Number::Int(rhs)) => Number::Float(lhs + rhs as f64),
//             (Number::Int(lhs), Number::Float(rhs)) => Number::Float(lhs as f64 + rhs),
//             (Number::Int(lhs), Number::Int(rhs)) => Number::Int(lhs + rhs),
//         }
//     }
// }

impl From<f64> for Number {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<f32> for Number {
    fn from(value: f32) -> Self {
        Self::Float(value.into())
    }
}

macro_rules! impl_from_int {
    ($($num: ident),+) => {
        $(
        impl From<$num> for Number {
            fn from(val: $num) -> Self {
                Self::Int(val as i64)
            }
        }
        )*
    };
}

macro_rules! impl_try_from_int {
    ($($num: ident),+) => {
        $(
        impl TryFrom<$num> for Number {
            type Error = $crate::Error;

            fn try_from(val: $num) -> std::result::Result<Self, $crate::Error> {
                match i64::try_from(val) {
                    Ok(v) => Ok(Number::Int(v)),
                    Err(e) => Err($crate::Error::new(format!("failed to convert to int: {}", e)))
                }
            }
        }
        )*
    };
}

macro_rules! impl_into_int {
    ($($num: ident),+) => {
        $(
        impl From<Number> for $num {
            fn from(val: Number) -> Self {
                match val {
                    Number::Float(f) => f as $num,
                    Number::Int(i) => i as $num
                }
            }
        }
        )*
    };
}

impl_from_int!(u8, u16, u32, i8, i16, i32, i64, isize);
impl_try_from_int!(u64, u128, i128, usize);
impl_into_int!(
    f32, f64, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
);

impl Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Number::Float(n) => write!(f, "{}.{}", n.trunc(), n.fract()),
            Number::Int(i) => write!(f, "{i}"),
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

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl<'a> From<&'a str> for Value {
    fn from(value: &'a str) -> Self {
        Self::String(value.to_owned())
    }
}

macro_rules! impl_from_num_for_value {
    ($($num: ident),+) => {
        $(
        impl From<$num> for Value {
            fn from(val: $num) -> Self {
                Self::Number(val.into())
            }
        }
        )*
    };
}

impl_from_num_for_value!(f32, f64, u8, u16, u32, i8, i16, i32, i64, isize);

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<Rc<Function>> for Value {
    fn from(value: Rc<Function>) -> Self {
        Self::Function(value)
    }
}

impl From<Function> for Value {
    fn from(value: Function) -> Self {
        Self::Function(Rc::new(value))
    }
}

impl From<HashMap<String, Value>> for Value {
    fn from(value: HashMap<String, Value>) -> Self {
        Self::Object(Object(value))
    }
}

impl<T> From<Option<T>> for Value
where
    T: Into<Value>,
{
    fn from(value: Option<T>) -> Self {
        match value {
            Some(v) => v.into(),
            None => Self::Void,
        }
    }
}

impl<T> From<Vec<T>> for Value
where
    T: Into<Value>,
{
    fn from(value: Vec<T>) -> Self {
        Self::Array(Array(value.into_iter().map(Into::into).collect()))
    }
}

impl TryFrom<Value> for usize {
    type Error = crate::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Number(n) => Ok(n.into()),
            _ => Err(crate::Error::new("expected number value")),
        }
    }
}

impl TryFrom<Value> for f64 {
    type Error = crate::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Number(Number::Float(f)) => Ok(f),
            Value::Number(Number::Int(i)) => Ok(i as f64),
            _ => Err(expected(ValueType::Float, &value)),
        }
    }
}

impl TryFrom<Value> for i64 {
    type Error = crate::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Number(Number::Int(t)) => Ok(t),
            _ => Err(expected(ValueType::Integer, &value)),
        }
    }
}

impl TryFrom<Value> for u32 {
    type Error = crate::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Number(number) => Ok(number.into()),
            _ => Err(crate::Error::new("expected number")),
        }
    }
}

impl TryFrom<Value> for bool {
    type Error = crate::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Bool(b) => Ok(b),
            _ => Err(expected(ValueType::Bool, &value)),
        }
    }
}

impl TryFrom<Value> for Object {
    type Error = crate::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Object(o) => Ok(o),
            _ => Err(expected(ValueType::Object, &value)),
        }
    }
}

impl TryFrom<Value> for Array {
    type Error = crate::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Array(a) => Ok(a),
            _ => Err(expected(ValueType::Array, &value)),
        }
    }
}

impl TryFrom<Value> for Number {
    type Error = crate::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Number(n) => Ok(n),
            _ => Err(crate::Error::new(format!(
                "expected integer or float value, got {}",
                value.value_type()
            ))),
        }
    }
}

fn expected(ty: ValueType, got: &Value) -> crate::Error {
    crate::Error::new(format!("expected {ty} value, got {}", got.value_type()))
}

impl TryFrom<Value> for String {
    type Error = crate::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::String(s) => Ok(s),
            _ => Err(crate::Error::new(format!(
                "expected string value, got {value}",
            ))),
        }
    }
}

impl TryFrom<Value> for Rc<Function> {
    type Error = crate::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Function(f) => Ok(f),
            _ => Err(crate::Error::new(format!(
                "expected function value, got {}",
                value.value_type()
            ))),
        }
    }
}

impl<T> TryFrom<Value> for Vec<T>
where
    T: TryFrom<Value>,
    crate::Error: From<<T as TryFrom<Value>>::Error>,
{
    type Error = crate::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Array(Array(array)) => array
                .into_iter()
                .map(|v| T::try_from(v).map_err(crate::Error::from))
                .collect::<crate::Result<Vec<T>>>(),
            _ => Err(crate::Error::new("expected array value")),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "\"{s}\""),
            Value::Number(n) => write!(f, "{n}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Object(obj) => write!(f, "{obj:?}"),
            Value::Array(arr) => write!(f, "{arr:?}"),
            Value::Function(fun) => write!(f, "{fun}"),
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
            Value::Number(Number::Int(n)) => *n != 0,
            Value::Number(Number::Float(n)) => *n != 0.,
            Value::Bool(b) => *b,
            Value::Null | Value::Void => false,
            Self::Object(Object(obj)) => !obj.is_empty(),
            Self::Array(Array(arr)) => !arr.is_empty(),
            Self::Function(_) => true,
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
            (Value::Function(s), Value::Function(o)) => Rc::ptr_eq(s, &o),
            (Value::Void, Value::Void) => true,
            _ => false,
        }))
    }

    pub fn neq(&self, other: Value) -> Result<Value, crate::Error> {
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
            Value::Number(Number::Float(_)) => ValueType::Integer,
            Value::Bool(_) => ValueType::Bool,
            Value::Object(_) => ValueType::Object,
            Value::Array(_) => ValueType::Array,
            Value::Function(_) => ValueType::Function,
            Value::Null => ValueType::Null,
            Value::Void => ValueType::Void,
        }
    }

    /// Get a useful representation of the value in a string
    ///
    /// Values that can't be useful in string conversion are ignored
    pub fn stringify(self) -> String {
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
                Value::Function(_) | Value::Void => return None,
            })
        }
    }
}
