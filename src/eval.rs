use std::{collections::HashMap, f64::consts};

use crate::{
    Error,
    lex::{Atom, Ident},
    parser::Node,
    value::{Array, Object, Value},
};

#[derive(Debug)]
pub struct PureFunction {
    f: fn(val: Value) -> crate::Result<Value>,
}

impl PureFunction {
    pub fn new(f: fn(val: Value) -> crate::Result<Value>) -> Self {
        Self { f }
    }
}

#[derive(Debug, Default)]
pub struct Env {
    pub fns: HashMap<&'static str, PureFunction>,
    pub runtime_objects: Object,
}

mod std_fns {
    use crate::{Error, value::Value};

    pub fn to_uppercase(val: Value) -> crate::Result<Value> {
        match val {
            Value::String(s) => Ok(Value::String(s.to_uppercase())),
            _ => Err(Error::new("unexpected data type")),
        }
    }

    pub fn to_lowercase(val: Value) -> crate::Result<Value> {
        match val {
            Value::String(s) => Ok(Value::String(s.to_lowercase())),
            _ => Err(Error::new("unexpected data type")),
        }
    }
}

impl Env {
    pub fn std() -> Self {
        Self {
            fns: HashMap::from_iter([
                (
                    "upper",
                    PureFunction {
                        f: std_fns::to_uppercase,
                    },
                ),
                (
                    "lower",
                    PureFunction {
                        f: std_fns::to_lowercase,
                    },
                ),
            ]),
            runtime_objects: Object(HashMap::from_iter([(
                String::from("constants"),
                Value::Object(Object(HashMap::from_iter([
                    (String::from("pi"), Value::Number(consts::PI)),
                    (String::from("tau"), Value::Number(consts::TAU)),
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

pub fn eval(node: Node, env: &Env) -> crate::Result<Value> {
    match node {
        Node::Op((punct, (lhs, rhs))) => match punct {
            crate::lex::Punct::Add => eval(*lhs, env)?.add(eval(*rhs, env)?),
            crate::lex::Punct::Mul => eval(*lhs, env)?.mul(eval(*rhs, env)?),
            crate::lex::Punct::Sub => eval(*lhs, env)?.sub(eval(*rhs, env)?),
            crate::lex::Punct::Div => eval(*lhs, env)?.div(eval(*rhs, env)?),
            crate::lex::Punct::Less => eval(*lhs, env)?.lt(eval(*rhs, env)?),
            crate::lex::Punct::LessOrEq => eval(*lhs, env)?.lte(eval(*rhs, env)?),
            crate::lex::Punct::More => eval(*lhs, env)?.mt(eval(*rhs, env)?),
            crate::lex::Punct::MoreOrEq => eval(*lhs, env)?.mte(eval(*rhs, env)?),
            crate::lex::Punct::CmpEqual => eval(*lhs, env)?.eq(eval(*rhs, env)?),
            crate::lex::Punct::Or => {
                let lhs = eval(*lhs, env)?;
                if lhs.truthy() {
                    Ok(lhs)
                } else {
                    eval(*rhs, env)
                }
            }
            crate::lex::Punct::And => {
                let lhs = eval(*lhs, env)?;
                if lhs.truthy() {
                    eval(*rhs, env)
                } else {
                    Ok(lhs)
                }
            }
            crate::lex::Punct::Pipe => {
                let lhs = eval(*lhs, env)?;
                let ident = match *rhs {
                    Node::Atom(Atom::Ident(Ident(ident))) => ident,
                    _ => return Err(Error::new("rhs of pipe should be ident")),
                };
                let f = env
                    .fns
                    .get(ident.as_str())
                    .ok_or_else(|| Error::new("could not find function"))?;
                (f.f)(lhs)
            }
            crate::lex::Punct::Coalesce => {
                let lhs = eval(*lhs, env)?;
                if lhs == Value::Null {
                    eval(*rhs, env)
                } else {
                    Ok(lhs)
                }
            }
            _ => Err(Error::new(format!("can't eval {punct}"))),
        },
        Node::Atom(atom) => match atom {
            crate::lex::Atom::Ident(Ident(i)) => match env.runtime_objects.0.get(&i) {
                Some(v) => Ok(v.clone()),
                None => Ok(Value::Null),
                // None => Err(Error::new("raw idents are not supported yet")),
            },
            crate::lex::Atom::StrLit(s) => Ok(Value::String(s)),
            crate::lex::Atom::NumLit(n) => Ok(Value::Number(n)),
            crate::lex::Atom::BoolLit(b) => Ok(Value::Bool(b)),
        },
        Node::Turnary {
            operand,
            truth_node,
            false_node,
        } => {
            let operand = eval(*operand, env)?;
            if operand.truthy() {
                eval(*truth_node, env)
            } else {
                eval(*false_node, env)
            }
        }
        Node::Index { object, index } => {
            let object = eval(*object, env)?;
            match object {
                Value::Object(Object(object)) => {
                    let index = eval(*index, env)?;
                    let index = match index {
                        Value::String(s) => s,
                        _ => return Err(Error::new("object can be only indexed by string")),
                    };
                    Ok(object.get(&index).cloned().unwrap_or(Value::Null))
                }
                Value::Array(Array(array)) => {
                    let index = eval(*index, env)?;
                    let index = match index {
                        Value::Number(n) if n >= 0. && n.fract() == 0. => n as usize,
                        _ => {
                            return Err(Error::new(
                                "array can be only indexed by unsigned integer",
                            ));
                        }
                    };
                    Ok(array.get(index).cloned().unwrap_or(Value::Null))
                }
                _ => Err(crate::Error::new(format!(
                    "only array or object can be indexed, got {object:?}"
                ))),
            }
        }
        Node::Member {
            object,
            field: Ident(field),
        } => {
            let object = eval(*object, env)?;
            match object {
                Value::Object(Object(obj)) => Ok(obj.get(&field).cloned().unwrap_or(Value::Null)),
                Value::Null => Ok(Value::Null),
                _ => Err(Error::new(format!(
                    "only object can have member access, got {object:?}"
                ))),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use serde_json::json;

    use super::*;
    use crate::{parser::parse_expr, span::Span};

    fn eval(node: Node) -> crate::Result<Value> {
        let env = Env::std();
        super::eval(node, &env)
    }

    fn eval_str(s: &str) -> crate::Result<Value> {
        let node = parse_expr(s)?;
        let mut env = Env::std();
        match scope() {
            Value::Object(object) => env.attach_object(object),
            _ => panic!("scope return object value"),
        }
        super::eval(node, &env)
    }

    #[test]
    fn basic_add() -> crate::Result<()> {
        assert_eq!(eval(parse_expr("2 + 2")?)?, Value::Number(4.0));
        Ok(())
    }

    #[test]
    fn add_and_mult() -> crate::Result<()> {
        assert_eq!(eval(parse_expr("(2 + 2) * 3")?)?, Value::Number(12.));
        Ok(())
    }

    #[test]
    fn turnary_true() -> crate::Result<()> {
        assert_eq!(eval(parse_expr("true ? 1 : 2")?)?, Value::Number(1.));
        Ok(())
    }

    #[test]
    fn turnary_false() -> crate::Result<()> {
        assert_eq!(eval(parse_expr("false ? 1 : 2")?)?, Value::Number(2.));
        Ok(())
    }

    #[test]
    fn nested_turnary_false() -> crate::Result<()> {
        assert_eq!(
            eval(parse_expr(
                "false ? 1 ? 10 + 10 : 0 : \"test\" ? 100 * 125 : 0"
            )?)?,
            Value::Number(100. * 125.)
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
    fn missing_paths_and_roots_are_null() {
        assert_eq!(ev("payment.nope"), Value::Null);
        assert_eq!(ev("payment.nope.deeper"), Value::Null);
        assert_eq!(ev("nosuchroot.x"), Value::Null);
        assert_eq!(ev("params.customer[\"3\"]"), Value::Null);
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
    #[ignore = "function calls are not supported yet"]
    fn reproduces_the_scripay_channel_rule() {
        // extra_return_param is the sentinel, so the merchant default wins.
        assert_eq!(
            ev("params.extra_return_param | null_if('_blank_') ?? settings.channel"),
            json!("Mpesa").into()
        );
    }

    #[test]
    #[ignore = "function calls are not supported yet"]
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
        assert!(err.message.contains("takes 1 argument"), "{}", err.message);
    }

    #[test]
    #[ignore = "function calls are not supported yet"]
    fn function_errors_carry_the_call_span() {
        let err = eval_str("params.customer | trim").unwrap_err();
        assert!(err.message.starts_with("trim:"), "{}", err.message);
        assert_eq!(err.span, Some(Span::new(18, 22)));
    }

    #[test]
    fn object_and_array_literals_evaluate() {
        assert_eq!(
            ev("{a: payment.token, b: 2}"),
            json!({"a": "tok_1", "b": 2}).into()
        );
        assert_eq!(ev("[1, payment.token]"), json!([1, "tok_1"]).into());
    }

    #[test]
    fn unfinished_expr_errors() {
        assert_matches!(eval_str("payment[\"test\"]]"), Err(_));
    }
}
