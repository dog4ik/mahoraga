use crate::Value;

#[derive(Debug)]
pub struct ConstantsPool(Vec<Value>);

impl ConstantsPool {
    pub const TRUE: u16 = 0;
    pub const FALSE: u16 = 1;
    pub const NULL: u16 = 2;
    pub const VOID: u16 = 3;

    pub fn new() -> Self {
        Self(vec![
            Value::Bool(true),
            Value::Bool(false),
            Value::Null,
            Value::Void,
        ])
    }

    pub fn add_value(&mut self, value: Value) -> u16 {
        let idx = self.0.len();
        self.0.push(value);
        idx as u16
    }

    pub fn expect_value(&self, index: u16) -> &Value {
        self.0
            .get(index as usize)
            .expect("all constant index accesses must be valid")
    }
}

impl Default for ConstantsPool {
    fn default() -> Self {
        Self::new()
    }
}
