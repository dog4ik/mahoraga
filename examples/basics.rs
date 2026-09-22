fn my_custom_fn(s: String, n: usize) -> mahoraga::Result<mahoraga::Value> {
    Ok(std::iter::repeat_n(s, n).collect::<String>().into())
}

fn main() {
    let mut env = mahoraga::Env::default();
    assert_eq!(
        mahoraga::eval_str("1 + 2 * 5", &env).unwrap(),
        mahoraga::Value::Number(11.)
    );

    env.fns.extend([mahoraga::declare_fn!(
        my_custom_fn(String, usize),
        "My custom function!"
    )]);

    assert_eq!(
        mahoraga::eval_str("my_custom_fn('test', 2)", &env).unwrap(),
        "testtest".into(),
    );
}
