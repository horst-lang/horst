use std::collections::HashMap;
use std::fmt;
use std::fmt::Debug;
use crate::frame::Chunk;
use crate::gc::GcRef;
use crate::instruction::Instruction;
use crate::module::Module;
use crate::value::Value;
use crate::vm::{FunctionUpvalue, VM};

#[derive(Clone, PartialEq, Debug)]
pub struct Function {
    pub name: String,
    pub arity: usize,
    pub chunk: Chunk,
    pub upvalues: Vec<FunctionUpvalue>,
    pub module: GcRef<Module>
}


#[derive(Clone)]
pub struct NativeFunction {
    pub function: fn(Vec<Value>, &mut VM) -> Value,
}

impl PartialEq for NativeFunction {
    fn eq(&self, other: &Self) -> bool {
        // Compare the raw function pointers
        let self_fn_ptr = self.function as *const ();
        let other_fn_ptr = other.function as *const ();

        self_fn_ptr == other_fn_ptr
    }
}

impl fmt::Debug for NativeFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Get the raw function pointer and cast it to a usize
        let fn_ptr = self.function as usize;

        // Format the output as a hexadecimal address
        write!(f, "NativeFunction {{ function: {:x?} }}", fn_ptr)
    }
}


impl Function {
    pub fn new<S: Into<String>>(name: S, arity: usize, chunk: Chunk, module: GcRef<Module>) -> Function {
        Function {
            name: name.into(),
            arity,
            chunk,
            upvalues: Vec::new(),
            module
        }
    }
}