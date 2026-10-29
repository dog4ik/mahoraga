use std::{collections::HashMap, debug_assert_matches, rc::Rc};

use crate::{
    Env, Node, RuntimeError, Spanned, Value,
    value::{Array, Number, Object, ValueType},
    vm::{closure::VmClosure, opcode::OpCode, stack::Stack},
};

mod chunk;
pub mod closure;
mod compiler;
mod constants;
mod opcode;
mod stack;

#[derive(Debug)]
struct CallFrame {
    chunk: Rc<chunk::Chunk>,
    return_ip: usize,
    /// Stack base index.
    /// Starts with first argument
    base_idx: usize,
}

impl CallFrame {}

pub struct VirtualMachine<'a> {
    frames: Vec<CallFrame>,
    env: &'a Env,
    ip: usize,
    stack: Stack,
}

impl<'a> VirtualMachine<'a> {
    pub fn new(chunk: chunk::Chunk, env: &'a Env) -> Self {
        Self {
            frames: vec![CallFrame {
                chunk: Rc::new(chunk),
                return_ip: 0,
                base_idx: 0,
            }],
            env,
            ip: 0,
            stack: Stack::new(),
        }
    }

    fn eval_op(
        &mut self,
        f: impl Fn(Value, Value) -> Result<Value, RuntimeError>,
    ) -> Result<(), RuntimeError> {
        let rhs = self.stack.pop();
        let lhs = self.stack.pop();
        println!("eval op {lhs} with {rhs}");
        self.stack.push(f(lhs, rhs)?);
        self.ip += 1;
        Ok(())
    }

    fn eval(&mut self) -> std::result::Result<Value, Spanned<RuntimeError>> {
        self.eval_inner().map_err(|e| {
            let frame = self
                .frames
                .last()
                .expect("frame still existing if error happened");
            e.with_span(frame.chunk.code.span_for(self.ip))
        })
    }

    fn eval_inner(&mut self) -> std::result::Result<Value, RuntimeError> {
        loop {
            let frame = self
                .frames
                .last_mut()
                .expect("last frame return should stop evaluation");
            let code = frame.chunk.code.get(self.ip).clone();
            match code {
                OpCode::LoadConstant { constant_idx } => {
                    self.stack
                        .push(frame.chunk.constants.expect_value(constant_idx).clone());
                    self.ip += 1;
                }
                OpCode::Add => {
                    self.eval_op(|lhs, rhs| lhs.add(&rhs))?;
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
                    let object = self.stack.pop();
                    let value = match &object {
                        Value::Object(Object(obj)) => match index {
                            Value::String(s) => obj.get(&s).cloned().unwrap_or_default(),
                            Value::Null | Value::Void => Value::Void,
                            _ => unreachable!("object index can be only string or nullish value"),
                        },
                        Value::Array(Array(arr)) => match index {
                            Value::Number(Number::Int(i)) => {
                                arr.get(i as usize).cloned().unwrap_or_default()
                            }
                            Value::Number(Number::Float(f)) => {
                                arr.get(f as usize).cloned().unwrap_or_default()
                            }
                            Value::Null | Value::Void => Value::Void,
                            _ => unreachable!("array index can be only number or nullish value"),
                        },
                        Value::Null | Value::Void => Value::Void,
                        _ => unreachable!(
                            "only array or hashmap can be indexed, got: {}, ({})",
                            object.value_type(),
                            object,
                        ),
                    };
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
                    match self.stack.0[callee_idx].clone() {
                        Value::Function(calee) => {
                            let args = self.stack.0.split_off(callee_idx + 1);
                            self.stack.pop();
                            self.stack.push(
                                calee.call(
                                    &mut crate::RuntimeEnv::new(self.env),
                                    crate::Args(args),
                                )?,
                            );
                            self.ip += 1;
                        }
                        Value::VmClosure(closure) => {
                            let base_idx = callee_idx + 1;
                            self.frames.push(CallFrame {
                                chunk: closure.chunk.clone(),
                                return_ip: self.ip + 1,
                                base_idx,
                            });
                            self.ip = closure.entry_idx;
                            self.stack.push(Value::VmClosure(closure));
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
                    if self.frames.is_empty() {
                        return Ok(self.stack.pop());
                    }

                    let return_value = self.stack.pop();
                    self.stack.0.drain(frame.base_idx - 1..);
                    self.stack.push(return_value);
                    self.ip = frame.return_ip;
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
                        unreachable!()
                    };
                    self.stack.push(c.captures[idx as usize].clone());
                    self.ip += 1;
                }
                OpCode::MakeClosure {
                    start_idx,
                    captures,
                } => {
                    let at = self.stack.len() - captures as usize;
                    let captures = self.stack.0.split_off(at);
                    self.stack.push(Value::VmClosure(Rc::new(VmClosure {
                        chunk: frame.chunk.clone(),
                        entry_idx: start_idx as usize,
                        captures,
                        arity: 0,
                    })));
                    self.ip += 1;
                }
            }
        }
    }
}

pub fn eval(node: Node, env: &Env) -> crate::Result<Value> {
    let mut complier = compiler::Compiler::new();
    complier.compile(&node);
    let mut vm = complier.into_vm(env);
    Ok(vm.eval()?)
}

#[cfg(test)]
mod tests {
    use crate::vm::compiler::Compiler;

