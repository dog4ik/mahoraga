use std::{collections::HashMap, rc::Rc};

use crate::{
    Callable, RuntimeError, Value,
    eval::{FunctionCallError, env::RuntimeEnv, fns::Function},
    value::{Array, CallableValue, Number, Object, ValueType},
};

#[macro_export]
macro_rules! declare_fn {
    ($callable:ident($($arg:ty),*), $help:literal) => {
        $crate::declare_fn!(stringify!($callable), $callable, ($($arg),*), $help)
    };
    // An explicit name, for a function that lives in a module or is registered
    // under a name it is not spelled with.
    ($name:expr, $callable:path, ($($arg:ty),*), $help:literal) => {
        (
            $name,
            ::std::rc::Rc::new($crate::Function {
                name: ::std::rc::Rc::from($name),
                help: Some($help),
                f: Box::new($callable as fn(&mut $crate::RuntimeEnv<'_>, $($arg),*) -> ::std::result::Result<$crate::Value, $crate::RuntimeError>),
            }),
        )
    };
}

pub fn std_fns() -> HashMap<&'static str, Rc<Function>> {
    [
        declare_fn!(len(Value), "Length of the value, 0 if not applicable"),
        declare_fn!(clamp(Number, i64, i64), "Clamp the value between x and y"),
        declare_fn!(
            sum(Vec<Value>),
            "Sum all the elements of the array, non number elements are ignored"
        ),
        declare_fn!(
            min(Vec<Value>),
            "Get min element in array, non number elements are ignored"
        ),
        declare_fn!(
            max(Vec<Value>),
            "Get max element in array, non number elements are ignored"
        ),
        declare_fn!(
            pluck(Vec<Value>, String),
            "Create a new array from indexing into every element object"
        ),
        declare_fn!(log_10(Number), "Calculate log 10 of the number"),
        declare_fn!(cos(Number), "Calculate cos of the value"),
        declare_fn!(sin(Number), "Calculate sin of the value"),
        declare_fn!(to_uppercase(String), "Convert string to uppercase"),
        declare_fn!(to_lowercase(String), "Convert string to lowercase"),
        declare_fn!(blank_as_null(Value), "Convert any blank value to null"),
        declare_fn!(blank_as_void(Value), "Convert any blank value to void"),
        declare_fn!(
            concat(Vec<Value>),
            "Concatenate an array of values into a string"
        ),
        declare_fn!(
            join(Vec<Value>, String),
            "Concatenate an array of values into a string using a separator"
        ),
        declare_fn!(
            trim(String),
            "Returns a string with leading and trailing whitespace removed"
        ),
        declare_fn!(
            trim_end(String),
            "Returns a string slice with trailing whitespace removed"
        ),
        declare_fn!(
            trim_start(String),
            "Returns a string slice with leading whitespace removed"
        ),
        declare_fn!(
            void_as_null(Value),
            "Convert void value to null, used to emit explicit nulls"
        ),
        declare_fn!(to_i(Value), "Convert number to integer"),
        declare_fn!(to_f(Value), "Convert number to float"),
        declare_fn!(to_s(Value), "Convert any value to string representation"),
        declare_fn!(map(Vec<Value>, CallableValue), "Map elements of an array"),
        declare_fn!(
            filter_map(Vec<Value>, CallableValue),
            "Filter and transform elements in the array"
        ),
        declare_fn!(
            filter(Vec<Value>, CallableValue),
            "Filter and transform elements in the array"
        ),
        declare_fn!(
            find(Vec<Value>, CallableValue),
            "Searches for an element of an array that satisfies a predicate"
        ),
        declare_fn!(
            find_map(Vec<Value>, CallableValue),
            "Applies function to the elements of array and returns the first truthy result"
        ),
        declare_fn!(
            replace_all(String, String, String),
            "Replaces all occurrences of the second argument with the third argument"
        ),
    ]
    .into_iter()
    .collect()
}

pub fn clamp(
    _: &mut RuntimeEnv<'_>,
    number: Number,
    min: i64,
    max: i64,
) -> Result<Value, RuntimeError> {
    Ok(Value::Number(match number {
        Number::Float(f) => f.clamp(min as f64, max as f64).into(),
        Number::Int(i) => i.clamp(min, max).into(),
    }))
}

pub fn sum(_: &mut RuntimeEnv<'_>, elements: Vec<Value>) -> Result<Value, RuntimeError> {
    Ok(Value::Number(
        elements
            .into_iter()
            .filter_map(|v| match v {
                Value::Number(number) => Some(number),
                _ => None,
            })
            .fold(Number::Int(0), |acc, n| acc.add(n).unwrap_or_default()),
    ))
}

pub fn max(_: &mut RuntimeEnv<'_>, elements: Vec<Value>) -> Result<Value, RuntimeError> {
    match elements
        .into_iter()
        .filter_map(|v| match v {
            Value::Number(number) => Some(number),
            _ => None,
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    {
        Some(n) => Ok(Value::Number(n)),
        None => Ok(Value::Void),
    }
}

pub fn min(_: &mut RuntimeEnv<'_>, elements: Vec<Value>) -> Result<Value, RuntimeError> {
    match elements
        .into_iter()
        .filter_map(|v| match v {
            Value::Number(number) => Some(number),
            _ => None,
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    {
        Some(n) => Ok(Value::Number(n)),
        None => Ok(Value::Void),
    }
}

pub fn pluck(
    _: &mut RuntimeEnv<'_>,
    elements: Vec<Value>,
    key: String,
) -> Result<Value, RuntimeError> {
    let arr: Vec<Value> = elements
        .into_iter()
        .filter_map(|v| match v {
            Value::Object(Object(mut obj)) => Some(obj.remove(&key)?),
            _ => None,
        })
        .collect();
    Ok(Value::Array(Array(arr)))
}

pub fn to_uppercase(_: &mut RuntimeEnv<'_>, val: String) -> Result<Value, RuntimeError> {
    Ok(Value::String(val.to_uppercase()))
}

pub fn concat(_: &mut RuntimeEnv<'_>, val: Vec<Value>) -> Result<Value, RuntimeError> {
    Ok(val.iter().map(Value::stringify).collect::<String>().into())
}

pub fn log_10(_: &mut RuntimeEnv<'_>, val: Number) -> Result<Value, RuntimeError> {
    Ok(Value::Number(
        match val {
            Number::Float(f) => f,
            Number::Int(i) => i as f64,
        }
        .log10()
        .into(),
    ))
}

pub fn cos(_: &mut RuntimeEnv<'_>, val: Number) -> Result<Value, RuntimeError> {
    Ok(Value::Number(
        match val {
            Number::Float(f) => f,
            Number::Int(i) => i as f64,
        }
        .cos()
        .into(),
    ))
}

pub fn sin(_: &mut RuntimeEnv<'_>, val: Number) -> Result<Value, RuntimeError> {
    Ok(Value::Number(
        match val {
            Number::Float(f) => f,
            Number::Int(i) => i as f64,
        }
        .sin()
        .into(),
    ))
}

pub fn join(
    _: &mut RuntimeEnv<'_>,
    val: Vec<Value>,
    separator: String,
) -> Result<Value, RuntimeError> {
    let mut out = String::new();
    let len = val.len();
    if len == 0 {
        return Ok(Value::String(String::new()));
    }
    for val in val.iter().map(Value::stringify).take(len - 1) {
        out += &val;
        out += &separator;
    }

    out += &val[len - 1].stringify();

    Ok(Value::String(out))
}

pub fn trim(_: &mut RuntimeEnv<'_>, val: String) -> Result<Value, RuntimeError> {
    Ok(Value::String(val.trim().to_owned()))
}

pub fn len(_: &mut RuntimeEnv<'_>, val: Value) -> Result<Value, RuntimeError> {
    Ok(Value::Number(Number::Int(match val {
        Value::String(s) => s.chars().count(),
        Value::Object(object) => object.len(),
        Value::Array(array) => array.len(),
        Value::Number(_)
        | Value::Bool(_)
        | Value::Function(_)
        | Value::Closure(_)
        | Value::VmClosure(_)
        | Value::Null
        | Value::Void => 0,
    } as i64)))
}

pub fn trim_start(_: &mut RuntimeEnv<'_>, val: String) -> Result<Value, RuntimeError> {
    Ok(Value::String(val.trim_start().to_owned()))
}

pub fn trim_end(_: &mut RuntimeEnv<'_>, val: String) -> Result<Value, RuntimeError> {
    Ok(Value::String(val.trim_end().to_owned()))
}

pub fn to_i(_: &mut RuntimeEnv<'_>, val: Value) -> Result<Value, RuntimeError> {
    match val {
        Value::Number(Number::Float(f)) => Ok(Value::Number(Number::Int(f as i64))),
        Value::Number(val @ Number::Int(_)) => Ok(Value::Number(val)),
        Value::String(s) => {
            let value = match s.parse::<i64>() {
                Ok(v) => v,
                Err(_) => s.parse::<f64>().map(|v| v as i64).map_err(|e| {
                    FunctionCallError::InvalidArgument {
                        argument_pos: 1,
                        err: super::ArgConversionError::FloatConversionError(e),
                    }
                })?,
            };
            Ok(Value::Number(Number::Int(value)))
        }
        _ => Err(FunctionCallError::InvalidArgument {
            argument_pos: 1,
            err: super::ArgConversionError::UnexpectedArgumentType {
                got: val.value_type(),
                expected: &[ValueType::Float, ValueType::Integer, ValueType::String],
            },
        })?,
    }
}

pub fn to_f(_: &mut RuntimeEnv<'_>, val: Value) -> Result<Value, RuntimeError> {
    match val {
        Value::Number(val @ Number::Float(_)) => Ok(Value::Number(val)),
        Value::Number(Number::Int(i)) => Ok(Value::Number(Number::Float(i as f64))),
        Value::String(s) => {
            let value = match s.parse::<f64>() {
                Ok(v) => v,
                Err(e) => s.parse::<i64>().map(|v| v as f64).map_err(|_| {
                    FunctionCallError::InvalidArgument {
                        argument_pos: 1,
                        err: super::ArgConversionError::FloatConversionError(e),
                    }
                })?,
            };
            Ok(Value::Number(Number::Float(value)))
        }
        _ => Err(FunctionCallError::InvalidArgument {
            argument_pos: 1,
            err: super::ArgConversionError::UnexpectedArgumentType {
                got: val.value_type(),
                expected: &[ValueType::Float, ValueType::Integer, ValueType::String],
            },
        })?,
    }
}

pub fn to_s(_: &mut RuntimeEnv<'_>, val: Value) -> Result<Value, RuntimeError> {
    Ok(Value::String(val.stringify()))
}

pub fn to_lowercase(_: &mut RuntimeEnv<'_>, val: String) -> Result<Value, RuntimeError> {
    Ok(Value::String(val.to_lowercase()))
}

pub fn blank_as_null(_: &mut RuntimeEnv<'_>, val: Value) -> Result<Value, RuntimeError> {
    Ok(if val.blank() { Value::Null } else { val })
}

pub fn blank_as_void(_: &mut RuntimeEnv<'_>, val: Value) -> Result<Value, RuntimeError> {
    Ok(if val.blank() { Value::Void } else { val })
}

pub fn void_as_null(_: &mut RuntimeEnv<'_>, val: Value) -> Result<Value, RuntimeError> {
    Ok(match val {
        Value::Void => Value::Null,
        _ => val,
    })
}

pub fn map(
    e: &mut RuntimeEnv<'_>,
    arr: Vec<Value>,
    f: CallableValue,
) -> Result<Value, RuntimeError> {
    Ok(Value::Array(Array(
        arr.into_iter()
            .map(|v| f.call(e, crate::Args(vec![v])))
            .collect::<Result<Vec<_>, _>>()?,
    )))
}

pub fn filter_map(
    e: &mut RuntimeEnv<'_>,
    arr: Vec<Value>,
    f: CallableValue,
) -> Result<Value, RuntimeError> {
    Ok(Value::Array(Array(
        arr.into_iter()
            .filter_map(|v| match f.call(e, crate::Args(vec![v])) {
                Ok(o) if o.truthy() => Some(Ok(o)),
                Ok(_) => None,
                Err(e) => Some(Err(e)),
            })
            .collect::<Result<Vec<_>, _>>()?,
    )))
}

pub fn filter(
    e: &mut RuntimeEnv<'_>,
    arr: Vec<Value>,
    f: CallableValue,
) -> Result<Value, RuntimeError> {
    Ok(Value::Array(Array(
        arr.into_iter()
            .filter_map(|v| match f.call(e, crate::Args(vec![v.clone()])) {
                Ok(o) if o.truthy() => Some(Ok(v)),
                Ok(_) => None,
                Err(e) => Some(Err(e)),
            })
            .collect::<Result<Vec<_>, _>>()?,
    )))
}

pub fn find(
    e: &mut RuntimeEnv<'_>,
    arr: Vec<Value>,
    f: CallableValue,
) -> Result<Value, RuntimeError> {
    Ok(arr
        .into_iter()
        .find_map(|v| match f.call(e, crate::Args(vec![v.clone()])) {
            Ok(o) if o.truthy() => Some(Ok(v)),
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
        .transpose()?
        .unwrap_or(Value::Void))
}

pub fn find_map(
    e: &mut RuntimeEnv<'_>,
    arr: Vec<Value>,
    f: CallableValue,
) -> Result<Value, RuntimeError> {
    Ok(arr
        .into_iter()
        .find_map(|v| match f.call(e, crate::Args(vec![v])) {
            Ok(o) if o.truthy() => Some(Ok(o)),
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
        .transpose()?
        .unwrap_or(Value::Void))
}

pub fn replace_all(
    _: &mut RuntimeEnv<'_>,
    input: String,
    needle: String,
    replacement: String,
) -> Result<Value, RuntimeError> {
    Ok(input.replace(&needle, &replacement).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Args, Callable, Env, RuntimeError, eval_str, value::Object};

    mod scaling {
        use crate::{
            eval::{RuntimeError, env::RuntimeEnv},
            value::Number,
        };

        pub fn scale(
            _: &mut RuntimeEnv<'_>,
            v: f64,
            by: u32,
        ) -> Result<crate::Value, RuntimeError> {
            Ok(crate::Value::Number(Number::Float(v * f64::from(by))))
        }
    }

    fn env() -> Env {
        let mut env = Env::std();
        env.fns.extend([crate::declare_fn!(
            "scale",
            scaling::scale,
            (f64, u32),
            "Multiplies by a whole number"
        )]);
        env
    }

    #[test]
    fn a_named_declaration_registers_under_its_name() {
        let env = env();
        let f = env.fns.get("scale").expect("registered under `scale`");
        assert_eq!(&*f.name, "scale");
        assert_eq!(f.arity(), 2, "the piped input counts");
        assert_eq!(
            eval_str("3 | scale(4)", &env).unwrap(),
            Value::Number(12.0.into())
        );
        assert_eq!(
            eval_str("scale(3, 4)", &env).unwrap(),
            Value::Number(12.0.into())
        );
    }

    #[test]
    fn typed_arguments_reject_the_wrong_value() {
        let err = eval_str("'x' | scale(4)", &env()).unwrap_err();
        assert!(err.to_string().contains("expected [Float]"), "{err}");
    }

    #[test]
    fn containers_and_bools_convert() {
        assert!(bool::try_from(Value::Bool(true)).unwrap());
        assert!(Object::try_from(Value::Object(Object::default())).is_ok());
        assert!(
            crate::value::Array::try_from(Value::Array(crate::value::Array::default())).is_ok()
        );
        assert!(bool::try_from(Value::Void).is_err());
    }

    #[test]
    fn join_empty_array() -> crate::Result<()> {
        assert_eq!(
            Value::String(String::new()),
            eval_str("[] | join(' ')", &Env::std())?
        );
        Ok(())
    }

    #[test]
    fn join_mixed_array() -> crate::Result<()> {
        assert_eq!(
            Value::String(String::from("10.0 10 hello")),
            eval_str("[10.0, 10, 'hello'] | join(' ')", &Env::std())?
        );
        Ok(())
    }

    /// `Args` is public, so a caller outside the crate can decorate a function.
    #[test]
    fn callable_can_be_implemented_from_outside() {
        struct SkipBlank(Box<dyn Callable>);
        impl Callable for SkipBlank {
            fn call(&self, env: &mut RuntimeEnv<'_>, args: Args) -> Result<Value, RuntimeError> {
                if args.0.first().is_some_and(Value::blank) {
                    return Ok(Value::Void);
                }
                self.0.call(env, args)
            }
            fn arity(&self) -> usize {
                self.0.arity()
            }
        }

        let mut env = Env::std();
        env.fns.insert(
            "upper",
            Rc::new(Function {
                name: Rc::from("upper"),
                help: None,
                f: Box::new(SkipBlank(Box::new(
                    to_uppercase as fn(&mut RuntimeEnv<'_>, String) -> Result<Value, RuntimeError>,
                ))),
            }),
        );
        assert_eq!(
            eval_str("'x' | upper", &env).unwrap(),
            Value::String("X".into())
        );
        assert_eq!(eval_str("'' | upper", &env).unwrap(), Value::Void);
        assert_eq!(eval_str("nope | upper", &env).unwrap(), Value::Void);
    }
}
