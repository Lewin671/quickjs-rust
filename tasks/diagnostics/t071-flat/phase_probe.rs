use std::{
    hint::black_box,
    io::{self, Write},
};
fn barrier(label: &str) {
    println!("__T071_{label}");
    io::stdout().flush().unwrap();
    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap();
}
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let source = std::fs::read_to_string(&args[2]).unwrap();
    if args[1] == "dump" {
        let ast = qjs_parser::parse_script(&source).unwrap();
        let bc = qjs_runtime::compile_script(&ast).unwrap();
        println!("{bc:#?}");
        return;
    }
    barrier("READY");
    let ast = qjs_parser::parse_script(black_box(&source)).unwrap();
    barrier("PARSED");
    let bc = qjs_runtime::compile_script(black_box(&ast)).unwrap();
    barrier("COMPILED");
    let value = qjs_runtime::eval_bytecode(black_box(&bc)).unwrap();
    black_box(&value);
    barrier("EXECUTED");
}
