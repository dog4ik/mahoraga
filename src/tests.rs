use std::assert_matches;

use serde_json::json;

use crate::{
    env::Env,
    error::{Error, RuntimeError},
    parser::parse_expr,
    span::{Span, Spanned},
    traverse::TraverseRuntime,
    value::{Array, Object, Value, ValueType},
    vm,
};

fn eval_str(s: &str) -> crate::Result<Value> {
    let node = parse_expr(s)?;
    let mut env = Env::std();
    match scope() {
        Value::Object(object) => env.attach_object(object),
        _ => unreachable!("scope return object value"),
    }
    let mut runtime = TraverseRuntime::new(&env, node.span);
    let traverse_result = runtime.eval(&node);
    let mut vm = vm::VirtualMachine::new(vm::compile(&node), &env);
    let vm_result: crate::Result<Value> = vm.eval().map_err(Into::into);
    if vm_result.is_ok() {
        assert!(
            vm.stack_values().is_empty(),
            "vm stack must be empty after eval finished, got {:?}",
            vm.stack_values(),
        );
    }
    match (&vm_result, &traverse_result) {
        (Ok(vm), Ok(traverse)) => {
            assert_eq!(vm, traverse, "vm and traverse values must be the same");
        }
        (Ok(vm), Err(traverse)) => {
            panic!("vm success ({vm}) | traverse error: {traverse}");
        }
        (Err(vm), Ok(traverse)) => {
            panic!("vm error ({vm}) | traverse success: {traverse}");
        }
        (Err(_), Err(_)) => {
            // todo: eval and traverse should have similar errors
            // currently vm can't emit function {name} not found errors

            // let Error::Runtime(e) = vm else {
            //     panic!("expected runtime error")
            // };
            // assert_eq!(
            //     e.to_string(),
            //     traverse.to_string(),
            //     "execution errors should match"
            // );
        }
    }
    Ok(traverse_result?)
}

fn scope() -> Value {
    json!({
      "payment": {
        "token": "tok_1",
        "gateway_amount": 1000,
        "product": null,
        "order_number": "ORD-9",
        "items": [{ "price": 10, "qty": 2 }, { "price": 5, "qty": 1 }]
      },
      "params": {
        "phone": null,
        "customer": {
          "phone": "0798288410"
        },
        "first_name": "John",
        "last_name": "Doe",
        "extra_return_param": "_blank_"
      },
      "settings": {
        "wallet": "w1",
        "code": "MPESA",
        "channel": "Mpesa"
      },
      "steps": {
        "auth": {
          "access_token": "abc"
        }
      },
      "env": {
        "callback_url": "https://cb.example/gateway/callback"
      }
    })
    .into()
}

#[test]
fn basic_add() -> crate::Result<()> {
    assert_eq!(eval_str("2 + 2")?, Value::Number(4.0.into()));
    Ok(())
}

#[test]
fn add_and_mult() -> crate::Result<()> {
    assert_eq!(eval_str("(2 + 2) * 3")?, Value::Number(12.into()));
    Ok(())
}

#[test]
fn turnary_true() -> crate::Result<()> {
    assert_eq!(eval_str("true ? 1 : 2")?, Value::Number(1.into()));
    Ok(())
}

#[test]
fn turnary_false() -> crate::Result<()> {
    assert_eq!(eval_str("false ? 1 : 2")?, Value::Number(2.into()));
    Ok(())
}

#[test]
fn nested_turnary_false() -> crate::Result<()> {
    assert_eq!(
        eval_str("false ? 1 ? 10 + 10 : 0 : \"test\" ? 100 * 125 : 0")?,
        Value::Number((100 * 125).into())
    );
    Ok(())
}

#[test]
fn mte() -> crate::Result<()> {
    assert_eq!(eval_str("11.3 >= 12")?, Value::Bool(false));
    assert_eq!(eval_str("10.0 >= 10")?, Value::Bool(true));
    assert_eq!(eval_str("9.0 >= 10")?, Value::Bool(false));
    Ok(())
}

