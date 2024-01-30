use horst::compiler::compile;
use horst::vm::VM;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    let file = &args[1];

    let source = std::fs::read_to_string(file).unwrap();
    let program = compile(&source).unwrap();
    // dbg!(program.clone());
    let mut vm = VM::new();
    vm.interpret(program);
}