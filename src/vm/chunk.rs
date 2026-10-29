use crate::{
    Span, Value,
    vm::{
        closure::ClosureInfo,
        constants::ConstantsPool,
        opcode::{Instructions, OpCode},
    },
};

#[derive(Debug, Default)]
pub struct Chunk {
    pub code: Instructions,
    pub constants: ConstantsPool,
    pub closures: Vec<ClosureInfo>,
}

impl Chunk {
    pub fn emit(&mut self, op: OpCode, span: impl Into<Span>) -> usize {
        self.code.emit(op, span)
    }

    pub fn load_constant(&mut self, value: Value, span: impl Into<Span>) -> usize {
        let constant_idx = self.constants.add_value(value);
        self.code.emit(OpCode::LoadConstant { constant_idx }, span)
    }

    pub fn patch_jump(&mut self, at: usize) {
        self.code.patch_jump(at)
    }
}
