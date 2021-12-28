use crate::instruction::Instruction;
use crate::value::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub instructions: Vec<Instruction>,
    pub arity: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativeFunction {
    function: fn(Vec<Value>) -> Value,
    arity: usize,
}

impl Function {
    pub const fn new(instructions: Vec<Instruction>, arity: usize) -> Function {
        Function {
            instructions,
            arity,
        }
    }
}