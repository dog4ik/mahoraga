use std::{collections::HashMap, f64::consts, rc::Rc};

use crate::{
    Error,
    eval::fns::{Args, Function},
    lex::{Atom, Ident},
    parser::Node,
    value::{Array, Object, Value},
};

pub mod fns;
pub mod std_fns;

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
            crate::lex::Punct::CmpNotEqual => eval(*lhs, env)?.neq(eval(*rhs, env)?),
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
                // `x | f` and `x | f(a, b)` both mean "call f with x first".
                let (callee, rest) = match *rhs {
                    Node::Call { callee, args } => (*callee, args),
                    node => (node, Vec::new()),
                };
                let name = callee_name(&callee).map(String::from);
                let callee = eval(callee, env)?;
                let mut values = Vec::with_capacity(rest.len() + 1);
                values.push(lhs);
                for arg in rest {
                    values.push(eval(arg, env)?);
                }
                call_value(callee, Args(values), name.as_deref())
            }
            crate::lex::Punct::Coalesce => {
                let lhs = eval(*lhs, env)?;
                if lhs.nullish() {
                    eval(*rhs, env)
                } else {
                    Ok(lhs)
                }
            }
            _ => Err(Error::new(format!("can't eval {punct}"))),
        },
        Node::Atom(atom) => match atom {
            // Attached scope shadows the function registry, so a payload field never gets
            // swallowed by a function of the same name.
            crate::lex::Atom::Ident(Ident(i)) => match env.runtime_objects.0.get(&i) {
                Some(v) => Ok(v.clone()),
                None => match env.fns.get(i.as_str()) {
                    Some(f) => Ok(Value::Function(f.clone())),
                    None => Ok(Value::Void),
                },
            },
            crate::lex::Atom::StrLit(s) => Ok(Value::String(s)),
            crate::lex::Atom::NumLit(n) => Ok(Value::Number(n)),
            crate::lex::Atom::BoolLit(b) => Ok(Value::Bool(b)),
            crate::lex::Atom::NullLit => Ok(Value::Null),
            crate::lex::Atom::VoidLit => Ok(Value::Void),
        },
        Node::ArrayLit(arr) => Ok(Value::Array(Array(
            arr.into_iter()
                .map(|v| eval(v, env))
                .collect::<Result<Vec<_>, _>>()?,
        ))),
        Node::ObjectLit(map) => Ok(Value::Object(Object(
            map.into_iter()
                .map(|(k, v)| Ok((k, eval(v, env)?)))
                .collect::<crate::Result<_>>()?,
        ))),
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
                    match index {
                        Value::String(s) => Ok(object.get(&s).cloned().unwrap_or(Value::Void)),
                        Value::Null | Value::Void => Ok(Value::Void),
                        _ => Err(Error::new("object can be only indexed by string")),
                    }
                }
                Value::Array(Array(array)) => {
                    let index = eval(*index, env)?;
                    match index {
                        Value::Number(n) if n >= 0. && n.fract() == 0. => {
                            Ok(array.get(n as usize).cloned().unwrap_or(Value::Void))
                        }
                        Value::Null | Value::Void => Ok(Value::Void),
                        _ => Err(Error::new("array can be only indexed by unsigned integer")),
                    }
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
                Value::Object(Object(obj)) => Ok(obj.get(&field).cloned().unwrap_or(Value::Void)),
                Value::Null | Value::Void => Ok(Value::Void),
                _ => Err(Error::new(format!(
                    "only object can have member access, got {object:?}"
                ))),
            }
        }
        Node::Call { callee, args } => {
            let name = callee_name(&callee).map(String::from);
            let callee = eval(*callee, env)?;
            let args = Args(
                args.into_iter()
                    .map(|v| eval(v, env))
                    .collect::<crate::Result<Vec<_>>>()?,
            );
            call_value(callee, args, name.as_deref())
        }
    }
}

/// The name a callee was written as, kept only so a failed call can say which one it was.
fn callee_name(node: &Node) -> Option<&str> {
    match node {
        Node::Atom(Atom::Ident(Ident(name))) => Some(name),
        Node::Member {
            field: Ident(f), ..
        } => Some(f),
        _ => None,
    }
}

fn call_value(callee: Value, args: Args, name: Option<&str>) -> crate::Result<Value> {
    match callee {
        Value::Function(f) => f.call(args),
        // An unresolved ident evaluates to void like any other missing lookup, so name it here
        // rather than reporting a bare type mismatch.
        Value::Void => Err(Error::new(match name {
            Some(name) => format!("function '{name}' is not found"),
            None => "only functions can be called, got void".into(),
        })),
        _ => Err(Error::new(format!(
            "only functions can be called, got {}",
            callee.value_type()
        ))),
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
                Value::Number(2.)
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
        assert!(
            err.message.contains("'nope' is not found"),
            "{}",
            err.message
        );
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
            err.message.contains("only functions can be called"),
            "{}",
            err.message
        );
    }

    #[test]
    fn arity_is_checked() {
        let err = eval_str("to_uppercase()").unwrap_err();
        assert!(err.message.contains("takes 1 argument"), "{}", err.message);
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
}