    use super::*;

    fn ev(s: &str) -> Value {
        let mut compiler = Compiler::new();
        compiler.compile(&crate::parser::parse_expr(s).unwrap());
        let env = crate::Env::std();
        let mut vm = compiler.into_vm(&env);
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
    fn basic_compiler() {
        assert_eq!(ev("2+2"), 4.into())
    }

    #[test]
    fn turnary() {
        // assert_eq!(ev("true ? 1 : 2"), 1.into());
        assert_eq!(ev("false ? 2 : 1"), 1.into());
    }

    #[test]
    fn eval_ordering() {
        assert_eq!(ev("!true"), false.into());
        assert_eq!(ev("5 - 3"), 2.into());
        assert_eq!(ev("\"a\" + \"b\""), "ab".into());
        assert_eq!(ev("true ? (false ? 1 : 2) : 3"), 2.into());

        assert_eq!(ev("null ?? 1"), 1.into());
        assert_eq!(ev("5 ?? 1"), 5.into());

        assert_eq!(ev("false || 1"), 1.into());
        assert_eq!(ev("true || 1"), true.into());

        assert_eq!(ev("1 && 0"), 0.into());
        assert_eq!(ev("false && 1"), false.into());
    }

    #[test]
    fn object_access() {
        assert_eq!(ev("constants.pi"), std::f64::consts::PI.into());
    }

    #[test]
    fn array_lit() {
        assert_eq!(
            ev("[10, \"foo\", [[\"test\"]]]"),
            Value::Array(Array(vec![
                10.into(),
                "foo".into(),
                Value::Array(Array(vec![Value::Array(Array(vec!["test".into()]))]))
            ]))
        );
    }

    #[test]
    fn object_lit() {
        assert_eq!(
            ev("{'foo': 'bar', 'baz': 39}"),
            Value::Object(Object(HashMap::from_iter([
                ("foo".to_string(), "bar".into()),
                ("baz".to_string(), 39.into())
            ])))
        );
    }

    #[test]
    fn external_fn_call() {
        // assert_eq!(ev("to_uppercase('bar')"), "BAR".into());
        assert_eq!(ev("'hahaha' | replace_all('ha', 'hi')"), "hihihi".into());
        assert_eq!(ev("replace_all('hahaha', 'ha', 'hi')"), "hihihi".into());
        assert_eq!(ev("' a ' | trim | to_uppercase"), "A".into());
    }

    #[test]
    fn closures() {
        assert_eq!(ev("(|v| v * 2)(4)"), 8.into());
        assert_eq!(ev("(|v| |c| v * c)(4)(2)"), 8.into());

        assert_eq!(ev("(|a, b| a - b)(5, 3)"), 2.into());
        assert_eq!(ev("1 + (|v| v)(2)"), 3.into());
    }
}
