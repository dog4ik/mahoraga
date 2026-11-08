use std::rc::Rc;

use crate::{value::Value, vm::chunk::Chunk};

#[derive(Debug)]
pub struct VmClosure {
    pub(crate) chunk: Rc<Chunk>,
    pub(crate) entry_idx: usize,
    pub(crate) captures: Vec<Value>,
    pub(crate) arity: u8,
}

impl PartialEq for VmClosure {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.chunk, &other.chunk)
    }
}

#[derive(Debug)]
pub(crate) struct ClosureInfo {
    pub(crate) start_idx: u16,
    pub(crate) captures: u8,
    pub(crate) arity: u8,
}
