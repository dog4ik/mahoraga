use crate::{
    Env, RuntimeEnv, Value,
    eval::traverse::eval,
    parse_expr,
    parser::Node,
    value::{Array, Object, ValueType},
};

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum SetEnvError {
    #[error("path is empty")]
    EmptyPath,
    /// `index` is the position of `key` in the path.
    #[error("can't set `{key}` (path segment {index}) on a value of type {found}")]
    NotContainer {
        index: usize,
        key: String,
        found: ValueType,
    },
    #[error("`{key}` (path segment {index}) is not a valid index into an array of length {len}")]
    BadIndex {
        index: usize,
        key: String,
        len: usize,
    },
}

#[derive(Debug, Default)]
pub struct Engine {
    env: Env,
}

fn set_at<S: AsRef<str>>(
    slot: &mut Value,
    path: &[S],
    index: usize,
    value: Value,
) -> Result<Value, SetEnvError> {
    let Some((key, rest)) = path.split_first() else {
        return Ok(std::mem::replace(slot, value));
    };
    let key = key.as_ref();
    if matches!(slot, Value::Null | Value::Void) {
        *slot = Value::Object(Object::default());
    }
    let next = match slot {
        Value::Object(Object(map)) => map.entry(key.to_owned()).or_default(),
        Value::Array(Array(items)) => {
            let len = items.len();
            key.parse::<usize>()
                .ok()
                .and_then(|i| items.get_mut(i))
                .ok_or_else(|| SetEnvError::BadIndex {
                    index,
                    key: key.to_owned(),
                    len,
                })?
        }
        other => {
            return Err(SetEnvError::NotContainer {
                index,
                key: key.to_owned(),
                found: other.value_type(),
            });
        }
    };
    set_at(next, rest, index + 1, value)
}

impl Engine {
    pub fn new(env: Env) -> Self {
        Self { env }
    }

    pub fn env(&self) -> &Env {
        &self.env
    }

    pub fn env_mut(&mut self) -> &mut Env {
        &mut self.env
    }

    pub fn set_env<S: AsRef<str>>(
        &mut self,
        path: &[S],
        value: Value,
    ) -> Result<Value, SetEnvError> {
        let (root, rest) = path.split_first().ok_or(SetEnvError::EmptyPath)?;
        let map = &mut self.env.runtime_objects.0;
        let root = root.as_ref();
        if rest.is_empty() {
            return Ok(map.insert(root.to_owned(), value).unwrap_or_default());
        }
        // a missing root is void, which `set_at` never fails on, so it's never left behind
        set_at(map.entry(root.to_owned()).or_default(), rest, 1, value)
    }

    /// Evaluates an already parsed expression.
    pub fn eval(&self, node: &Node) -> crate::Result<Value> {
        Ok(eval(node, &mut RuntimeEnv::new(&self.env))?)
    }

    pub fn eval_str(&self, s: &str) -> crate::Result<Value> {
        self.eval(&parse_expr(s)?)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::value::Number;

    fn int(i: i64) -> Value {
        Value::Number(Number::Int(i))
    }

    fn obj<const N: usize>(entries: [(&str, Value); N]) -> Value {
        Value::Object(Object(HashMap::from_iter(
            entries.map(|(k, v)| (k.to_owned(), v)),
        )))
    }

    fn engine() -> Engine {
        let mut env = Env::std();
        env.attach_object(Object(HashMap::from_iter([
            (
                "a".to_owned(),
                obj([("b", int(1)), ("s", Value::String("x".into()))]),
            ),
            (
                "list".to_owned(),
                Value::Array(Array(vec![int(10), obj([])])),
            ),
            ("n".to_owned(), Value::Null),
        ])));
        Engine::new(env)
    }

    #[test]
    fn set_replaces_and_returns_previous() {
        let mut e = engine();
        assert_eq!(e.set_env(&["a", "b"], int(5)), Ok(int(1)));
        assert_eq!(e.eval_str("a.b + 1").unwrap(), int(6));
        assert_eq!(e.set_env(&["n"], int(2)), Ok(Value::Null));
        assert_eq!(e.eval_str("n").unwrap(), int(2));
    }

    #[test]
    fn set_creates_missing_path() {
        let mut e = engine();
        assert_eq!(e.set_env(&["x", "y", "z"], int(3)), Ok(Value::Void));
        assert_eq!(e.eval_str("x.y.z").unwrap(), int(3));
        // null and void along the way become objects
        assert_eq!(e.set_env(&["n", "k"], int(4)), Ok(Value::Void));
        assert_eq!(e.eval_str("n.k").unwrap(), int(4));
        assert_eq!(e.set_env(&["a", "missing", "k"], int(1)), Ok(Value::Void));
        assert_eq!(e.eval_str("a.missing.k + a.b").unwrap(), int(2));
    }

    #[test]
    fn set_into_arrays() {
        let mut e = engine();
        assert_eq!(e.set_env(&["list", "0"], int(7)), Ok(int(10)));
        assert_eq!(e.set_env(&["list", "1", "k"], int(8)), Ok(Value::Void));
        assert_eq!(e.eval_str("list[0] + list[1].k").unwrap(), int(15));
        assert_eq!(
            e.set_env(&["list", "2"], int(0)),
            Err(SetEnvError::BadIndex {
                index: 1,
                key: "2".into(),
                len: 2
            })
        );
        assert!(matches!(
            e.set_env(&["list", "x"], int(0)),
            Err(SetEnvError::BadIndex { .. })
        ));
    }

    #[test]
    fn set_errors_leave_env_untouched() {
        let mut e = engine();
        assert_eq!(e.set_env::<&str>(&[], int(0)), Err(SetEnvError::EmptyPath));
        assert_eq!(
            e.set_env(&["a", "s", "k"], int(0)),
            Err(SetEnvError::NotContainer {
                index: 2,
                key: "k".into(),
                found: ValueType::String
            })
        );
        assert_eq!(e.eval_str("a.s").unwrap(), "x".into());
    }

    #[test]
    fn parsed_node_is_reusable() {
        let mut e = engine();
        let node = parse_expr("a.b * 2").unwrap();
        let mut out = Vec::new();
        for i in 0..3 {
            e.set_env(&["a", "b"], int(i)).unwrap();
            out.push(e.eval(&node).unwrap());
        }
        assert_eq!(out, [int(0), int(2), int(4)]);
    }
}
