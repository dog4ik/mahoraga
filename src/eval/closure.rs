use std::rc::Rc;

use crate::{
    Args, Ident, Value,
    eval::{self, ExecutionContext},
    parser::Node,
    value::CallableRef,
};

#[derive(Debug, Clone)]
pub struct Closure {
    /// Shared so that calls can bind arguments without allocating their names
    pub args: Rc<[Rc<str>]>,
    pub body: Rc<Node>,
}

impl PartialEq for Closure {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl Closure {
    pub fn new(args: &[Ident], node: Rc<Node>) -> Self {
        Self {
            args: args.iter().map(|Ident(i)| Rc::from(i.as_str())).collect(),
            body: node,
        }
    }

    pub fn call(&self, env: &mut dyn ExecutionContext, args: Args) -> eval::Result<Value> {
        ExecutionContext::call(env, &CallableRef::Closure(self), args)
    }
}
