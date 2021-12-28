

use horst::{
    scanner::{Scanner},
    compiler::{Compiler},
    vm::{VM},
};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        println!("Usage: {} <file>", args[0]);
        return;
    }
    let filename = &args[1];
    let contents = std::fs::read_to_string(filename).expect("Something went wrong reading the file");

    let mut scanner = Scanner::new(contents);
    scanner.scan_tokens();

    let mut compiler = Compiler::new(scanner.tokens);
    let program = compiler.compile();

    let mut vm = VM::new(program);
    let result = vm.run();

    println!("Program exited with {}", result);
}