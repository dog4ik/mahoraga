use crate::{Punct, Span, span::Spanned, value::ValueType};

/// Bytecode evaluation implementation
pub mod bytecode;
pub mod closure;
pub mod env;
pub mod fns;
pub mod stack;
pub mod std_fns;
/// Traverse evaluation implementation
pub mod traverse;

pub use env::Env;

type Result<T> = std::result::Result<T, Spanned<RuntimeError>>;

const MAX_DEPTH: usize = 250;

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("{0}")]
    OperationError(#[from] OpError),
    #[error("object can be indexed only by strings, got {}", .0)]
    InvalidObjectKey(ValueType),
    #[error("array can be indexed only by integers, got {}", .0)]
    InvalidArrayIndex(ValueType),
    #[error("array or object are allowed to be indexed")]
    InvalidIndexOperand,
    #[error("unexpected value type, got {got}, expected {expected}")]
    UnexpectedType { got: ValueType, expected: ValueType },
    #[error("function call failed: {0}")]
    FunctionCallError(#[from] FunctionCallError),
    #[error("Integer overflow")]
    IntegerOverflow,
    #[error("division by zero")]
    DivisionByZero,
    #[error("non-nulish assertion violated")]
    NonNullishAssertion,
    #[error("evaluation depth limit reached")]
    DepthLimitReached,
}

impl RuntimeError {
    pub fn help(&self) -> Option<String> {
        match self {
            RuntimeError::OperationError(op_error) => match op_error {
                OpError::UnsupportedOperands(Punct::Add, ValueType::String, _) => {
                    Some("string can be added only to another string, consider using `to_s` to convert the second operand to string".to_string())
                }
                _ => None,
            },
            RuntimeError::InvalidObjectKey(_) => Some("object can be only keys by string values".to_string()),
            RuntimeError::InvalidArrayIndex(_) => Some("array can only be keyed by integers or rounded floats e.g. 3 or 3.0".to_string()),
            RuntimeError::InvalidIndexOperand => None,
            RuntimeError::UnexpectedType { .. } => None,
            RuntimeError::FunctionCallError(_) => None,
            RuntimeError::IntegerOverflow => None,
            RuntimeError::DivisionByZero => None,
            RuntimeError::NonNullishAssertion => Some("accesses annotated with non nullish operator must not be void or null".to_string()),
            RuntimeError::DepthLimitReached => None,
        }
    }

    pub fn with_span(self, span: Span) -> Spanned<Self> {
        Spanned { inner: self, span }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OpError {
    #[error("unsupported {0} operands: {1} and {2}")]
    UnsupportedOperands(Punct, ValueType, ValueType),
}

#[derive(Debug, thiserror::Error)]
pub enum ArgConversionError {
    #[error("expected {expected:?}, got {got}")]
    UnexpectedArgumentType {
        got: ValueType,
        expected: &'static [ValueType],
    },
    #[error("number overflow")]
    NumberOverflow,
    #[error("{0}")]
    IntConversionError(#[from] std::num::TryFromIntError),
    #[error("{0}")]
    FloatConversionError(#[from] std::num::ParseFloatError),
}

impl From<std::convert::Infallible> for ArgConversionError {
    fn from(value: std::convert::Infallible) -> Self {
        match value {}
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FunctionCallError {
    #[error("function {name} not found")]
    NotFound { name: String },
    #[error("unexpected number of arguments for function {name}, got {got}, expected {expected}")]
    InvalidArity {
        name: String,
        got: usize,
        expected: usize,
    },
    #[error("only functions can be called, got {0}")]
    UncallableType(ValueType),
    #[error("invalid function argument in pos {argument_pos}, {err}")]
    InvalidArgument {
        argument_pos: usize,
        err: ArgConversionError,
    },
    /// Raised by a function implemented outside of mahoraga, e.g. a host callback.
    #[error("{0}")]
    External(String),
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use serde_json::json;

    use super::*;
    use crate::{
        Node, RuntimeEnv, Value,
        parser::parse_expr,
        span::Span,
        value::{Array, Object, ValueType},
    };

    fn eval(node: Node) -> crate::Result<Value> {
        let env = Env::std();
        let mut env = RuntimeEnv::new(&env);
        Ok(traverse::eval(&node, &mut env)?)
    }

    fn eval_str(s: &str) -> crate::Result<Value> {
        let node = parse_expr(s)?;
        let mut env = Env::std();
        match scope() {
            Value::Object(object) => env.attach_object(object),
            _ => panic!("scope return object value"),
        }
        let mut env = RuntimeEnv::new(&env);
        Ok(traverse::eval(&node, &mut env)?)
    }

    fn err(s: &str) -> crate::Error {
        eval(parse_expr(s).unwrap()).unwrap_err()
    }

    #[test]
    fn basic_add() -> crate::Result<()> {
        assert_eq!(eval(parse_expr("2 + 2")?)?, Value::Number(4.0.into()));
        Ok(())
    }

    #[test]
    fn add_and_mult() -> crate::Result<()> {
        assert_eq!(eval(parse_expr("(2 + 2) * 3")?)?, Value::Number(12.into()));
        Ok(())
    }

    #[test]
    fn turnary_true() -> crate::Result<()> {
        assert_eq!(eval(parse_expr("true ? 1 : 2")?)?, Value::Number(1.into()));
        Ok(())
    }

    #[test]
    fn turnary_false() -> crate::Result<()> {
        assert_eq!(eval(parse_expr("false ? 1 : 2")?)?, Value::Number(2.into()));
        Ok(())
    }

    #[test]
    fn nested_turnary_false() -> crate::Result<()> {
        assert_eq!(
            eval(parse_expr(
                "false ? 1 ? 10 + 10 : 0 : \"test\" ? 100 * 125 : 0"
            )?)?,
            Value::Number((100 * 125).into())
        );
        Ok(())
    }

    #[test]
    fn mte() -> crate::Result<()> {
        assert_eq!(eval(parse_expr("11.3 >= 12")?)?, Value::Bool(false));
        assert_eq!(eval(parse_expr("10.0 >= 10")?)?, Value::Bool(true));
        assert_eq!(eval(parse_expr("9.0 >= 10")?)?, Value::Bool(false));
        Ok(())
    }

    #[test]
    fn mt() -> crate::Result<()> {
        assert_eq!(eval(parse_expr("11.3 > 12")?)?, Value::Bool(false));
        assert_eq!(eval(parse_expr("10.0 > 10")?)?, Value::Bool(false));
        assert_eq!(eval(parse_expr("9.0 > 10")?)?, Value::Bool(false));
        Ok(())
    }

    #[test]
    fn eq_with_ternary() -> crate::Result<()> {
        assert_eq!(
            eval(parse_expr("10 > 5 ? true : false")?)?,
            Value::Bool(true)
        );
        Ok(())
    }

    #[test]
    fn eq_with_ternary2() -> crate::Result<()> {
        assert_eq!(
            eval(parse_expr("10 >= 10 ? true : false")?)?,
            Value::Bool(true)
        );
        Ok(())
    }

    fn scope() -> Value {
        json!({
            "payment": { "token": "tok_1", "gateway_amount": 1000, "product": null,
                         "order_number": "ORD-9" },
            "params":  { "phone": null, "customer": { "phone": "0798288410" },
                         "first_name": "John", "last_name": "Doe",
                         "extra_return_param": "_blank_" },
            "settings": { "wallet": "w1", "code": "MPESA", "channel": "Mpesa" },
            "steps":   { "auth": { "access_token": "abc" } },
            "env":     { "callback_url": "https://cb.example/gateway/callback" }
        })
        .into()
    }

    fn ev(src: &str) -> Value {
        eval_str(src).unwrap_or_else(|e| panic!("{src}: {e}"))
    }

    #[test]
    fn reads_nested_paths() {
        assert_eq!(ev("payment.token"), json!("tok_1").into());
        assert_eq!(ev("steps.auth.access_token"), json!("abc").into());
        assert_eq!(
            ev("env.callback_url"),
            json!("https://cb.example/gateway/callback").into()
        );
    }

    #[test]
    fn missing_paths_and_roots_are_void() {
        assert_eq!(ev("payment.nope"), Value::Void);
        assert_eq!(ev("payment.nope.deeper"), Value::Void);
        assert_eq!(ev("nosuchroot.x"), Value::Void);
        assert_eq!(ev("params.customer[\"3\"]"), Value::Void);
    }

    #[test]
    fn null_is_preserved() {
        assert_eq!(ev("params.phone"), Value::Null);
    }

    #[test]
    fn or_skips_null_and_empty() {
        assert_eq!(
            ev("payment.product || payment.order_number || \"Payment\""),
            json!("ORD-9").into()
        );
        // assert_eq!(
        //     ev("payment.product || payment.nope || \"Payment\""),
        //     json!("Payment").into()
        // );
        assert_eq!(
            ev("params.phone || params.customer.phone"),
            json!("0798288410").into()
        );
    }

    #[test]
    fn reproduces_the_scripay_account_name_rule() {
        assert_eq!(
            ev("[params.first_name, params.last_name] | join(' ') | trim"),
            json!("John Doe").into()
        );
    }

    #[test]
    #[ignore = "behavior is not defined yet"]
    fn absence_flows_through_a_pipeline() {
        assert_eq!(ev("payment.product | trim | to_uppercase"), Value::Null);
        assert_eq!(
            ev("payment.product | trim ?? 'fallback'"),
            json!("fallback").into()
        );
    }

    #[test]
    fn function_errors_carry_the_call_span() {
        let err = eval_str("params.customer | trim").unwrap_err();
        assert_eq!(err.span(), Span::new(18..22));
    }

    #[test]
    fn object_and_array_literals_evaluate() {
        assert_eq!(
            ev("{\"a\": payment.token, \"b\": 2}"),
            json!({"a": "tok_1", "b": 2}).into()
        );
        assert_eq!(ev("[1, payment.token]"), json!([1, "tok_1"]).into());
    }

    #[test]
    fn object_null_index_should_produce_void() {
        assert_eq!(ev("{\"a\": payment.token, \"b\": 2}[null]"), Value::Void,);
    }

    #[test]
    fn array_literals_can_be_indexed_and_compared() {
        assert_eq!(ev("[[1, 2], [3]][0][1]"), json!(2).into());
        assert_eq!(
            ev("[params.first_name, payment.nope, 1 + 1]"),
            // json!(["John", null, 2]).into()
            Value::Array(Array(vec![
                Value::String("John".into()),
                Value::Void,
                Value::Number(2.into())
            ]))
        );
        assert_eq!(ev("[1, 2][5]"), Value::Void);
        assert_eq!(ev("[1, \"a\"] == [1, \"a\"]"), Value::Bool(true));
    }

    #[test]
    fn calls_and_pipes_resolve_the_same_function() {
        assert_eq!(ev("to_uppercase(params.first_name)"), json!("JOHN").into());
        assert_eq!(ev("params.first_name | to_uppercase"), json!("JOHN").into());
        assert_eq!(
            ev("params.first_name | to_uppercase | to_lowercase"),
            json!("john").into()
        );
    }

    #[test]
    fn unknown_function_errors() {
        let err = eval_str("params.first_name | nope").unwrap_err();
        assert!(err.to_string().contains("function nope not found"), "{err}",);
    }

    #[test]
    fn functions_are_values() {
        assert_matches!(ev("to_uppercase"), Value::Function(_));
        assert_eq!(ev("to_uppercase").value_type(), ValueType::Function);
        assert_eq!(ev("to_uppercase").to_string(), "fn to_uppercase/1");
        assert_eq!(ev("to_uppercase == to_uppercase"), Value::Bool(true));
        assert_eq!(ev("to_uppercase == to_lowercase"), Value::Bool(false));
    }

    #[test]
    fn functions_can_be_chosen_at_runtime() {
        assert_eq!(
            ev("(params.first_name == \"John\" ? to_uppercase : to_lowercase)(params.last_name)"),
            json!("DOE").into()
        );
        assert_eq!(
            ev("params.last_name | (false ? to_uppercase : to_lowercase)"),
            json!("doe").into()
        );
    }

    #[test]
    fn functions_can_be_stored_and_passed_around() {
        assert_eq!(
            ev("{\"upper\": to_uppercase}[\"upper\"](params.first_name)"),
            json!("JOHN").into()
        );
        assert_eq!(ev("[to_uppercase][0](\"x\")"), json!("X").into());
    }

    #[test]
    fn calling_a_non_function_errors() {
        let err = eval_str("payment.token()").unwrap_err();
        assert!(
            err.to_string().contains("only functions can be called"),
            "{}",
            err.to_string()
        );
    }

    #[test]
    fn unfinished_expr_errors() {
        assert_matches!(eval_str("payment[\"test\"]]"), Err(_));
    }

    #[test]
    fn top_level_resolution() {
        assert_eq!(ev("(nope || payment).token"), Value::String("tok_1".into()));
    }

    #[test]
    fn neq() {
        assert_eq!(ev("5 != 2"), Value::Bool(true));
        assert_eq!(ev("5 != 5"), Value::Bool(false));
        assert_eq!(ev("'test' != 5"), Value::Bool(true));
        assert_eq!(ev("[1, 2, 3] != [1, 2, 5]"), Value::Bool(true));
    }

    #[test]
    fn negation() {
        assert_eq!(ev("!true"), Value::Bool(false));
        assert_eq!(ev("!!true"), Value::Bool(true));
        assert_eq!(ev("!!5"), Value::Bool(true));
        assert_eq!(ev("!!5.0"), Value::Bool(true));
        assert_eq!(ev("!(!(false))"), Value::Bool(false));
    }

    #[test]
    fn integer_errors_do_not_panic() {
        assert!(err("1 / 0").to_string().contains("division by zero"));
        assert!(
            err("9223372036854775807 + 1")
                .to_string()
                .contains("overflow")
        );
        assert!(
            err("0 - 9223372036854775807 - 2")
                .to_string()
                .contains("overflow")
        );
        assert!(
            err("9223372036854775807 * 2")
                .to_string()
                .contains("overflow")
        );
    }

    #[test]
    fn integer_casting() {
        assert_eq!(ev("5 + 5.9"), Value::Number(10.9.into()));
        assert_eq!(ev("5 + 1"), Value::Number(6.into()));
        assert_eq!(ev("0.0 + 1"), Value::Number(1.0.into()));
        assert_eq!(ev("0.0 * 1"), Value::Number(0.0.into()));
        assert_eq!(ev("0.0 - 1"), Value::Number((-1.0).into()));
        assert_eq!(ev("0 - 1"), Value::Number((-1).into()));
        assert_eq!(ev("5.9 + 0.1"), Value::Number(6.0.into()));
        assert_eq!(ev("5859 / 100."), Value::Number(58.59.into()));
    }

    #[test]
    fn non_nullish_assertion() {
        err("null!");
        err("void!");
        assert_eq!(ev("(5.9 + 0.1)!"), Value::Number(6.0.into()));
        assert_eq!(ev("[]!"), Value::Array(Array::default()));
        assert_eq!(ev("{}!"), Value::Object(Object::default()));
        assert_eq!(ev("!5!"), Value::Bool(false));
        assert_eq!(ev("!0!"), Value::Bool(true));
    }

    #[test]
    fn closure_call() {
        assert_eq!(ev("(|v| v * 2)(3)"), Value::Number(6.into()));
    }

    #[test]
    #[ignore = "capturing closures are not implemented yet"]
    fn nested_closure_capture() {
        assert_eq!(ev("(|v| |v2| v2 * v)(3)(2)"), Value::Number(6.into()));
    }

    #[test]
    fn stack_overflow() {
        assert_matches!(
            err("(|f| f(f))(|f| f(f))"),
            crate::Error::Runtime(Spanned {
                inner: RuntimeError::DepthLimitReached,
                span: _
            })
        );
    }
}
