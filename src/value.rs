use std::fmt;
use crate::class::Class;
use crate::function::{Function, NativeFunction};
use crate::instance::Instance;
use crate::vm::{Collectable, VM};

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
    Foreign(usize),
}

impl Value {
    pub fn is_falsey(&self) -> bool {
        matches!(self, Value::Nil | Value::Boolean(false))
    }

    pub fn is_truthy(&self) -> bool {
        !matches!(self, Value::Nil | Value::Boolean(false))
    }

    pub fn to_string(&self, vm: &VM) -> String {
        return match self {
            Value::Number(n) => format!("{}", n),
            Value::String(s) => format!("{}", s),
            Value::Boolean(b) => format!("{}", b),
            Value::Nil => format!("nil"),
            Value::Function(_) => format!("<function>"),
            Value::Native(_) => format!("<native fn>"),
            Value::Class(c) => format!("class {}", c.name),
            Value::Instance(i) => format!("<class instance #{}>", i),
            Value::Foreign(f) => if let Some(foreign) = vm.heap.get(f) {
                if let Some(to_string) = foreign.to_string(vm) {
                    format!("{}", to_string)
                } else {
                    format!("<foreign>")
                }
            } else {
                format!("<foreign>")
            },
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
            Value::Class(c) => write!(f, "class {}", c.name),
            Value::Instance(i) => write!(f, "<class instance #{}>", i),
            Value::Foreign(_) => write!(f, "<foreign>"),
        }
    }
}

impl Collectable for Value {
    fn collect(&self) -> Vec<usize> {
        match self {
            Value::Instance(id) => vec![*id],
            Value::Foreign(id) => vec![*id],
            _ => vec![],
        }
    }

    fn as_any(&self) -> &dyn Any { self }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn to_string(&self, vm: &VM) -> Option<String> {
        Some(self.to_string(vm))
    }
}
