use crate::Span;

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
pub struct Instructions {
    code: Vec<OpCode>,
    spans: Vec<Span>,
}

impl Instructions {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            spans: Vec::new(),
        }
    }

    pub fn span_for(&self, instruction_idx: usize) -> Span {
        self.spans[instruction_idx]
    }

    pub fn emit(&mut self, op: OpCode, span: impl Into<Span>) -> usize {
        self.code.push(op);
        self.spans.push(span.into());
        self.code.len() - 1
    }

    /// Make jump opcode point to the next instruction
    pub fn patch_jump(&mut self, at: usize) {
        let target = self.code.len() as u32;
        match &mut self.code[at] {
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
        &self.code[n]
    }

    pub fn len(&self) -> usize {
        self.code.len()
    }

    pub fn is_empty(&self) -> bool {
        self.code.is_empty()
    }
}
