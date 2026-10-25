use crate::{
    Atom, Env, Node, NodeKind, Punct, Value,
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
                            .emit(OpCode::JumpIfNullishNoPop { offset: u16::MAX });
                        self.code.emit(OpCode::Pop);
                        self.compile(*rhs);
                        self.code.patch_jump(jump);
                    }
                    Punct::Or => {
                        let (lhs, rhs) = op;
                        self.compile(*lhs);
                        let jump = self
                            .code
                            .emit(OpCode::JumpIfTruthyNoPop { offset: u16::MAX });
                        self.code.emit(OpCode::Pop);
                        self.compile(*rhs);
                        self.code.patch_jump(jump);
                    }
                    Punct::And => {
                        let (lhs, rhs) = op;
                        self.compile(*lhs);
                        let jump = self
                            .code
                            .emit(OpCode::JumpIfFalsyNoPop { offset: u16::MAX });
                        self.code.emit(OpCode::Pop);
                        self.compile(*rhs);
                        self.code.patch_jump(jump);
                    }
                    Punct::Pipe => todo!(),
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
            NodeKind::Member { object, field, .. } => {
                self.compile(*object);
                self.code.emit(OpCode::LoadConstant {
                    constant_idx: self.constants.add_value(field.0.into()),
                });
                self.code.emit(OpCode::Index);
            }
            NodeKind::Index { object, index } => {
                self.compile(*object);
                self.compile(*index);
                self.code.emit(OpCode::Index);
            }
            NodeKind::Call { callee, args } => {
                self.compile(*callee);
                let count = args.len();
                for arg in args {
                    self.compile(arg);
                }
                self.code.emit(OpCode::FnCall {
                    args_count: count as u16,
                });
            }
            NodeKind::Closure { args, body } => todo!(),
            NodeKind::NonNullishAssertion(node) => {
                self.compile(*node);
                self.code.emit(OpCode::AssertNotNullish);
            }
            NodeKind::Negation(node) => {
                self.compile(*node);
                self.code.emit(OpCode::Not);
            }
            NodeKind::ArrayLit(nodes) => {
                let size = nodes.len();
                for node in nodes {
                    self.compile(node);
                }
                self.code.emit(OpCode::MakeArray { size: size as u16 });
            }
            NodeKind::ObjectLit(hash_map) => {
                let size = hash_map.len();
                for (key, value) in hash_map {
                    self.code.emit(OpCode::LoadConstant {
                        constant_idx: self.constants.add_value(key.into()) as u16,
                    });
                    self.compile(value);
                }
                self.code.emit(OpCode::MakeObject { size: size as u16 });
            }
            NodeKind::Atom(atom) => match atom {
                Atom::Ident(ident) => {
                    self.code.emit(OpCode::StaticAccess {
                        key_constant_idx: self.constants.add_value(ident.0.into()),
                    });
                }
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

    pub fn into_vm(self, env: &Env) -> VirtualMachine<'_> {
        VirtualMachine::new(self.code, self.constants, env)
    }
}
