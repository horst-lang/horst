use std::fmt::Debug;
use crate::instruction::Instruction;
use crate::value::Value;

#[derive(Clone, PartialEq, Debug)]
pub struct Function {
    pub instructions: Vec<Instruction>,
    pub arity: usize,
}

#[derive(Clone, PartialEq, Debug)]
pub struct NativeFunction {
    pub function: fn(Vec<Value>) -> Value,
}

impl Function {
    pub const fn new(instructions: Vec<Instruction>, arity: usize) -> Function {
        Function {
            instructions,
            arity,
        }
    }
}