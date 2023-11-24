use std::any::Any;
use std::collections::HashMap;
use crate::function::Function;
use crate::value::Value;
use crate::vm::Collectable;

#[derive(Clone, PartialEq, Debug)]
pub struct Class {
    pub name: String,
    pub methods: HashMap<String, Value>,
}

pub type ClassRef = usize;

impl Class {
    pub fn new(name: String) -> Class {
        Class {
            name,
            methods: HashMap::new(),
        }
    }
}

impl Collectable for Class {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}