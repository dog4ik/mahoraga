use std::rc::Rc;

use crate::{Env, Node, Value, vm::{VirtualMachine, chunk::Chunk}};

pub fn eval(node: &Node, env: &Env) -> crate::Result<Value> {
    let mut compiler = crate::vm::compiler::Compiler::new();
    compiler.compile(node);
    let mut vm = compiler.into_vm(env);
    Ok(vm.eval()?)
}

pub fn eval_chunk(chunk: Rc<Chunk>, env: &Env) -> crate::Result<Value> {
    let mut vm = VirtualMachine::new(chunk, env);
    Ok(vm.eval()?)
}

pub fn compile<'a>(node: &Node) -> Rc<Chunk> {
    let mut compiler = crate::vm::compiler::Compiler::new();
    compiler.compile(node);
    Rc::new(compiler.into_chunk())
}
