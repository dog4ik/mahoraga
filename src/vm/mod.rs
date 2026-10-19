use crate::{
    RuntimeError, Value,
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

pub struct VirtualMachine {
    ip: usize,
    code: Instructions,
    constants: ConstantsPool,
    stack: Stack,
}

impl VirtualMachine {
    pub fn new(code: Instructions, constants: ConstantsPool) -> Self {
        Self {
            ip: 0,
            code,
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

    fn eval(&mut self) -> std::result::Result<Value, RuntimeError> {
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
                    self.eval_op(|lhs, rhs| lhs.mt(&rhs))?;
                }
                OpCode::Gte => {
                    self.eval_op(|lhs, rhs| lhs.mte(&rhs))?;
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
                OpCode::JumpIfNullish { offset } => {
                    let value = self.stack.pop();
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
            }
        }
        Ok(self.stack.pop())
    }
}

#[cfg(test)]
mod tests {
    use crate::vm::compiler::Compiler;

    use super::*;

    fn ev(s: &str) -> Value {
        let mut compiler = Compiler::new();
        compiler.compile(crate::parser::parse_expr(s).unwrap());
        let mut vm = compiler.into_vm();
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
    }
}
