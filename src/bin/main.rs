use horst::compiler::compile;
use horst::vm::VM;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: cargo run <file>");
        std::process::exit(64);
    }

    let file = &args[1];

    let source = if let Ok(source) = std::fs::read_to_string(file) {
        source
    } else {
        eprintln!("Could not read file");
        std::process::exit(66);
    };

    let program = if let Ok(program) = compile(&source) {
        program
    } else {
        std::process::exit(65);
    };

    let mut vm = VM::new();
    vm.interpret(program);
}