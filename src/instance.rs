use std::any::Any;
use std::collections::HashMap;
use crate::class::{Class, ClassRef};
use crate::value::Value;
use crate::vm::Collectable;

#[derive(Clone, PartialEq, Debug)]
pub struct Instance {
    pub class: ClassRef,
    pub fields: HashMap<String, Value>,
}

impl Instance {
    pub fn new(class: ClassRef) -> Instance {
        Instance {
            class,
            fields: HashMap::new()
        }
    }
}

impl Collectable for Instance {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}