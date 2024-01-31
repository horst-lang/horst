use crate::vm::{FunctionUpvalue, UpvalueRegistry};

#[derive(Clone, Debug, PartialEq)]
pub enum Instruction {
    // constants
    Constant(usize),

    // immediates
    Nil,
    True,
    False,

    // actions
    Pop,
    GetGlobal(usize),
    DefineGlobal(usize),
    SetGlobal(usize),
    GetLocal(usize),
    SetLocal(usize),
    GetUpvalue(usize),
    SetUpvalue(usize),
    GetProperty(usize),
    SetProperty(usize),
    GetSuper(usize),

    // operators
    Equal,
    Greater,
    Less,
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Not,
    Negate,

    // control flow
    Print,
    Jump(usize),
    JumpIfFalse(usize),
    Loop(usize),
    Call(usize),
    Invoke(usize, usize),
    SuperInvoke(usize, usize),
    Closure(usize),
    CloseUpvalue,
    Return,
    Class(usize),
    Inherit,
    Method(usize),
}