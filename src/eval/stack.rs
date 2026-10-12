use std::{collections::HashMap, iter::Rev, rc::Rc, slice};

use crate::Value;

#[derive(Debug)]
pub struct StackFrame {
    pub name: Rc<str>,
    pub scope: HashMap<String, Value>,
}

impl StackFrame {
    pub fn new(name: impl Into<Rc<str>>) -> Self {
        Self {
            name: name.into(),
            scope: HashMap::new(),
        }
    }
}

impl<'a> IntoIterator for &'a Stack {
    type Item = &'a StackFrame;
    type IntoIter = Rev<slice::Iter<'a, StackFrame>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter().rev()
    }
}

impl<'a> IntoIterator for &'a mut Stack {
    type Item = &'a mut StackFrame;
    type IntoIter = Rev<slice::IterMut<'a, StackFrame>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut().rev()
    }
}

#[derive(Debug, Default)]
pub struct Stack(Vec<StackFrame>);

impl Stack {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn pop(&mut self) -> Option<StackFrame> {
        self.0.pop()
    }

    pub fn push(&mut self, frame: StackFrame) {
        self.0.push(frame);
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
