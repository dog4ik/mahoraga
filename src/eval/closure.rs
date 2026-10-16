use std::rc::Rc;

use crate::{
    Args, Ident, Value,
    eval::{self, env::RuntimeEnv, stack::StackFrame},
    parser::Node,
};

#[derive(Debug, Clone)]
pub struct Closure {
    pub args: Vec<String>,
    body: Rc<Node>,
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

    pub fn call(&self, env: &mut RuntimeEnv<'_>, Args(args): Args) -> eval::Result<Value> {
        let mut frame = StackFrame::new();
        let mut args = args.into_iter();
        for closure_arg in &self.args {
            let value = args.next().unwrap_or(Value::Void);
            frame.add(closure_arg, value);
        }
        env.stack.push(frame);
        let res = crate::eval(&self.body, env);
        env.stack.pop();
        res
    }
}
