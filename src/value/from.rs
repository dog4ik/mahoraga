use std::{collections::HashMap, rc::Rc};

use crate::{
    ArgConversionError, Function, Value,
    eval::closure::Closure,
    value::{Array, Number, Object, ValueType},
};

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
            type Error = $crate::eval::ArgConversionError;

            fn try_from(val: $num) -> ::std::result::Result<Self, Self::Error> {
                match i64::try_from(val) {
                    Ok(v) => Ok(Number::Int(v)),
                    Err(e) => Err($crate::eval::ArgConversionError::IntConversionError(e))
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
    type Error = ArgConversionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Number(n) => Ok(n.into()),
            _ => Err(ArgConversionError::UnexpectedArgumentType {
                got: value.value_type(),
                expected: &[ValueType::Integer],
            }),
        }
    }
}

impl TryFrom<Value> for Closure {
    type Error = ArgConversionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Closure(n) => Ok(n),
            _ => Err(ArgConversionError::UnexpectedArgumentType {
                got: value.value_type(),
                expected: &[ValueType::Function],
            }),
        }
    }
}

impl TryFrom<Value> for f64 {
    type Error = ArgConversionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Number(Number::Float(f)) => Ok(f),
            Value::Number(Number::Int(i)) => Ok(i as f64),
            _ => Err(expected(&[ValueType::Float], &value)),
        }
    }
}

impl TryFrom<Value> for i64 {
    type Error = ArgConversionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Number(Number::Int(t)) => Ok(t),
            _ => Err(expected(&[ValueType::Integer], &value)),
        }
    }
}

impl TryFrom<Value> for u32 {
    type Error = ArgConversionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Number(number) => Ok(number.into()),
            _ => Err(expected(&[ValueType::Integer, ValueType::Float], &value)),
        }
    }
}

impl TryFrom<Value> for bool {
    type Error = ArgConversionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Bool(b) => Ok(b),
            _ => Err(expected(&[ValueType::Bool], &value)),
        }
    }
}

impl TryFrom<Value> for Object {
    type Error = ArgConversionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Object(o) => Ok(o),
            _ => Err(expected(&[ValueType::Object], &value)),
        }
    }
}

impl TryFrom<Value> for Array {
    type Error = ArgConversionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Array(a) => Ok(a),
            _ => Err(expected(&[ValueType::Array], &value)),
        }
    }
}

impl TryFrom<Value> for Number {
    type Error = ArgConversionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Number(n) => Ok(n),
            _ => Err(expected(&[ValueType::Integer, ValueType::Float], &value)),
        }
    }
}

fn expected(ty: &'static [ValueType], got: &Value) -> ArgConversionError {
    ArgConversionError::UnexpectedArgumentType {
        got: got.value_type(),
        expected: ty,
    }
}

impl TryFrom<Value> for String {
    type Error = ArgConversionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::String(s) => Ok(s),
            _ => Err(expected(&[ValueType::String], &value)),
        }
    }
}

impl TryFrom<Value> for Rc<Function> {
    type Error = ArgConversionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Function(f) => Ok(f),
            _ => Err(expected(&[ValueType::Function], &value)),
        }
    }
}

impl<T> TryFrom<Value> for Vec<T>
where
    T: TryFrom<Value>,
    crate::eval::ArgConversionError: From<<T as TryFrom<Value>>::Error>,
{
    type Error = crate::eval::ArgConversionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Array(Array(array)) => array
                .into_iter()
                .map(|v| T::try_from(v).map_err(ArgConversionError::from))
                .collect::<std::result::Result<Vec<T>, ArgConversionError>>(),
            _ => Err(ArgConversionError::UnexpectedArgumentType {
                got: value.value_type(),
                expected: &[ValueType::Array],
            }),
        }
    }
}

impl TryFrom<Value> for super::CallableValue {
    type Error = ArgConversionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Function(function) => Ok(Self::ExternalFunction(function)),
            Value::Closure(closure) => Ok(Self::Closure(closure)),
            _ => Err(ArgConversionError::UnexpectedArgumentType {
                got: value.value_type(),
                expected: &[ValueType::Function],
            }),
        }
    }
}