#[test]
fn mt() -> crate::Result<()> {
    assert_eq!(eval_str("11.3 > 12")?, Value::Bool(false));
    assert_eq!(eval_str("10.0 > 10")?, Value::Bool(false));
    assert_eq!(eval_str("9.0 > 10")?, Value::Bool(false));
    Ok(())
}

#[test]
fn eq_with_ternary() -> crate::Result<()> {
    assert_eq!(eval_str("10 > 5 ? true : false")?, Value::Bool(true));
    Ok(())
}

#[test]
fn eq_with_ternary2() -> crate::Result<()> {
    assert_eq!(eval_str("10 >= 10 ? true : false")?, Value::Bool(true));
    Ok(())
}

#[test]
fn reads_nested_paths() {
    assert_eq!(eval_str("payment.token").unwrap(), json!("tok_1").into());
    assert_eq!(
        eval_str("steps.auth.access_token").unwrap(),
        json!("abc").into()
    );
    assert_eq!(
        eval_str("env.callback_url").unwrap(),
        json!("https://cb.example/gateway/callback").into()
    );
}

#[test]
fn missing_paths_and_roots_are_void() {
    assert_eq!(eval_str("payment.nope").unwrap(), Value::Void);
    assert_eq!(eval_str("payment.nope.deeper").unwrap(), Value::Void);
    assert_eq!(eval_str("nosuchroot.x").unwrap(), Value::Void);
    assert_eq!(eval_str("params.customer[\"3\"]").unwrap(), Value::Void);
}

#[test]
fn null_is_preserved() {
    assert_eq!(eval_str("params.phone").unwrap(), Value::Null);
}

#[test]
fn or_skips_null_and_empty() {
    assert_eq!(
        eval_str("payment.product || payment.order_number || \"Payment\"").unwrap(),
        json!("ORD-9").into()
    );
    assert_eq!(
        eval_str("payment.product || payment.nope || \"Payment\"").unwrap(),
        json!("Payment").into()
    );
    assert_eq!(
        eval_str("params.phone || params.customer.phone").unwrap(),
        json!("0798288410").into()
    );
}

#[test]
fn reproduces_the_scripay_account_name_rule() {
    assert_eq!(
        eval_str("[params.first_name, params.last_name] | join(' ') | trim").unwrap(),
        json!("John Doe").into()
    );
}

#[test]
#[ignore = "behavior is not defined yet"]
fn absence_flows_through_a_pipeline() {
    assert_eq!(
        eval_str("payment.product | trim | to_uppercase").unwrap(),
        Value::Null
    );
    assert_eq!(
        eval_str("payment.product | trim ?? 'fallback'").unwrap(),
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
        eval_str("{\"a\": payment.token, \"b\": 2}").unwrap(),
        json!({"a": "tok_1", "b": 2}).into()
    );
    assert_eq!(
        eval_str("[1, payment.token]").unwrap(),
        json!([1, "tok_1"]).into()
    );
}

#[test]
fn object_null_index_should_produce_void() {
    assert_eq!(
        eval_str("{\"a\": payment.token, \"b\": 2}[null]").unwrap(),
        Value::Void,
    );
}

#[test]
fn array_literals_can_be_indexed_and_compared() {
    assert_eq!(eval_str("[[1, 2], [3]][0][1]").unwrap(), json!(2).into());
    assert_eq!(
        eval_str("[params.first_name, payment.nope, 1 + 1]").unwrap(),
        // json!(["John", null, 2]).into()
        Value::Array(Array(vec![
            Value::String("John".into()),
            Value::Void,
            Value::Number(2.into())
        ]))
    );
    assert_eq!(eval_str("[1, 2][5]").unwrap(), Value::Void);
    assert_eq!(
        eval_str("[1, \"a\"] == [1, \"a\"]").unwrap(),
        Value::Bool(true)
    );
}

