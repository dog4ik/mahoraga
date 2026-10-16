use std::{iter::Rev, slice};

use crate::Value;

#[derive(Debug)]
pub struct StackFrame {
    scope: Vec<(String, Value)>,
}

impl StackFrame {
    pub fn new() -> Self {
        Self { scope: Vec::new() }
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.scope.iter().find(|(k, _)| k == name).map(|(_, v)| v)
    }

    pub fn add(&mut self, name: impl Into<String>, value: Value) {
        self.scope.push((name.into(), value));
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
