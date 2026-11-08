use std::{rc::Rc, sync::Arc};

use crate::{
    fns::{Args, ExecutionContext},
    lex::Ident,
    parser::Node,
    traverse,
    value::{CallableRef, Value},
};

#[derive(Debug, Clone)]
pub struct Closure {
    /// Shared so that calls can bind arguments without allocating their names
    pub(crate) args: Rc<[Rc<str>]>,
    pub(crate) body: Arc<Node>,
}

impl PartialEq for Closure {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl Closure {
    pub(crate) fn new(args: &[Ident], node: Arc<Node>) -> Self {
        Self {
            args: args.iter().map(|Ident(i)| Rc::from(i.as_str())).collect(),
            body: node,
        }
    }

    pub(crate) fn call(
        &self,
        env: &mut dyn ExecutionContext,
        args: Args,
    ) -> traverse::Result<Value> {
        ExecutionContext::call(env, &CallableRef::Closure(self), args)
    }
}
