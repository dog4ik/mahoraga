#[derive(Debug)]
pub enum OpCode {
    LoadConstant {
        constant_idx: u16,
    },
    Add,
    Sub,
    Mul,
    Div,
    Lt,
    Lte,
    Gt,
    Gte,
    Eq,
    Neq,
    Not,
    Jump {
        offset: u16,
    },
    JumpIfFalsy {
        offset: u16,
    },
    JumpIfTruthy {
        offset: u16,
    },
    JumpIfNullishNoPop {
        offset: u16,
    },
    JumpIfFalsyNoPop {
        offset: u16,
    },
    JumpIfTruthyNoPop {
        offset: u16,
    },
    AssertNotNullish,
    /// Pop a single value from the stack
    Pop,
    MakeArray {
        size: u16,
    },
    MakeObject {
        size: u16,
    },
    Index,
    StaticAccess {
        key_constant_idx: u16,
    },
    FnCall {
        args_count: u16,
    },
}

#[derive(Debug)]
pub struct Instructions(Vec<OpCode>);

impl Instructions {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn emit(&mut self, op: OpCode) -> usize {
        self.0.push(op);
        self.0.len() - 1
    }

    /// Make jump opcode point to the next instruction
    pub fn patch_jump(&mut self, at: usize) {
        let target = self.0.len() as u32;
        match &mut self.0[at] {
            OpCode::Jump { offset }
            | OpCode::JumpIfFalsy { offset }
            | OpCode::JumpIfTruthy { offset }
            | OpCode::JumpIfFalsyNoPop { offset }
            | OpCode::JumpIfTruthyNoPop { offset } => *offset = target as u16,
            OpCode::JumpIfNullishNoPop { offset } => *offset = target as u16,
            rest => unreachable!("only jump opcodes can be patched, got {rest:?}"),
        }
    }

    pub fn get(&mut self, n: usize) -> &OpCode {
        &self.0[n]
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
