use std::{collections::HashMap, rc::Rc};

use crate::{
    Env, Node, RuntimeError, Spanned, Value,
    value::{Array, Number, Object, ValueType},
    vm::{
        constants::ConstantsPool,
        opcode::{Instructions, OpCode},
        stack::Stack,
    },
};

mod compiler;
mod constants;
mod opcode;
mod stack;

#[derive(Debug)]
struct CallFrame {
    instructions: Instructions,
    ip: usize,
    base_idx: usize,
}

pub struct VirtualMachine<'a> {
    ip: usize,
    code: Instructions,
    frames: Vec<CallFrame>,
    constants: ConstantsPool,
    env: &'a Env,
    stack: Stack,
}

impl<'a> VirtualMachine<'a> {
    pub fn new(code: Instructions, constants: ConstantsPool, env: &'a Env) -> Self {
        Self {
            ip: 0,
            frames: Vec::new(),
            code,
            env,
            constants,
            stack: Stack::new(),
        }
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

    fn eval(&mut self) -> std::result::Result<Value, Spanned<RuntimeError>> {
        self.eval_inner()
            .map_err(|e| e.with_span(self.code.span_for(self.ip)))
    }

    fn eval_inner(&mut self) -> std::result::Result<Value, RuntimeError> {
        while self.ip < self.code.len() {
            let code = self.code.get(self.ip);
            match code {
                OpCode::LoadConstant { constant_idx } => {
                    self.stack
                        .push(self.constants.expect_value(*constant_idx).clone());
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
                OpCode::JumpIfTruthy { offset } => {
                    let value = self.stack.pop();
                    if value.truthy() {
                        self.ip = *offset as usize
                    } else {
                        self.ip += 1;
                    }
                }
                OpCode::JumpIfTruthyNoPop { offset } => {
                    let value = self.stack.peek();
                    if value.truthy() {
                        self.ip = *offset as usize
                    } else {
                        self.ip += 1;
                    }
                }
                OpCode::JumpIfFalsy { offset } => {
                    let value = self.stack.pop();
                    if !value.truthy() {
                        self.ip = *offset as usize
                    } else {
                        self.ip += 1;
                    };
                }
                OpCode::JumpIfFalsyNoPop { offset } => {
                    let value = self.stack.peek();
                    if !value.truthy() {
                        self.ip = *offset as usize
                    } else {
                        self.ip += 1;
                    };
                }
                OpCode::JumpIfNullishNoPop { offset } => {
                    let value = self.stack.peek();
                    if !value.nullish() {
                        self.ip = *offset as usize
                    } else {
                        self.ip += 1;
                    };
                }
                OpCode::Jump { offset } => {
                    self.ip = *offset as usize;
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
                            "only array or hashmap can be indexed, got: {}",
                            object.value_type()
                        ),
                    };
                    self.stack.push(value);
                    self.ip += 1;
                }
                OpCode::MakeArray { size } => {
                    let size = *size as usize;
                    let array = Vec::from_iter(self.stack.0.drain(self.stack.len() - size..));
                    self.stack.push(array.into());
                    self.ip += 1;
                }
                OpCode::MakeObject { size } => {
                    let size = *size as usize;
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
                    let key = match self.constants.expect_value(*key_constant_idx) {
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
                        .unwrap();
                    self.stack.push(v);
                    self.ip += 1;
                }
                OpCode::FnCall { args_count } => {
                    let mut args = Vec::with_capacity((*args_count) as usize);
                    for _ in 0..*args_count {
                        args.push(self.stack.pop());
                    }
                    let calee = match self.stack.pop() {
                        Value::Function(calee) => calee,
                        rest => {
                            return Err(RuntimeError::UnexpectedType {
                                got: rest.value_type(),
                                expected: ValueType::Function,
                            });
                        }
                    };
                    self.stack.push(
                        calee.call(&mut crate::RuntimeEnv::new(self.env), crate::Args(args))?,
                    );
                    self.ip += 1;
                }
            }
        }
        Ok(self.stack.pop())
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
        let value = vm.eval().unwrap();
        debug_assert!(
            vm.stack.is_empty(),
            "stack must be empty after eval finished"
        );
        value
    }

    #[test]
    fn basic_compiler() {
        assert_eq!(ev("2+2"), 4.into())
    }

    #[test]
    fn turnary() {
        assert_eq!(ev("true ? 1 : 2"), 1.into());
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
        assert_eq!(ev("to_uppercase('bar')"), "BAR".into());
        assert_eq!(ev("'hahaha' | replace_all('ha', 'hi')"), "hihihi".into());
        assert_eq!(ev("replace_all('hahaha', 'ha', 'hi')"), "hihihi".into());
        assert_eq!(ev("' a ' | trim | to_uppercase"), "A".into());
    }
}
