use std::collections::HashMap;
use crate::class::Class;
use crate::value::Value;

#[derive(Clone, PartialEq, Debug)]
pub struct Instance {
    pub class: Class,
    pub fields: HashMap<String, Value>,
}

impl Instance {
    pub fn new(class: Class) -> Instance {
        Instance {
            class,
            fields: HashMap::new()
        }
    }
}