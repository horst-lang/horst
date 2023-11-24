use std::any::Any;
use std::collections::HashMap;
use crate::function::Function;
use crate::gc::{GcRef, GcTrace};
use crate::value::Value;

#[derive(Clone, PartialEq, Debug)]
pub struct Class {
    pub name: String,
    pub methods: HashMap<String, Value>,
}

pub type ClassRef = GcRef<Class>;

impl Class {
    pub fn new(name: String) -> Class {
        Class {
            name,
            methods: HashMap::new(),
        }
    }
}

impl GcTrace for Class {
    fn size(&self) -> usize {
        std::mem::size_of::<Self>()
    }

    fn trace(&self, _gc: &mut crate::gc::Gc) {}

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}