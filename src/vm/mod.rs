use std::{collections::HashMap, rc::Rc};

use crate::{
    env::Env,
    error::RuntimeError,
    fns::{Args, Callable, ExecutionContext},
    parser::Node,
    span::Spanned,
    value::{Array, CallableRef, Number, Object, Value, ValueType},
    vm::{
        chunk::{Chunk, PathRoot},
        closure::{ClosureInfo, VmClosure},
        compiler::Compiler,
        opcode::OpCode,
        stack::Stack,
    },
};

pub mod chunk;
pub mod closure;
mod compiler;
mod constants;
mod opcode;
mod stack;

/// Compiles `node` to bytecode and evaluates it.
pub fn eval(node: &Node, env: &Env) -> crate::Result<Value> {
    let mut vm = VirtualMachine::new(compile(node), env);
    Ok(vm.eval()?)
}

/// Compiles `node` to a reusable bytecode [Chunk].
pub fn compile(node: &Node) -> Rc<Chunk> {
    let mut compiler = Compiler::new();
    compiler.compile(node);
    Rc::new(compiler.into_chunk())
}

/// Evaluates a chunk produced by [compile].
pub fn eval_chunk(chunk: Rc<Chunk>, env: &Env) -> crate::Result<Value> {
    let mut vm = VirtualMachine::new(chunk, env);
    Ok(vm.eval()?)
}

const MAX_STACK: usize = 500;

#[derive(Debug)]
struct CallFrame {
    chunk: Rc<chunk::Chunk>,
    return_ip: usize,
    /// Stack base index.
    /// Starts with first argument
    base_idx: usize,
}

pub(crate) struct VirtualMachine<'a> {
    frames: Vec<CallFrame>,
    env: &'a Env,
    ip: usize,
    stack: Stack,
}

