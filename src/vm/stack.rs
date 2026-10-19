use crate::Value;

#[derive(Debug)]
pub struct Stack(pub Vec<Value>);

impl Stack {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn pop(&mut self) -> Value {
        self.0.pop().expect("stack pop should always succeed")
    }

    pub fn push(&mut self, value: Value) {
        self.0.push(value);
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn peek(&self) -> &Value {
        self.0.last().expect("peek always gets value")
    }
}
