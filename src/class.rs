use std::collections::HashMap;
use crate::function::Function;
use crate::value::Value;

#[derive(Clone, PartialEq, Debug)]
pub struct Class {
    pub name: String,
    pub methods: HashMap<String, Value>,
}

impl Class {
    pub fn new(name: String, methods: HashMap<String, Function>) -> Class {
        Class {
            name,
            methods: methods.into_iter().map(|(name, function)| (name, Value::Function(function))).collect()
        }
    }
}