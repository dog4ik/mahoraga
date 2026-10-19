use crate::{
    Atom, Env, Ident, Node, NodeKind, Punct, Span, Value,
    value::ValueType,
    vm::{
        VirtualMachine,
        chunk::Chunk,
        constants::ConstantsPool,
        opcode::{Instructions, OpCode},
    },
};

#[derive(Debug, thiserror::Error)]
pub enum CompilerError {
    #[error("uncallable type {got}")]
    UncallableType { got: ValueType },
    #[error("reached max expression depth limit")]
    ExpressionDepthLimit,
}

#[derive(Debug)]
pub struct CompiledFn {
    code: Instructions,
    constants: ConstantsPool,
}

#[derive(Debug)]
pub struct Capture {
    name: String,
    from: Access,
}

#[derive(Debug, Clone, Copy)]
enum Access {
    Local(u16),
    Capture(u16),
}

#[derive(Debug)]
pub struct FnScope {
    captures: Vec<Capture>,
    arg_names: Vec<String>,
}

impl FnScope {
    pub fn new() -> Self {
        Self {
            captures: Vec::new(),
            arg_names: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct Compiler {
    chunk: Chunk,
    scopes: Vec<FnScope>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            chunk: Chunk::default(),
            scopes: vec![FnScope::new()],
        }
    }

    pub fn compile(&mut self, node: &Node) {
        self.compile_inner(node);
        self.chunk.emit(OpCode::Return, node.span);
    }

    pub fn into_vm(self, env: &Env) -> VirtualMachine<'_> {
        VirtualMachine::new(self.chunk, env)
    }

    fn compile_op(&mut self, (lhs, rhs): (&Node, &Node), op: OpCode) {
        self.compile_inner(lhs);
        self.compile_inner(rhs);
        self.chunk.emit(op, lhs.span.start..rhs.span.end);
    }

    fn resolve_ident(&mut self, depth: usize, ident: &Ident) -> Option<Access> {
        if let Some(slot) = self.scopes[depth]
            .arg_names
            .iter()
            .position(|c| c == &ident.0)
        {
            return Some(Access::Local(slot as u16));
        }
        if depth == 0 {
            return None;
        }

        let from = self.resolve_ident(depth - 1, ident)?;
        let captures = &mut self.scopes[depth].captures;
        let idx = match captures.iter().position(|c| c.name == ident.0) {
            Some(i) => i,
            None => {
                captures.push(Capture {
                    name: ident.0.to_owned(),
                    from,
                });
                captures.len() - 1
            }
        };

        Some(Access::Capture(idx as u16))
    }

    fn compile_closure(&mut self, args: &[Ident], body: &Node, span: Span) {
        let jump_over_closure = self.chunk.emit(OpCode::Jump { position: u16::MAX }, span);
        let closure_start = self.chunk.code.len();
        self.scopes.push(FnScope {
            captures: Vec::new(),
            arg_names: args.into_iter().map(|Ident(i)| i.to_string()).collect(),
        });
        self.compile_inner(body);
        let scope = self.scopes.pop().expect("scope was just pushed");

        self.chunk.emit(OpCode::Return, span);
        self.chunk.patch_jump(jump_over_closure);

        for c in &scope.captures {
            self.chunk.emit(
                match c.from {
                    Access::Local(slot) => OpCode::LoadLocal { slot },
                    Access::Capture(idx) => OpCode::LoadCapture { idx },
                },
                span,
            );
        }

        self.chunk.emit(
            OpCode::MakeClosure {
                start_idx: closure_start as u16,
                captures: scope.captures.len() as u8,
            },
            span,
        );
    }

    fn compile_identifier(&mut self, ident: &Ident, span: Span) {
        match self.resolve_ident(self.scopes.len() - 1, ident) {
            Some(Access::Local(local)) => self.chunk.emit(OpCode::LoadLocal { slot: local }, span),
            Some(Access::Capture(idx)) => self.chunk.emit(OpCode::LoadCapture { idx }, span),
            None => {
                let key_constant_idx = self.chunk.constants.add_value(ident.0.clone().into());
                self.chunk
                    .emit(OpCode::StaticAccess { key_constant_idx }, span)
            }
        };
    }

