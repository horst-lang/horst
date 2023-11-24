use horst::compiler::compile;
use horst::vm::VM;

fn main() {
    let source = std::fs::read_to_string("main.horst").unwrap();
    let program = compile(&source).unwrap();
    dbg!(program.clone());
    let mut vm = VM::new();
    vm.interpret(program);


}