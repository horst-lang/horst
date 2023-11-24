use std::fmt;
use crate::class::{Class, ClassRef};
use crate::function::{Function, NativeFunction};
use crate::instance::Instance;
use crate::vm::UpvalueRegistry;

#[derive(Clone, PartialEq, Debug)]
pub enum Value {
    Number(f64),
    String(String),
    Boolean(bool),
    Nil,
    Function(Function),
    NativeFunction(NativeFunction),
    Class(ClassRef),
    Instance(InstanceRef),
    Closure(Function, Vec<UpvalueRegistryRef>),
    BoundMethod {
        receiver: InstanceRef,
        function: Function,
        upvalues: Vec<UpvalueRegistryRef>,
    },
}

pub type InstanceRef = usize;
pub type UpvalueRegistryRef = usize;

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
            Value::NativeFunction(_) => write!(f, "<native fn>"),
            Value::Class(c) => write!(f, "<class>"),
            Value::Instance(i) => write!(f, "<class instance>"),
            Value::Closure(_, _) => write!(f, "<closure>"),
            Value::BoundMethod { receiver, function, .. } => {
                write!(f, "<bound method {}>", function.name)
            }
        }
    }
}