impl<'a> VirtualMachine<'a> {
    pub(crate) fn new(chunk: Rc<chunk::Chunk>, env: &'a Env) -> Self {
        Self {
            frames: vec![CallFrame {
                chunk,
                return_ip: 0,
                base_idx: 0,
            }],
            env,
            ip: 0,
            stack: Stack::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn stack_values(&self) -> &[Value] {
        &self.stack.0
    }

    fn eval_op(
        &mut self,
        f: impl Fn(Value, Value) -> Result<Value, RuntimeError>,
    ) -> Result<(), RuntimeError> {
        let rhs = self.stack.pop();
        let lhs = self.stack.pop();
        self.stack.push(f(lhs, rhs)?);
        self.ip += 1;
        Ok(())
    }

    pub(crate) fn eval(&mut self) -> std::result::Result<Value, Spanned<RuntimeError>> {
        self.run(0)
    }

    /// Callbacks from native functions reenter the vm with their own `stop_depth`.
    fn run(&mut self, stop_depth: usize) -> std::result::Result<Value, Spanned<RuntimeError>> {
        self.eval_inner(stop_depth).map_err(|e| {
            let frame = self
                .frames
                .last()
                .expect("frame still existing if error happened");
            e.with_span(frame.chunk.code.span_for(self.ip))
        })
    }

    fn eval_inner(&mut self, stop_depth: usize) -> std::result::Result<Value, RuntimeError> {
        loop {
            let frame = self
                .frames
                .last_mut()
                .expect("last frame return should stop evaluation");
            let code = *frame.chunk.code.get(self.ip);
            match code {
                OpCode::LoadConstant { constant_idx } => {
                    self.stack
                        .push(frame.chunk.constants.expect_value(constant_idx).clone());
                    self.ip += 1;
                }
                OpCode::Add => {
                    self.eval_op(|lhs, rhs| lhs.add_owned(&rhs))?;
                }
                OpCode::Sub => {
                    self.eval_op(|lhs, rhs| lhs.sub(&rhs))?;
                }
                OpCode::Mul => {
                    self.eval_op(|lhs, rhs| lhs.mul(&rhs))?;
                }
                OpCode::Div => {
                    self.eval_op(|lhs, rhs| lhs.div(&rhs))?;
                }
                OpCode::Lt => {
                    self.eval_op(|lhs, rhs| lhs.lt(&rhs))?;
                }
                OpCode::Lte => {
                    self.eval_op(|lhs, rhs| lhs.lte(&rhs))?;
                }
                OpCode::Gt => {
                    self.eval_op(|lhs, rhs| lhs.gt(&rhs))?;
                }
                OpCode::Gte => {
                    self.eval_op(|lhs, rhs| lhs.gte(&rhs))?;
                }
                OpCode::Eq => {
                    self.eval_op(|lhs, rhs| lhs.eq(&rhs))?;
                }
                OpCode::Neq => {
                    self.eval_op(|lhs, rhs| lhs.neq(&rhs))?;
                }
                OpCode::Not => {
                    let value = self.stack.pop();
                    self.stack.push(Value::Bool(!value.truthy()));
                    self.ip += 1;
                }
                OpCode::AssertNotNullish => {
                    let value = self.stack.peek();
                    if value.nullish() {
                        return Err(RuntimeError::NonNullishAssertion);
                    }
                    self.ip += 1;
                }
                OpCode::JumpIfTruthy { position } => {
                    let value = self.stack.pop();
                    if value.truthy() {
                        self.ip = position as usize
                    } else {
                        self.ip += 1;
                    }
                }
                OpCode::JumpIfTruthyNoPop { position } => {
                    let value = self.stack.peek();
                    if value.truthy() {
                        self.ip = position as usize
                    } else {
                        self.ip += 1;
                    }
                }
                OpCode::JumpIfFalsy { position } => {
                    let value = self.stack.pop();
                    if !value.truthy() {
                        self.ip = position as usize
                    } else {
                        self.ip += 1;
                    };
                }
                OpCode::JumpIfFalsyNoPop { position } => {
                    let value = self.stack.peek();
                    if !value.truthy() {
                        self.ip = position as usize
                    } else {
                        self.ip += 1;
                    };
                }
                OpCode::JumpIfNullishNoPop { position } => {
                    let value = self.stack.peek();
                    if !value.nullish() {
                        self.ip = position as usize
                    } else {
                        self.ip += 1;
                    };
                }
                OpCode::Jump { position } => {
                    self.ip = position as usize;
                }
                OpCode::Pop => {
                    self.stack.pop();
                    self.ip += 1;
                }
                OpCode::Index => {
                    let index = self.stack.pop();
                    // The object is a temporary, so the value can be moved out of it
                    let value = match self.stack.pop() {
                        Value::Object(Object(mut obj)) => match index {
                            Value::String(s) => obj.remove(&s),
                            Value::Null | Value::Void => None,
                            _ => return Err(RuntimeError::InvalidObjectKey(index.value_type())),
                        },
                        Value::Array(Array(mut arr)) => array_position(&index)?
                            .filter(|&i| i < arr.len())
                            .map(|i| arr.swap_remove(i)),
                        Value::Null | Value::Void => None,
                        rest => return Err(RuntimeError::InvalidIndexOperand(rest.value_type())),
                    };
                    self.stack.push(value.unwrap_or_default());
                    self.ip += 1;
                }
                OpCode::LoadPath { path_idx } => {
                    let path = &frame.chunk.paths[path_idx as usize];
                    let root = match &path.root {
                        PathRoot::Local(slot) => &self.stack.0[frame.base_idx + *slot as usize],
                        PathRoot::Capture(idx) => {
                            let Value::VmClosure(c) = &self.stack.0[frame.base_idx - 1] else {
                                unreachable!(
                                    "closure value should live right below the arguments on the stack"
                                )
                            };
                            &c.captures[*idx as usize]
                        }
                        PathRoot::Static(key) => match self.env.runtime_objects.0.get(key) {
                            Some(value) => value,
                            None => &self
                                .env
                                .fns
                                .get(key.as_str())
                                .map(|f| Value::Function(f.clone()))
                                .unwrap_or_default(),
                        },
                    };
                    let value = path
                        .segments
                        .iter()
                        .try_fold(Some(root), |object, index| match object {
                            Some(object) => index_ref(object, index),
                            None => Ok(None),
                        })?
                        .cloned()
                        .unwrap_or_default();
                    self.stack.push(value);
                    self.ip += 1;
                }
                OpCode::MakeArray { size } => {
                    let size = size as usize;
                    let array = Vec::from_iter(self.stack.0.drain(self.stack.len() - size..));
                    self.stack.push(array.into());
                    self.ip += 1;
                }
                OpCode::MakeObject { size } => {
                    let size = size as usize;
                    let mut object = HashMap::with_capacity(size);
                    for _ in 0..size {
                        let value = self.stack.pop();
                        let key = match self.stack.pop() {
                            Value::String(s) => s,
                            rest => {
                                unreachable!(
                                    "unexpected object key value type to be string, got: {}",
                                    rest.value_type()
                                )
                            }
                        };
                        object.insert(key, value);
                    }
                    self.stack.push(Value::Object(Object(object)));
                    self.ip += 1;
                }
                OpCode::StaticAccess { key_constant_idx } => {
                    let key = match frame.chunk.constants.expect_value(key_constant_idx) {
                        Value::String(key) => key,
                        _ => panic!("static accessor should be string"),
                    };
                    let v = self
                        .env
                        .runtime_objects
                        .0
                        .get(key)
                        .cloned()
                        .or_else(|| Some(Value::Function(self.env.fns.get(key.as_str())?.clone())))
                        .unwrap_or_default();
                    self.stack.push(v);
                    self.ip += 1;
                }
                OpCode::FnCall { args_count } => {
                    let callee_idx = self.stack.len() - args_count as usize - 1;
                    match &self.stack.0[callee_idx] {
                        Value::Function(_) => {
                            let args = self.stack.0.split_off(callee_idx + 1);
                            let Value::Function(callee) = self.stack.pop() else {
                                unreachable!("callee was just matched as a function")
                            };
                            let return_value = callee.call(self, Args(args))?;
                            self.stack.push(return_value);
                            self.ip += 1;
                        }
                        Value::Closure(_) => {
                            panic!("closures created in traverse evaluator can't be called with vm")
                        }
                        Value::VmClosure(closure) => {
                            let call_frame = CallFrame {
                                chunk: closure.chunk.clone(),
                                return_ip: self.ip + 1,
                                base_idx: callee_idx + 1,
                            };
                            self.ip = closure.entry_idx;
                            for _ in args_count..closure.arity as u16 {
                                self.stack.push(Value::Void);
                            }
                            if self.frames.len() > MAX_STACK {
                                return Err(RuntimeError::DepthLimitReached);
                            }
                            self.frames.push(call_frame);
                        }
                        rest => {
                            return Err(RuntimeError::UnexpectedType {
                                got: rest.value_type(),
                                expected: ValueType::Function,
                            });
                        }
                    };
                }
                OpCode::Return => {
                    let frame = self.frames.pop().expect("at least one frame");
                    let return_value = self.stack.pop();
                    if self.frames.is_empty() {
                        return Ok(return_value);
                    }

                    self.stack.0.truncate(frame.base_idx - 1);
                    self.ip = frame.return_ip;
                    if self.frames.len() == stop_depth {
                        return Ok(return_value);
                    }
                    self.stack.push(return_value);
                }
                OpCode::LoadLocal { slot } => {
                    self.stack.push(
                        self.stack
                            .0
                            .get(frame.base_idx + slot as usize)
                            .expect("each local is loaded in the stack in OpCode::MakeClosure")
                            .clone(),
                    );
                    self.ip += 1;
                }
                OpCode::LoadCapture { idx } => {
                    let Value::VmClosure(c) = &self.stack.0[frame.base_idx - 1] else {
                        unreachable!(
                            "closure value should live right below the arguments on the stack"
                        )
                    };
                    self.stack.push(c.captures[idx as usize].clone());
                    self.ip += 1;
                }
                OpCode::MakeClosure { closure_idx } => {
                    let ClosureInfo {
                        start_idx,
                        captures,
                        arity,
                    } = frame.chunk.closures[closure_idx as usize];
                    let at = self.stack.len() - captures as usize;
                    let captures = self.stack.0.split_off(at);
                    self.stack.push(Value::VmClosure(Rc::new(VmClosure {
                        chunk: frame.chunk.clone(),
                        entry_idx: start_idx as usize,
                        captures,
                        arity,
                    })));
                    self.ip += 1;
                }
            }
        }
    }
}

impl ExecutionContext for VirtualMachine<'_> {
    fn call(
        &mut self,
        f: &CallableRef,
        args: Args,
    ) -> std::result::Result<Value, Spanned<RuntimeError>> {
        match f {
            CallableRef::Closure(_) => {
                panic!("calling traverse evaluator closures inside vm is not supported")
            }
            CallableRef::ExternalFunction(function) => {
                let value = function.f.call(self, args).map_err(|e| {
                    let frame = self.frames.last().expect("at least one frame");
                    e.with_span(frame.chunk.code.span_for(self.ip))
                })?;
                Ok(value)
            }
            CallableRef::VmClosure(vm_closure) => {
                // Same stack layout as OpCode::FnCall: closure followed by the arguments
                let stop_depth = self.frames.len();
                let return_ip = self.ip;
                let base_idx = self.stack.len() + 1;
                self.stack.push(Value::VmClosure(Rc::clone(vm_closure)));
                let args_count = args.len();
                self.stack.0.extend(args.0);
                for _ in args_count..vm_closure.arity as usize {
                    self.stack.push(Value::Void);
                }
                self.frames.push(CallFrame {
                    chunk: vm_closure.chunk.clone(),
                    return_ip,
                    base_idx,
                });
                self.ip = vm_closure.entry_idx;

                let result = self.run(stop_depth);
                if result.is_err() {
                    // Unwind the callback so the caller keeps a consistent vm state
                    self.frames.truncate(stop_depth);
                    self.stack.0.truncate(base_idx - 1);
                    self.ip = return_ip;
                }
                result
            }
        }
    }
}

/// Index semantics shared by `OpCode::Index` and `OpCode::LoadPath`.
/// `None` means a missing value
fn index_ref<'v>(object: &'v Value, index: &Value) -> Result<Option<&'v Value>, RuntimeError> {
    Ok(match object {
        Value::Object(Object(obj)) => match index {
            Value::String(s) => obj.get(s),
            Value::Null | Value::Void => None,
            _ => return Err(RuntimeError::InvalidObjectKey(index.value_type())),
        },
        Value::Array(Array(arr)) => array_position(index)?.and_then(|idx| arr.get(idx)),
        Value::Null | Value::Void => None,
        _ => Err(RuntimeError::InvalidIndexOperand(object.value_type()))?,
    })
}