#[test]
fn calls_and_pipes_resolve_the_same_function() {
    assert_eq!(
        eval_str("to_uppercase(params.first_name)").unwrap(),
        json!("JOHN").into()
    );
    assert_eq!(
        eval_str("params.first_name | to_uppercase").unwrap(),
        json!("JOHN").into()
    );
    assert_eq!(
        eval_str("params.first_name | to_uppercase | to_lowercase").unwrap(),
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
    assert_matches!(eval_str("to_uppercase").unwrap(), Value::Function(_));
    assert_eq!(
        eval_str("to_uppercase").unwrap().value_type(),
        ValueType::Function
    );
    assert_eq!(
        eval_str("to_uppercase").unwrap().to_string(),
        "fn to_uppercase/1"
    );
    assert_eq!(
        eval_str("to_uppercase == to_uppercase").unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        eval_str("to_uppercase == to_lowercase").unwrap(),
        Value::Bool(false)
    );
}

#[test]
fn functions_can_be_chosen_at_runtime() {
    assert_eq!(
        eval_str("(params.first_name == \"John\" ? to_uppercase : to_lowercase)(params.last_name)")
            .unwrap(),
        json!("DOE").into()
    );
    assert_eq!(
        eval_str("params.last_name | (false ? to_uppercase : to_lowercase)").unwrap(),
        json!("doe").into()
    );
}

#[test]
fn functions_can_be_stored_and_passed_around() {
    assert_eq!(
        eval_str("{\"upper\": to_uppercase}[\"upper\"](params.first_name)").unwrap(),
        json!("JOHN").into()
    );
    assert_eq!(
        eval_str("[to_uppercase][0](\"x\")").unwrap(),
        json!("X").into()
    );
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
    assert_eq!(
        eval_str("(nope || payment).token").unwrap(),
        Value::String("tok_1".into())
    );
}

#[test]
fn neq() {
    assert_eq!(eval_str("5 != 2").unwrap(), Value::Bool(true));
    assert_eq!(eval_str("5 != 5").unwrap(), Value::Bool(false));
    assert_eq!(eval_str("'test' != 5").unwrap(), Value::Bool(true));
    assert_eq!(
        eval_str("[1, 2, 3] != [1, 2, 5]").unwrap(),
        Value::Bool(true)
    );
}

#[test]
fn negation() {
    assert_eq!(eval_str("!true").unwrap(), Value::Bool(false));
    assert_eq!(eval_str("!!true").unwrap(), Value::Bool(true));
    assert_eq!(eval_str("!!5").unwrap(), Value::Bool(true));
    assert_eq!(eval_str("!!5.0").unwrap(), Value::Bool(true));
    assert_eq!(eval_str("!(!(false))").unwrap(), Value::Bool(false));
}

#[test]
fn integer_errors_do_not_panic() {
    assert!(
        eval_str("1 / 0")
            .unwrap_err()
            .to_string()
            .contains("division by zero")
    );
    assert!(
        eval_str("9223372036854775807 + 1")
            .unwrap_err()
            .to_string()
            .contains("overflow")
    );
    assert!(
        eval_str("0 - 9223372036854775807 - 2")
            .unwrap_err()
            .to_string()
            .contains("overflow")
    );
    assert!(
        eval_str("9223372036854775807 * 2")
            .unwrap_err()
            .to_string()
            .contains("overflow")
    );
}

#[test]
fn integer_casting() {
    assert_eq!(eval_str("5 + 5.9").unwrap(), Value::Number(10.9.into()));
    assert_eq!(eval_str("5 + 1").unwrap(), Value::Number(6.into()));
    assert_eq!(eval_str("0.0 + 1").unwrap(), Value::Number(1.0.into()));
    assert_eq!(eval_str("0.0 * 1").unwrap(), Value::Number(0.0.into()));
    assert_eq!(eval_str("0.0 - 1").unwrap(), Value::Number((-1.0).into()));
    assert_eq!(eval_str("0 - 1").unwrap(), Value::Number((-1).into()));
    assert_eq!(eval_str("5.9 + 0.1").unwrap(), Value::Number(6.0.into()));
    assert_eq!(
        eval_str("5859 / 100.").unwrap(),
        Value::Number(58.59.into())
    );
}

#[test]
fn non_nullish_assertion() {
    eval_str("null!").unwrap_err();
    eval_str("void!").unwrap_err();
    assert_eq!(eval_str("(5.9 + 0.1)!").unwrap(), Value::Number(6.0.into()));
    assert_eq!(eval_str("[]!").unwrap(), Value::Array(Array::default()));
    assert_eq!(eval_str("{}!").unwrap(), Value::Object(Object::default()));
    assert_eq!(eval_str("!5!").unwrap(), Value::Bool(false));
    assert_eq!(eval_str("!0!").unwrap(), Value::Bool(true));
}

#[test]
fn closure_call() {
    assert_eq!(eval_str("(|v| v * 2)(3)").unwrap(), Value::Number(6.into()));
}

#[test]
fn place_access_matches_regular_evaluation() {
    assert_eq!(
        eval_str("steps.auth[\"access_token\"]").unwrap(),
        json!("abc").into()
    );
    assert_eq!(
        eval_str("(|p| p.customer.phone)(params)").unwrap(),
        json!("0798288410").into()
    );
    assert_eq!(
        eval_str("(|p| p[\"customer\"][\"phone\"])(params)").unwrap(),
        json!("0798288410").into()
    );
    assert_eq!(eval_str("constants[null]").unwrap(), Value::Void);
    assert_eq!(eval_str("payment.product.nope").unwrap(), Value::Void);
    assert_eq!(
        eval_str("{\"a\": {\"b\": 1}}.a.b").unwrap(),
        json!(1).into()
    );
    assert_eq!(eval_str("[[1, 2], [3]][0][1.0]").unwrap(), json!(2).into());
}

#[test]
fn place_access_errors() {
    assert_matches!(
        eval_str("constants.pi.x").unwrap_err(),
        Error::Runtime(Spanned {
            inner: RuntimeError::InvalidIndexOperand(_),
            ..
        })
    );
    assert_matches!(
        eval_str("constants.pi[0]").unwrap_err(),
        Error::Runtime(Spanned {
            inner: RuntimeError::InvalidIndexOperand(_),
            ..
        })
    );
    assert_matches!(eval_str("constants.nope[0]"), Ok(Value::Void));
    assert_matches!(
        eval_str("constants[1]").unwrap_err(),
        Error::Runtime(Spanned {
            inner: RuntimeError::InvalidObjectKey(ValueType::Integer),
            ..
        })
    );
}

#[test]
fn string_helpers() {
    assert_eq!(eval_str("'a' + 'b' + 'c'").unwrap(), json!("abc").into());
    assert_eq!(
        eval_str("[1, 'a', true, null, 1.5] | join('-')").unwrap(),
        json!("1-a-true--1.5").into()
    );
    assert_eq!(eval_str("[1, 'a'] | concat").unwrap(), json!("1a").into());
    assert_eq!(eval_str("'x' | to_s").unwrap(), json!("x").into());
    assert_eq!(eval_str("2.5 | to_s").unwrap(), json!("2.5").into());
    assert_eq!(
        eval_str("[1, 'a', true] | join(', ')").unwrap(),
        json!("1, a, true").into()
    );
}

#[test]
fn stack_overflow() {
    assert_matches!(
        eval_str("(|f| f(f))(|f| f(f))").unwrap_err(),
        Error::Runtime(Spanned {
            inner: RuntimeError::DepthLimitReached,
            span: _
        })
    );
}

#[test]
fn eval_ordering() {
    assert_eq!(eval_str("!true").unwrap(), false.into());
    assert_eq!(eval_str("5 - 3").unwrap(), 2.into());
    assert_eq!(eval_str("\"a\" + \"b\"").unwrap(), "ab".into());
    assert_eq!(eval_str("true ? (false ? 1 : 2) : 3").unwrap(), 2.into());

    assert_eq!(eval_str("null ?? 1").unwrap(), 1.into());
    assert_eq!(eval_str("5 ?? 1").unwrap(), 5.into());

    assert_eq!(eval_str("false || 1").unwrap(), 1.into());
    assert_eq!(eval_str("true || 1").unwrap(), true.into());

    assert_eq!(eval_str("1 && 0").unwrap(), 0.into());
    assert_eq!(eval_str("false && 1").unwrap(), false.into());
}

#[test]
fn object_access() {
    assert_eq!(
        eval_str("constants.pi").unwrap(),
        std::f64::consts::PI.into()
    );
}

#[test]
fn array_lit() {
    assert_eq!(
        eval_str("[10, \"foo\", [[\"test\"]]]").unwrap(),
        json!([10, "foo", [["test"]]]).into()
    );
}

#[test]
fn object_lit() {
    assert_eq!(
        eval_str("{'foo': 'bar', 'baz': 39}").unwrap(),
        json!({"foo": "bar", "baz": 39}).into()
    );
}

#[test]
fn external_fn_call() {
    assert_eq!(eval_str("to_uppercase('bar')").unwrap(), "BAR".into());
    assert_eq!(
        eval_str("'hahaha' | replace_all('ha', 'hi')").unwrap(),
        "hihihi".into()
    );
    assert_eq!(
        eval_str("replace_all('hahaha', 'ha', 'hi')").unwrap(),
        "hihihi".into()
    );
    assert_eq!(eval_str("' a ' | trim | to_uppercase").unwrap(), "A".into());
}

#[test]
fn closures() {
    assert_eq!(eval_str("(|v| v * 2)(4)").unwrap(), 8.into());
    assert_eq!(eval_str("(|a, b| a - b)(5, 3)").unwrap(), 2.into());
    assert_eq!(eval_str("1 + (|v| v)(2)").unwrap(), 3.into());
}

#[test]
fn external_fns() {
    assert_eq!(
        eval_str("[1, 2, 3] | map(|v| v * 2)").unwrap(),
        vec![2, 4, 6].into()
    );
    assert_eq!(
        eval_str("1 + ([1, 2] | map(|v| v) | len)").unwrap(),
        3.into()
    );
}

#[test]
fn access_paths() {
    assert_eq!(eval_str("payment.items[1].price").unwrap(), 5.into());
    assert_eq!(
        eval_str("payment[\"items\"][0.0][\"qty\"]").unwrap(),
        2.into()
    );
    assert_eq!(eval_str("payment.items[5].price").unwrap(), Value::Void);
    assert_eq!(eval_str("payment.product.name").unwrap(), Value::Void);
    assert_eq!(eval_str("payment[null]").unwrap(), Value::Void);
    assert_eq!(eval_str("nope.deeper[0]").unwrap(), Value::Void);
    assert_eq!(
        eval_str("payment.items | map(|i| i.price * i.qty)").unwrap(),
        vec![20, 5].into()
    );
    // local and captured roots
    assert_eq!(eval_str("(|o| o.a[1])({\"a\": [1, 2]})").unwrap(), 2.into());
    assert_eq!(
        eval_str("(|o| (|x| o.a[1] + x)(1))({\"a\": [1, 2]})").unwrap(),
        3.into()
    );
    // non literal indexes fall back to OpCode::Index
    assert_eq!(
        eval_str("(|i| payment.items[i].price)(0)").unwrap(),
        10.into()
    );
}

#[test]
fn missing_arguments_are_void() {
    assert_eq!(eval_str("(|a, b| b)(1)").unwrap(), Value::Void);
    assert_eq!(eval_str("(|a, b| a)(1)").unwrap(), 1.into());
    assert_eq!(
        eval_str("[1, 2] | map(|a, b| b)").unwrap(),
        vec![Value::Void, Value::Void].into()
    );
}

#[test]
fn nested_callbacks() {
    assert_eq!(
        eval_str("[1, 2] | map(|a| [10, 20] | map(|b| a * b))").unwrap(),
        json!([[10, 20], [20, 40]]).into()
    );
    assert_eq!(
        eval_str("(|m| [1, 2, 3] | map(|v| v * m))(3)").unwrap(),
        vec![3, 6, 9].into()
    );
}
