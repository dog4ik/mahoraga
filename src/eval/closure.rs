use std::rc::Rc;

use crate::{
    Args, Ident, Value,
    eval::{self, ExecutionContext},
    parser::Node,
    value::CallableRef,
};

#[derive(Debug, Clone)]
pub struct Closure {
    pub args: Vec<String>,
    pub body: Rc<Node>,
}

impl PartialEq for Closure {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl Closure {
    pub fn new(args: Vec<Ident>, node: Rc<Node>) -> Self {
        Self {
            args: args.into_iter().map(|Ident(i)| i).collect(),
            body: node,
        }
    }

    pub fn call(&self, env: &mut dyn ExecutionContext, args: Args) -> eval::Result<Value> {
        Ok(ExecutionContext::call(
            env,
            &CallableRef::Closure(self),
            args,
        )?)
    }
}
