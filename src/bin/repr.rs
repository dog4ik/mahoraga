use std::io::stdin;

use mahoraga::{
    eval::{self, Env},
    parser::parse_expr,
};

fn main() {
    let mut lines = stdin().lines();
    while let Some(Ok(line)) = lines.next() {
        let node = match parse_expr(&line) {
            Ok(node) => node,
            Err(e) => {
                eprintln!("{e}");
                print!("> ");
                continue;
            }
        };
        let env = Env::std();
        match eval::eval(node, &env) {
            Ok(v) => println!("{v}"),
            Err(e) => eprintln!("{e}"),
        }
    }
}