fn array_position(index: &Value) -> Result<Option<usize>, RuntimeError> {
    Ok(match index {
        Value::Number(Number::Int(i)) => Some(*i as usize),
        Value::Number(Number::Float(f)) => Some(*f as usize),
        Value::Null | Value::Void => None,
        _ => Err(RuntimeError::InvalidArrayIndex(index.value_type()))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_expr;

    fn ev(s: &str) -> Value {
        let env = Env::std();
        let mut vm = VirtualMachine::new(compile(&parse_expr(s).unwrap()), &env);
        dbg!(&vm.frames);
        let value = vm.eval().unwrap();
        debug_assert!(
            vm.stack.is_empty(),
            "stack must be empty after eval finished, got {:?}",
            vm.stack,
        );
        value
    }

    #[test]
    fn capturing_closures() {
        assert_eq!(ev("(|v| |c| v * c)(4)(2)"), 8.into());
        assert_eq!(ev("(|v| |v2| v2 * v)(3)(2)"), 6.into());
    }

    #[test]
    fn callback_error_unwinds() {
        let env = Env::std();
        let node = parse_expr("[1, 2] | map(|v| v * {})").unwrap();
        let mut vm = VirtualMachine::new(compile(&node), &env);
        assert!(vm.eval().is_err());
        assert_eq!(vm.frames.len(), 1);
    }
}
