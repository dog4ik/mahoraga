use crate::{
    Atom, Node, NodeKind, Punct, Value,
    vm::{
        VirtualMachine,
        constants::ConstantsPool,
        opcode::{Instructions, OpCode},
    },
};

#[derive(Debug)]
pub struct Compiler {
    code: Instructions,
    constants: ConstantsPool,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            code: Instructions::new(),
            constants: ConstantsPool::new(),
        }
    }

    fn compile_op(&mut self, (lhs, rhs): (Box<Node>, Box<Node>), op: OpCode) {
        self.compile(*lhs);
        self.compile(*rhs);
        self.code.emit(op);
    }

    pub fn compile(&mut self, node: Node) {
        match node.kind {
            NodeKind::Op((punct, op)) => {
                match punct {
                    Punct::Add => self.compile_op(op, OpCode::Add),
                    Punct::Mul => self.compile_op(op, OpCode::Mul),
                    Punct::Sub => self.compile_op(op, OpCode::Sub),
                    Punct::Div => self.compile_op(op, OpCode::Div),
                    Punct::Less => self.compile_op(op, OpCode::Lt),
                    Punct::LessOrEq => self.compile_op(op, OpCode::Lte),
                    Punct::More => self.compile_op(op, OpCode::Gt),
                    Punct::MoreOrEq => self.compile_op(op, OpCode::Gte),
                    Punct::CmpEqual => self.compile_op(op, OpCode::Eq),
                    Punct::CmpNotEqual => self.compile_op(op, OpCode::Neq),

                    Punct::Coalesce => {
                        let (lhs, rhs) = op;
                        self.compile(*lhs);
                        let jump = self
                            .code
                            .emit(OpCode::JumpIfNullish { offset: u16::MAX });
                        self.compile(*rhs);
                        self.code.patch_jump(jump);
                    }
                    Punct::Pipe => todo!(),
                    Punct::Or => {
                        let (lhs, rhs) = op;
                        self.compile(*lhs);
                        let jump = self.code.emit(OpCode::JumpIfTruthy { offset: u16::MAX });
                        self.compile(*rhs);
                        self.code.patch_jump(jump);
                    }
                    Punct::And => {
                        let (lhs, rhs) = op;
                        self.compile(*lhs);
                        let jump = self.code.emit(OpCode::JumpIfFalsy { offset: u16::MAX });
                        self.compile(*rhs);
                        self.code.patch_jump(jump);
                    }
                    _ => unreachable!("{punct} is not a valid op"),
                };
            }
            NodeKind::Turnary {
                operand,
                truth_node,
                false_node,
            } => {
                self.compile(*operand);
                let jump_to_false = self.code.emit(OpCode::JumpIfFalsy { offset: u16::MAX });
                self.compile(*truth_node);
                let jump_over_false = self.code.emit(OpCode::Jump { offset: u16::MAX });
                self.code.patch_jump(jump_to_false);
                self.compile(*false_node);
                self.code.patch_jump(jump_over_false);
            }
            NodeKind::Member {
                object,
                field,
                field_span,
            } => todo!(),
            NodeKind::Index { object, index } => todo!(),
            NodeKind::Call { callee, args } => todo!(),
            NodeKind::Closure { args, body } => todo!(),
            NodeKind::NonNullishAssertion(node) => {
                self.compile(*node);
                self.code.emit(OpCode::AssertNotNullish);
            }
            NodeKind::Negation(node) => {
                self.compile(*node);
                self.code.emit(OpCode::Not);
            }
            NodeKind::ArrayLit(nodes) => todo!(),
            NodeKind::ObjectLit(hash_map) => todo!(),
            NodeKind::Atom(atom) => match atom {
                Atom::Ident(ident) => todo!(),
                Atom::StrLit(s) => {
                    self.code.emit(OpCode::LoadConstant {
                        constant_idx: self.constants.add_value(Value::String(s)) as u16,
                    });
                }
                Atom::FloatLit(f) => {
                    self.code.emit(OpCode::LoadConstant {
                        constant_idx: self.constants.add_value(Value::Number(f.into())) as u16,
                    });
                }
                Atom::IntLit(i) => {
                    self.code.emit(OpCode::LoadConstant {
                        constant_idx: self.constants.add_value(Value::Number(i.into())) as u16,
                    });
                }
                Atom::BoolLit(b) => {
                    self.code.emit(OpCode::LoadConstant {
                        constant_idx: match b {
                            true => ConstantsPool::TRUE,
                            false => ConstantsPool::FALSE,
                        },
                    });
                }
                Atom::NullLit => {
                    self.code.emit(OpCode::LoadConstant {
                        constant_idx: ConstantsPool::NULL,
                    });
                }
                Atom::VoidLit => {
                    self.code.emit(OpCode::LoadConstant {
                        constant_idx: ConstantsPool::VOID,
                    });
                }
            },
        }
    }

    pub fn into_vm(self) -> VirtualMachine {
        VirtualMachine::new(self.code, self.constants)
    }
}

impl From<Compiler> for VirtualMachine {
    fn from(value: Compiler) -> Self {
        value.into_vm()
    }
}
