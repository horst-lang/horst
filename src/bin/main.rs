use horst::compiler::compile;
use horst::vm::VM;
use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: cargo run <file>");
        std::process::exit(64);
    }

    let file = &args[1];

    if Path::new(file).file_name().unwrap() != "haupt.horst" {
        println!("Using file name 'haupt.horst' is recommended");
    }

    let source = if let Ok(source) = std::fs::read_to_string(file) {
        source
    } else {
        eprintln!("Could not read file");
        std::process::exit(66);
    };

    let mut vm = VM::new(Path::new(file).canonicalize().unwrap().to_str().unwrap().to_string());
    let program = if let Ok(program) = vm.compile(None, &source) {
        program
    } else {
        std::process::exit(65);
    };

    vm.interpret(program);
}