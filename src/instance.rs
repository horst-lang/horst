use std::any::Any;
use std::collections::HashMap;
use crate::class::{Class, ClassRef};
use crate::gc::GcTrace;
use crate::value::Value;

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