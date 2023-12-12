use std::fmt;
use crate::class::{Class};
use crate::function::{Function, NativeFunction};
use crate::gc::{GcRef, GcRefRaw, GcTrace};
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
    Class(GcRef<Class>),
    Instance(GcRef<Instance>),
    Closure(Function, Vec<GcRef<UpvalueRegistry>>),
    BoundMethod {
        receiver: GcRef<Instance>,
        function: Function,
        upvalues: Vec<GcRef<UpvalueRegistry>>,
    },
    Foreign(GcRefRaw)
}

pub type InstanceRef = GcRef<Instance>;
pub type UpvalueRegistryRef = GcRef<UpvalueRegistry>;

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
            Value::Foreign(_) => write!(f, "<foreign>")
        }
    }
}

impl GcTrace for Value {
    fn size(&self) -> usize {
        std::mem::size_of::<Self>()
    }

    fn trace(&self, _: &mut crate::gc::Gc) {}

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}