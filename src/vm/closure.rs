use std::rc::Rc;

use crate::{Value, vm::chunk::Chunk};

#[derive(Debug)]
pub struct VmClosure {
    pub chunk: Rc<Chunk>,
    pub entry_idx: usize,
    pub captures: Vec<Value>,
    pub arity: u8,
}

impl PartialEq for VmClosure {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.chunk, &other.chunk)
    }
}

#[derive(Debug)]
pub struct ClosureInfo {
    entry_instruction_idx: usize,
}
