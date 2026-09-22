use std::{collections::HashMap, rc::Rc};

use crate::{Value, eval::fns::Function};

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
                f: Box::new($callable as fn($($arg),*) -> $crate::Result<$crate::Value>),
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
            void_as_null(Value),
            "Convert void value to null, used to emit explicit nulls"
        ),
    ]
    .into_iter()
    .collect()
}

pub fn to_uppercase(val: String) -> crate::Result<Value> {
    Ok(Value::String(val.to_uppercase()))
}

pub fn concat(val: Vec<Value>) -> crate::Result<Value> {
    Ok(val
        .into_iter()
        .map(Value::stringify)
        .collect::<String>()
        .into())
}

pub fn to_lowercase(val: String) -> crate::Result<Value> {
    Ok(Value::String(val.to_lowercase()))
}

pub fn blank_as_null(val: Value) -> crate::Result<Value> {
    Ok(if val.blank() { Value::Null } else { val })
}

pub fn blank_as_void(val: Value) -> crate::Result<Value> {
    Ok(if val.blank() { Value::Void } else { val })
}

pub fn void_as_null(val: Value) -> crate::Result<Value> {
    Ok(match val {
        Value::Void => Value::Null,
        _ => val,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Args, Callable, Env, eval_str, value::Object};

    mod scaling {
        pub fn scale(v: f64, by: u32) -> crate::Result<crate::Value> {
            Ok(crate::Value::Number(v * f64::from(by)))
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
        assert_eq!(eval_str("3 | scale(4)", &env).unwrap(), Value::Number(12.));
        assert_eq!(eval_str("scale(3, 4)", &env).unwrap(), Value::Number(12.));
    }

    #[test]
    fn typed_arguments_reject_the_wrong_value() {
        let err = eval_str("'x' | scale(4)", &env()).unwrap_err();
        assert!(err.message.contains("expected number value"), "{err}");
        let err = eval_str("3 | scale(1.5)", &env()).unwrap_err();
        assert!(err.message.contains("not a whole number"), "{err}");
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

    /// `Args` is public, so a caller outside the crate can decorate a function.
    #[test]
    fn callable_can_be_implemented_from_outside() {
        struct SkipBlank(Box<dyn Callable>);
        impl Callable for SkipBlank {
            fn call(&self, args: Args) -> crate::Result<Value> {
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
                    to_uppercase as fn(String) -> crate::Result<Value>,
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
