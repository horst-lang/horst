use std::fmt;
use crate::function::{Function, NativeFunction};

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    String(String),
    Boolean(bool),
    Nil,
    Function(Function),
    Native(NativeFunction),
}

impl Value {
    pub fn is_falsey(&self) -> bool {
        match self {
            Value::Nil | Value::Boolean(false) => true,
            _ => false,
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Nil | Value::Boolean(false) => false,
            _ => true,
        }
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
        }
    }
}