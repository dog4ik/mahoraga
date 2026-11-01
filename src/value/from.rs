use std::{collections::HashMap, rc::Rc};

use crate::{
    ArgConversionError, Function, Value,
    eval::{closure::Closure, fns::TryFromArgument},
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

impl_from_int!(u8, u16, u32, i8, i16, i32, i64, isize);
impl_try_from_int!(u64, u128, i128, usize);

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

impl TryFromArgument for Value {
    fn parse_argument(value: Value) -> Result<Self, crate::ArgConversionError> {
        Ok(value)
    }
}

impl TryFromArgument for usize {
    fn parse_argument(value: Value) -> Result<Self, crate::ArgConversionError> {
        match value {
            Value::Number(Number::Float(f)) if f >= 0. && f.is_finite() => Ok(f as usize),
            Value::Number(Number::Int(i)) => Ok(usize::try_from(i)?),
            Value::Number(_) => Err(ArgConversionError::NumberOverflow),
            _ => Err(ArgConversionError::UnexpectedArgumentType {
                got: value.value_type(),
                expected: &[ValueType::Integer],
            }),
        }
    }
}

impl TryFromArgument for Closure {
    fn parse_argument(value: Value) -> Result<Self, crate::ArgConversionError> {
        match value {
            Value::Closure(n) => Ok(n),
            _ => Err(ArgConversionError::UnexpectedArgumentType {
                got: value.value_type(),
                expected: &[ValueType::Function],
            }),
        }
    }
}

impl TryFromArgument for f64 {
    fn parse_argument(value: Value) -> Result<Self, crate::ArgConversionError> {
        match value {
            Value::Number(Number::Float(f)) => Ok(f),
            Value::Number(Number::Int(i)) => Ok(i as f64),
            _ => Err(expected(&[ValueType::Float], &value)),
        }
    }
}

impl TryFromArgument for i64 {
    fn parse_argument(value: Value) -> Result<Self, crate::ArgConversionError> {
        match value {
            Value::Number(Number::Int(t)) => Ok(t),
            Value::Number(Number::Float(f)) if f.fract() == 0. => Ok(f as i64),
            _ => Err(expected(&[ValueType::Integer], &value)),
        }
    }
}

impl TryFromArgument for u32 {
    fn parse_argument(value: Value) -> Result<Self, crate::ArgConversionError> {
        match value {
            Value::Number(Number::Int(i)) => Ok(u32::try_from(i)?),
            Value::Number(Number::Float(f))
                if f.fract() == 0.0 && f >= 0.0 && f <= u32::MAX as f64 =>
            {
                Ok(f as u32)
            }
            _ => Err(expected(&[ValueType::Integer, ValueType::Float], &value)),
        }
    }
}

impl TryFromArgument for bool {
    fn parse_argument(value: Value) -> Result<Self, crate::ArgConversionError> {
        match value {
            Value::Bool(b) => Ok(b),
            _ => Err(expected(&[ValueType::Bool], &value)),
        }
    }
}

impl TryFromArgument for Object {
    fn parse_argument(value: Value) -> Result<Self, crate::ArgConversionError> {
        match value {
            Value::Object(o) => Ok(o),
            _ => Err(expected(&[ValueType::Object], &value)),
        }
    }
}

impl TryFromArgument for Array {
    fn parse_argument(value: Value) -> Result<Self, crate::ArgConversionError> {
        match value {
            Value::Array(a) => Ok(a),
            _ => Err(expected(&[ValueType::Array], &value)),
        }
    }
}

impl TryFromArgument for Number {
    fn parse_argument(value: Value) -> Result<Self, crate::ArgConversionError> {
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

impl TryFromArgument for String {
    fn parse_argument(value: Value) -> Result<Self, crate::ArgConversionError> {
        match value {
            Value::String(s) => Ok(s),
            _ => Err(expected(&[ValueType::String], &value)),
        }
    }
}

impl TryFromArgument for Rc<Function> {
    fn parse_argument(value: Value) -> Result<Self, crate::ArgConversionError> {
        match value {
            Value::Function(f) => Ok(f),
            _ => Err(expected(&[ValueType::Function], &value)),
        }
    }
}

impl<T> TryFromArgument for Vec<T>
where
    T: TryFromArgument,
{
    fn parse_argument(value: Value) -> Result<Self, crate::ArgConversionError> {
        match value {
            Value::Array(Array(array)) => array
                .into_iter()
                .map(T::parse_argument)
                .collect::<std::result::Result<Vec<T>, ArgConversionError>>(
            ),
            _ => Err(ArgConversionError::UnexpectedArgumentType {
                got: value.value_type(),
                expected: &[ValueType::Array],
            }),
        }
    }
}

impl TryFromArgument for super::CallableValue {
    fn parse_argument(value: Value) -> Result<Self, crate::ArgConversionError> {
        match value {
            Value::Function(function) => Ok(Self::ExternalFunction(function)),
            Value::Closure(closure) => Ok(Self::Closure(closure)),
            Value::VmClosure(closure) => Ok(Self::VmClosure(closure)),
            _ => Err(ArgConversionError::UnexpectedArgumentType {
                got: value.value_type(),
                expected: &[ValueType::Function],
            }),
        }
    }
}
