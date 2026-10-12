use crate::{
    Args, Ident, Value,
    eval::{self, env::RuntimeEnv, stack::StackFrame},
    parser::Node,
};

#[derive(Debug, Clone)]
pub struct Closure {
    pub args: Vec<String>,
    body: Box<Node>,
}

impl PartialEq for Closure {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl Closure {
    pub fn new(args: Vec<Ident>, node: Box<Node>) -> Self {
        Self {
            args: args.into_iter().map(|Ident(i)| i).collect(),
            body: node,
        }
    }

    pub fn call(&self, env: &mut RuntimeEnv<'_>, Args(args): Args) -> eval::Result<Value> {
        let mut frame = StackFrame::new("anonymous_closure");
        for (i, closure_arg) in self.args.iter().enumerate() {
            let value = args.get(i).unwrap_or(&Value::Void);
            frame.scope.insert(closure_arg.clone(), value.clone());
        }
        env.stack.push(frame);
        let res = crate::eval(&self.body, env)?;
        env.stack.pop();
        Ok(res)
    }
}
