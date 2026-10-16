use std::{collections::HashMap, f64::consts, rc::Rc};

use crate::{
    Function, Value,
    eval::{stack::Stack, std_fns},
    value::{Number, Object},
};

/// User provider immutable scope.
#[derive(Debug, Default, Clone)]
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

/// Mutable variable scope used at runtime
pub struct RuntimeEnv<'a> {
    pub env: &'a Env,
    pub current_depth: usize,
    pub stack: Stack,
}

impl RuntimeEnv<'_> {
    pub fn new(env: &Env) -> RuntimeEnv<'_> {
        RuntimeEnv {
            env,
            current_depth: 0,
            stack: Stack::new(),
        }
    }

    pub fn lookup_value(&self, name: &str) -> Option<&Value> {
        self.stack
            .into_iter()
            .find_map(|frame| frame.get(name))
            .or_else(|| self.env.runtime_objects.0.get(name))
    }
}
