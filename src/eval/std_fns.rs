use std::{collections::HashMap, rc::Rc};

use crate::{
    Value,
    eval::{FunctionCallError, fns::Function},
    value::Number,
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
                name: $name,
                help: Some($help),
                f: Box::new($callable as fn($($arg),*) -> ::std::result::Result<$crate::Value, $crate::FunctionCallError>),
            }),
        )
    };
}

pub fn std_fns() -> HashMap<&'static str, Rc<Function>> {
    [
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
        declare_fn!(to_i(Number), "Convert number to integer"),
        declare_fn!(to_f(Number), "Convert number to float"),
        declare_fn!(to_s(Value), "Convert any value to string representation"),
    ]
    .into_iter()
    .collect()
}

pub fn to_uppercase(val: String) -> Result<Value, FunctionCallError> {
    Ok(Value::String(val.to_uppercase()))
}

pub fn concat(val: Vec<Value>) -> Result<Value, FunctionCallError> {
    Ok(val.iter().map(Value::stringify).collect::<String>().into())
}

pub fn join(val: Vec<Value>, separator: String) -> Result<Value, FunctionCallError> {
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

pub fn trim(val: String) -> Result<Value, FunctionCallError> {
    Ok(Value::String(val.trim().to_owned()))
}

pub fn trim_start(val: String) -> Result<Value, FunctionCallError> {
    Ok(Value::String(val.trim_start().to_owned()))
}

pub fn trim_end(val: String) -> Result<Value, FunctionCallError> {
    Ok(Value::String(val.trim_end().to_owned()))
}

pub fn to_i(val: Number) -> Result<Value, FunctionCallError> {
    match val {
        Number::Float(f) => Ok(Value::Number(Number::Int(f as i64))),
        Number::Int(_) => Ok(Value::Number(val)),
    }
}

pub fn to_f(val: Number) -> Result<Value, FunctionCallError> {
    match val {
        Number::Float(_) => Ok(Value::Number(val)),
        Number::Int(i) => Ok(Value::Number(Number::Float(i as f64))),
    }
}

pub fn to_s(val: Value) -> Result<Value, FunctionCallError> {
    Ok(Value::String(val.stringify()))
}

pub fn to_lowercase(val: String) -> Result<Value, FunctionCallError> {
    Ok(Value::String(val.to_lowercase()))
}

pub fn blank_as_null(val: Value) -> Result<Value, FunctionCallError> {
    Ok(if val.blank() { Value::Null } else { val })
}

pub fn blank_as_void(val: Value) -> Result<Value, FunctionCallError> {
    Ok(if val.blank() { Value::Void } else { val })
}

pub fn void_as_null(val: Value) -> Result<Value, FunctionCallError> {
    Ok(match val {
        Value::Void => Value::Null,
        _ => val,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Args, Callable, Env, eval::FunctionCallError, eval_str, value::Object};

    mod scaling {
        use crate::{eval::FunctionCallError, value::Number};

        pub fn scale(v: f64, by: u32) -> Result<crate::Value, FunctionCallError> {
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
        assert_eq!(f.name, "scale");
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
        assert!(err.to_string().contains("expected float value"), "{err}");
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
            fn call(&self, args: Args) -> Result<Value, FunctionCallError> {
                if args.0.first().is_some_and(Value::blank) {
                    return Ok(Value::Void);
                }
                self.0.call(args)
            }
            fn arity(&self) -> usize {
                self.0.arity()
            }
        }

        let mut env = Env::std();
        env.fns.insert(
            "upper",
            Rc::new(Function {
                name: "upper",
                help: None,
                f: Box::new(SkipBlank(Box::new(
                    to_uppercase as fn(String) -> Result<Value, FunctionCallError>,
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
