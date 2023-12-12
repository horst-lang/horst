use std::any::Any;
use std::collections::HashMap;
use crate::class::{Class};
use crate::gc::{GcRef, GcTrace};
use crate::value::Value;

#[derive(Clone, PartialEq, Debug)]
pub struct Instance {
    pub class: GcRef<Class>,
    pub fields: HashMap<String, Value>,
}

impl Instance {
    pub fn new(class: GcRef<Class>) -> Instance {
        Instance {
            class,
            fields: HashMap::new()
        }
    }
}

impl GcTrace for Instance {
    fn size(&self) -> usize {
        std::mem::size_of::<Self>()
    }

    fn trace(&self, _gc: &mut crate::gc::Gc) {
        for (_, value) in self.fields.iter() {
            value.trace(_gc);
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}