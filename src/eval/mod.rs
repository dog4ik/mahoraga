use std::{collections::HashMap, f64::consts, rc::Rc};

use crate::{
    Punct,
    eval::fns::{Args, Function},
    lex::{Atom, Ident},
    parser::{Node, NodeKind},
    span::Spanned,
    value::{Array, Number, Object, Value, ValueType},
};

pub mod fns;
pub mod std_fns;

type Result<T> = std::result::Result<T, Spanned<RuntimeError>>;

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
        }
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
    #[error("{0}")]
    IntConversionError(std::num::TryFromIntError),
    #[error("{0}")]
    FloatConversionError(std::num::TryFromIntError),
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

#[derive(Debug, Default)]
pub struct Env {
    pub fns: HashMap<&'static str, Rc<Function>>,
    pub runtime_objects: Object,
}

impl Env {
    pub fn std() -> Self {
        Self {
            fns: std_fns::std_fns(),
            runtime_objects: Object(HashMap::from_iter([(
                String::from("constants"),
                Value::Object(Object(HashMap::from_iter([
                    (String::from("pi"), Value::Number(Number::Float(consts::PI))),
                    (
                        String::from("tau"),
                        Value::Number(Number::Float(consts::TAU)),
                    ),
                ]))),
            )])),
        }
    }

    pub fn attach_object(&mut self, obj: impl Into<Object>) {
        self.runtime_objects.0.extend(obj.into().0);
    }

    pub fn merge(&mut self, other: Env) {
        self.fns.extend(other.fns);
        self.runtime_objects.0.extend(other.runtime_objects.0);
    }
}

pub fn eval(Node { span, kind }: &Node, env: &Env) -> Result<Value> {
    match kind {
        NodeKind::Op((punct, (lhs, rhs))) => {
            let spanitize = |e: RuntimeError| Spanned::new(e, *span);
            match punct {
                crate::lex::Punct::Add => eval(lhs, env)?.add(&eval(rhs, env)?).map_err(spanitize),
                crate::lex::Punct::Mul => eval(lhs, env)?.mul(&eval(rhs, env)?).map_err(spanitize),
                crate::lex::Punct::Sub => eval(lhs, env)?.sub(&eval(rhs, env)?).map_err(spanitize),
                crate::lex::Punct::Div => eval(lhs, env)?.div(&eval(rhs, env)?).map_err(spanitize),
                crate::lex::Punct::Less => eval(lhs, env)?.lt(&eval(rhs, env)?).map_err(spanitize),
                crate::lex::Punct::LessOrEq => {
                    eval(lhs, env)?.lte(&eval(rhs, env)?).map_err(spanitize)
                }
                crate::lex::Punct::More => eval(lhs, env)?.mt(&eval(rhs, env)?).map_err(spanitize),
                crate::lex::Punct::MoreOrEq => {
                    eval(lhs, env)?.mte(&eval(rhs, env)?).map_err(spanitize)
                }
                crate::lex::Punct::CmpEqual => {
                    eval(lhs, env)?.eq(&eval(rhs, env)?).map_err(spanitize)
                }
                crate::lex::Punct::CmpNotEqual => {
                    eval(lhs, env)?.neq(&eval(rhs, env)?).map_err(spanitize)
                }
                crate::lex::Punct::Or => {
                    let lhs = eval(lhs, env)?;
                    if lhs.truthy() {
                        Ok(lhs)
                    } else {
                        eval(rhs, env)
                    }
                }
                crate::lex::Punct::And => {
                    let lhs = eval(lhs, env)?;
                    if lhs.truthy() {
                        eval(rhs, env)
                    } else {
                        Ok(lhs)
                    }
                }
                crate::lex::Punct::Pipe => {
                    let lhs = eval(lhs, env)?;
                    // `x | f` and `x | f(a, b)` both mean "call f with x first".
                    let (callee, rest) = match &rhs.kind {
                        NodeKind::Call { callee, args } => (callee, args),
                        _ => (rhs, &Vec::new()),
                    };
                    let name = callee_name(&callee.kind).map(String::from);
                    let callee = eval(callee, env)?;
                    let mut values = Vec::with_capacity(rest.len() + 1);
                    values.push(lhs);
                    for arg in rest {
                        values.push(eval(arg, env)?);
                    }
                    call_value(callee, Args(values), name.as_deref())
                        .map_err(|e| Spanned::new(e.into(), rhs.span))
                }
                crate::lex::Punct::Coalesce => {
                    let lhs = eval(lhs, env)?;
                    if lhs.nullish() {
                        eval(rhs, env)
                    } else {
                        Ok(lhs)
                    }
                }
                _ => unreachable!("The is no implementation to evaluate '{punct}'"),
            }
        }
        NodeKind::Atom(atom) => match atom {
            // Attached scope shadows the function registry, so a payload field never gets
            // swallowed by a function of the same name.
            crate::lex::Atom::Ident(Ident(i)) => match env.runtime_objects.0.get(i) {
                Some(v) => Ok(v.clone()),
                None => match env.fns.get(i.as_str()) {
                    Some(f) => Ok(Value::Function(f.clone())),
                    None => Ok(Value::Void),
                },
            },
            crate::lex::Atom::StrLit(s) => Ok(Value::String(s.to_owned())),
            crate::lex::Atom::FloatLit(n) => Ok(Value::Number(Number::Float(*n))),
            crate::lex::Atom::IntLit(n) => Ok(Value::Number(Number::Int(*n))),
            crate::lex::Atom::BoolLit(b) => Ok(Value::Bool(*b)),
            crate::lex::Atom::NullLit => Ok(Value::Null),
            crate::lex::Atom::VoidLit => Ok(Value::Void),
        },
        NodeKind::ArrayLit(arr) => Ok(Value::Array(Array(
            arr.iter()
                .map(|v| eval(v, env))
                .collect::<Result<Vec<_>>>()?,
        ))),
        NodeKind::ObjectLit(map) => Ok(Value::Object(Object(
            map.iter()
                .map(|(k, v)| Ok((k.to_owned(), eval(v, env)?)))
                .collect::<Result<_>>()?,
        ))),
        NodeKind::Turnary {
            operand,
            truth_node,
            false_node,
        } => {
            let operand = eval(operand, env)?;
            if operand.truthy() {
                eval(truth_node, env)
            } else {
                eval(false_node, env)
            }
        }
        NodeKind::Index { object, index } => {
            let object = eval(object, env)?;
            match object {
                Value::Object(Object(object)) => {
                    let index_value = eval(index, env)?;
                    match index_value {
                        Value::String(s) => Ok(object.get(&s).cloned().unwrap_or(Value::Void)),
                        Value::Null | Value::Void => Ok(Value::Void),
                        _ => Err(Spanned::new(
                            RuntimeError::InvalidObjectKey(index_value.value_type()),
                            index.span,
                        )),
                    }
                }
                Value::Array(Array(array)) => {
                    let index_value = eval(index, env)?;
                    match index_value {
                        Value::Number(Number::Float(n)) if n >= 0. && n.fract() == 0. => {
                            Ok(array.get(n as usize).cloned().unwrap_or(Value::Void))
                        }
                        Value::Number(Number::Int(n)) if n >= 0 => {
                            Ok(array.get(n as usize).cloned().unwrap_or(Value::Void))
                        }
                        Value::Null | Value::Void => Ok(Value::Void),
                        _ => Err(Spanned::new(
                            RuntimeError::InvalidArrayIndex(index_value.value_type()),
                            index.span,
                        )),
                    }
                }
                _ => Err(Spanned::new(RuntimeError::InvalidIndexOperand, index.span)),
            }
        }
        NodeKind::Member {
            object,
            field: Ident(field),
            ..
        } => {
            let object_value = eval(object, env)?;
            match object_value {
                Value::Object(Object(obj)) => Ok(obj.get(field).cloned().unwrap_or(Value::Void)),
                Value::Null | Value::Void => Ok(Value::Void),
                _ => Err(Spanned::new(RuntimeError::InvalidIndexOperand, object.span)),
            }
        }
        NodeKind::Call { callee, args } => {
            let name = callee_name(&callee.kind).map(String::from);
            let callee = eval(callee, env)?;
            let args = Args(
                args.iter()
                    .map(|v| eval(v, env))
                    .collect::<Result<Vec<_>>>()?,
            );
            call_value(callee, args, name.as_deref()).map_err(|e| Spanned::new(e.into(), *span))
        }
        NodeKind::Negation(statement) => {
            let value = eval(statement, env)?;
            Ok(Value::Bool(!value.truthy()))
        }
    }
}

