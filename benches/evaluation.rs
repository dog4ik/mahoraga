use mahoraga::{Env, Value, parse_expr, traverse::RuntimeEnv};
fn main() {
    divan::main();
}

#[derive(Debug, Clone, Copy)]
enum Executor {
    Vm,
    Traverse,
}

use divan::{Bencher, black_box};

#[divan::bench(args = [Executor::Vm, Executor::Traverse], sample_size = 1000, sample_count = 100)]
fn basic_eval(bencher: Bencher, executor: Executor) {
    let mut env = Env::std();
    env.runtime_objects.0.insert(
        "test".to_string(),
        Vec::from_iter(std::iter::repeat_n(Value::String("hello ".into()), 100)).into(),
    );
    let node =
        parse_expr("test | map(|v| len(v)) | map(|v| v | to_s | to_uppercase) | join('')").unwrap();

    match executor {
        Executor::Vm => {
            let compiled = mahoraga::bytecode::compile(&node);
            bencher.bench_local(|| {
                mahoraga::bytecode::eval_chunk(black_box(compiled.clone()), black_box(&env))
                    .unwrap()
            });
        }
        Executor::Traverse => bencher
            // fresh RuntimeEnv per iteration, built outside the timed region
            .with_inputs(|| RuntimeEnv::new(&env))
            .bench_local_refs(|rt| mahoraga::traverse::eval(black_box(&node), rt).unwrap()),
    }
}