    fn compile_inner(&mut self, node: &Node) {
        match &node.kind {
            NodeKind::Op((punct, (lhs, rhs))) => {
                let op = (lhs.as_ref(), rhs.as_ref());
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
                        self.compile_inner(lhs);
                        let jump = self
                            .chunk
                            .emit(OpCode::JumpIfNullishNoPop { position: u16::MAX }, node.span);
                        self.chunk.emit(OpCode::Pop, node.span);
                        self.compile_inner(rhs);
                        self.chunk.patch_jump(jump);
                    }
                    Punct::Or => {
                        let (lhs, rhs) = op;
                        self.compile_inner(lhs);
                        let jump = self
                            .chunk
                            .emit(OpCode::JumpIfTruthyNoPop { position: u16::MAX }, node.span);
                        self.chunk.emit(OpCode::Pop, node.span);
                        self.compile_inner(rhs);
                        self.chunk.patch_jump(jump);
                    }
                    Punct::And => {
                        let (lhs, rhs) = op;
                        self.compile_inner(lhs);
                        let jump = self
                            .chunk
                            .emit(OpCode::JumpIfFalsyNoPop { position: u16::MAX }, node.span);
                        self.chunk.emit(OpCode::Pop, node.span);
                        self.compile_inner(rhs);
                        self.chunk.patch_jump(jump);
                    }
                    Punct::Pipe => {
                        let (callee, rest) = match &rhs.kind {
                            NodeKind::Call { callee, args } => (&**callee, args),
                            _ => (&**rhs, &Vec::new()),
                        };
                        self.compile_inner(callee);
                        self.compile_inner(lhs);
                        for arg in rest {
                            self.compile_inner(arg);
                        }
                        let argc = rest.len() + 1;
                        self.chunk.emit(
                            OpCode::FnCall {
                                args_count: argc as u16,
                            },
                            rhs.span,
                        );
                    }
                    _ => unreachable!("{punct} is not a valid op"),
                };
            }
            NodeKind::Turnary {
                operand,
                truth_node,
                false_node,
            } => {
                self.compile_inner(operand);
                let jump_to_false = self
                    .chunk
                    .emit(OpCode::JumpIfFalsy { position: u16::MAX }, false_node.span);
                self.compile_inner(truth_node);
                let jump_over_false = self
                    .chunk
                    .emit(OpCode::Jump { position: u16::MAX }, truth_node.span);
                self.chunk.patch_jump(jump_to_false);
                self.compile_inner(false_node);
                self.chunk.patch_jump(jump_over_false);
            }
            NodeKind::Member {
                object,
                field,
                field_span,
            } => {
                self.compile_inner(object);
                self.chunk
                    .load_constant(field.0.clone().into(), *field_span);
                self.chunk.emit(OpCode::Index, *field_span);
            }
            NodeKind::Index { object, index } => {
                self.compile_inner(object);
                self.compile_inner(index);
                self.chunk.emit(OpCode::Index, index.span);
            }
            NodeKind::Call { callee, args } => {
                let count = args.len();
                self.compile_inner(callee);
                for arg in args {
                    self.compile_inner(arg);
                }
                self.chunk.emit(
                    OpCode::FnCall {
                        args_count: count as u16,
                    },
                    node.span,
                );
            }
            NodeKind::Closure { args, body } => {
                self.compile_closure(args, body, node.span);
            }
            NodeKind::NonNullishAssertion(node) => {
                self.compile_inner(node);
                self.chunk.emit(OpCode::AssertNotNullish, node.span);
            }
            NodeKind::Negation(node) => {
                self.compile_inner(node);
                self.chunk.emit(OpCode::Not, node.span);
            }
            NodeKind::ArrayLit(nodes) => {
                let size = nodes.len();
                for node in nodes {
                    self.compile_inner(node);
                }
                self.chunk
                    .emit(OpCode::MakeArray { size: size as u16 }, node.span);
            }
            NodeKind::ObjectLit(hash_map) => {
                let size = hash_map.len();
                for (key, value) in hash_map {
                    self.chunk.load_constant(key.clone().into(), node.span);
                    self.compile_inner(value);
                }
                self.chunk
                    .emit(OpCode::MakeObject { size: size as u16 }, node.span);
            }
            NodeKind::Atom(atom) => match atom {
                Atom::Ident(ident) => {
                    self.compile_identifier(ident, node.span);
                }
                Atom::StrLit(s) => {
                    self.chunk
                        .load_constant(Value::String(s.clone()), node.span);
                }
                Atom::FloatLit(f) => {
                    self.chunk
                        .load_constant(Value::Number((*f).into()), node.span);
                }
                Atom::IntLit(i) => {
                    self.chunk
                        .load_constant(Value::Number((*i).into()), node.span);
                }
                Atom::BoolLit(b) => {
                    self.chunk.emit(
                        OpCode::LoadConstant {
                            constant_idx: match b {
                                true => ConstantsPool::TRUE,
                                false => ConstantsPool::FALSE,
                            },
                        },
                        node.span,
                    );
                }
                Atom::NullLit => {
                    self.chunk.emit(
                        OpCode::LoadConstant {
                            constant_idx: ConstantsPool::NULL,
                        },
                        node.span,
                    );
                }
                Atom::VoidLit => {
                    self.chunk.emit(
                        OpCode::LoadConstant {
                            constant_idx: ConstantsPool::VOID,
                        },
                        node.span,
                    );
                }
            },
        }
    }
}