/// The name a callee was written as, kept only so a failed call can say which one it was.
fn callee_name(node: &NodeKind) -> Option<&str> {
    match node {
        NodeKind::Atom(Atom::Ident(Ident(name))) => Some(name),
        NodeKind::Member {
            field: Ident(f), ..
        } => Some(f),
        _ => None,
    }
}

fn call_value(
    callee: Value,
    args: Args,
    name: Option<&str>,
) -> std::result::Result<Value, FunctionCallError> {
    match callee {
        Value::Function(f) => f.call(args),
        // An unresolved ident evaluates to void like any other missing lookup, so name it here
        // rather than reporting a bare type mismatch.
        Value::Void => Err(match name {
            Some(name) => FunctionCallError::NotFound {
                name: name.to_owned(),
            },
            None => FunctionCallError::UncallableType(ValueType::Void),
        }),
        _ => Err(FunctionCallError::UncallableType(callee.value_type())),
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use serde_json::json;

    use super::*;
    use crate::{parser::parse_expr, span::Span, value::ValueType};

    fn eval(node: Node) -> crate::Result<Value> {
        let env = Env::std();
        Ok(super::eval(&node, &env)?)
    }

    fn eval_str(s: &str) -> crate::Result<Value> {
        let node = parse_expr(s)?;
        let mut env = Env::std();
        match scope() {
            Value::Object(object) => env.attach_object(object),
            _ => panic!("scope return object value"),
        }
        Ok(super::eval(&node, &env)?)
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
    #[ignore = "required methods are not implemented yet"]
    fn absence_flows_through_a_pipeline() {
        assert_eq!(ev("payment.product | trim | upper"), Value::Null);
        assert_eq!(
            ev("payment.product | trim ?? 'fallback'"),
            json!("fallback").into()
        );
    }

    #[test]
    #[ignore = "function calls are not supported yet"]
    fn arity_is_checked_before_evaluation() {
        let err = eval_str("payment.token | null_if").unwrap_err();
        assert!(
            err.to_string().contains("takes 1 argument"),
            "{}",
            err.to_string()
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
        let err = |src| eval(parse_expr(src).unwrap()).unwrap_err();
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
}
