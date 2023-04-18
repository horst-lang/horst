use std::fmt;
use crate::class::Class;
use crate::function::{Function, NativeFunction};
use crate::instance::Instance;

#[derive(Clone, PartialEq, Debug)]
pub enum Value {
    Number(f64),
    String(String),
    Boolean(bool),
    Nil,
    Function(Function),
    Native(NativeFunction),
    Class(Class),
    Instance(usize),
}

impl Value {
    pub fn is_falsey(&self) -> bool {
        matches!(self, Value::Nil | Value::Boolean(false))
    }

    pub fn is_truthy(&self) -> bool {
        !matches!(self, Value::Nil | Value::Boolean(false))
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{}", s),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Nil => write!(f, "nil"),
            Value::Function(_) => write!(f, "<function>"),
            Value::Native(_) => write!(f, "<native fn>"),
            Value::Class(c) => write!(f, "class {}", c.name),
            Value::Instance(i) => write!(f, "<class instance #{}>", i),
        }
    }
}