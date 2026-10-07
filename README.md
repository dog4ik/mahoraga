# Mahoraga

Small WIP expression language

## Usage example

```rust
fn my_custom_fn(s: String, n: usize) -> Result<mahoraga::Value, mahoraga::FunctionCallError> {
    Ok(std::iter::repeat_n(s, n).collect::<String>().into())
}

fn main() -> miette::Result<()> {
    let mut env = mahoraga::Env::default();
    assert_eq!(
        mahoraga::eval_str("1 + 2 * 5", &env).unwrap(),
        mahoraga::Value::Number(11.0.into())
    );

    env.fns.extend([mahoraga::declare_fn!(
        my_custom_fn(String, usize),
        "My custom function!"
    )]);

    assert_eq!(
        mahoraga::eval_str("my_custom_fn('test', 2)", &env).unwrap(),
        "testtest".into(),
    );

    let source = "'test' + 2";
    mahoraga::eval_str(source, &env)
        .map_err(|err| miette::Report::new(err).with_source_code(source.to_string()))?;
    Ok(())
}
```
