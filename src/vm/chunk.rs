use crate::{
    span::Span,
    value::Value,
    vm::{
        closure::ClosureInfo,
        constants::ConstantsPool,
        opcode::{Instructions, OpCode},
    },
};

/// Where the access chain starts
#[derive(Debug)]
pub(crate) enum PathRoot {
    Static(String),
    Local(u16),
    Capture(u16),
}

/// Member and literal index chain like `payment.items[0].price`
#[derive(Debug)]
pub(crate) struct AccessPath {
    pub(crate) root: PathRoot,
    /// Index values in access order
    pub(crate) segments: Vec<Value>,
}

#[derive(Debug, Default)]
pub struct Chunk {
    pub(crate) code: Instructions,
    pub(crate) constants: ConstantsPool,
    pub(crate) closures: Vec<ClosureInfo>,
    pub(crate) paths: Vec<AccessPath>,
}

impl Chunk {
    pub(crate) fn emit(&mut self, op: OpCode, span: impl Into<Span>) -> usize {
        self.code.emit(op, span)
    }

    pub(crate) fn load_constant(&mut self, value: Value, span: impl Into<Span>) -> usize {
        let constant_idx = self.constants.add_value(value);
        self.code.emit(OpCode::LoadConstant { constant_idx }, span)
    }

    pub(crate) fn add_path(&mut self, path: AccessPath) -> u16 {
        self.paths.push(path);
        (self.paths.len() - 1) as u16
    }

    pub(crate) fn add_closure(&mut self, closure: ClosureInfo) -> u16 {
        self.closures.push(closure);
        (self.closures.len() - 1) as u16
    }

    pub(crate) fn patch_jump(&mut self, at: usize) {
        self.code.patch_jump(at)
    }
}
