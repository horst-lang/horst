use std::collections::HashMap;
use std::io::Read;
use std::ops::Add;
use lazy_static::lazy_static;
use crate::class::Class;
use crate::frame::CallFrame;
use crate::function::{Function, NativeFunction};
use crate::instance::Instance;
use crate::value::Value;
use crate::vm::{Collectable, VM};
lazy_static!(
    pub static ref NATIVE_FUNCTIONS: HashMap<String, NativeFunction> = {
        let mut map = HashMap::new();
        map.insert("readln".to_string(), NativeFunction { function: readln });
        map.insert("fetch".to_string(), NativeFunction { function: fetch });
        map
    };

    pub static ref NATIVE_CLASSES: HashMap<String, Class> = {
        let mut map = HashMap::new();
        map.insert("Map".to_string(), make_map());
        map.insert("List".to_string(), make_list());
        map
    };
);

fn readln(_: Vec<Value>, vm: &mut VM) -> Value {
    let mut s = String::new();
    std::io::stdin().read_line(&mut s).unwrap();
    s.pop();
    Value::String(s)
}

fn fetch(args: Vec<Value>, vm: &mut VM) -> Value {
    let mut args = args;
    let url = if let Value::String(url) = args.pop().unwrap() {
        url
    } else {
        panic!("First argument must be a string");
    };
    let mut res = reqwest::blocking::get(&url).unwrap();
    let mut body = String::new();
    res.read_to_string(&mut body).unwrap();
    Value::String(body)
}

fn make_map() -> Class {
    let mut methods = HashMap::new();
    methods.insert("get".to_string(), Value::Native(NativeFunction { function: map_get }));
    methods.insert("set".to_string(), Value::Native(NativeFunction { function: map_set }));
    Class {
        name: "Map".to_string(),
        methods,
    }
}

fn map_get(args: Vec<Value>, vm: &mut VM) -> Value {
    let mut args = args;
    let map = if let Value::Instance(map) = args.pop().unwrap() {
        vm.get_instance(map).unwrap()
    } else {
        panic!("First argument must be a map");
    };
    let key = if let Value::String(key) = args.pop().unwrap() {
        key
    } else {
        panic!("Second argument must be a string");
    };
    map.fields.get(&key).unwrap_or(&Value::Nil).clone()
}

fn map_set(args: Vec<Value>, vm: &mut VM) -> Value {
    let mut args = args;
    let mut map = if let Value::Instance(map) = args.pop().unwrap() {
        vm.get_instance_mut(map).unwrap()
    } else {
        panic!("First argument must be a map");
    };
    let key = if let Value::String(key) = args.pop().unwrap() {
        key
    } else {
        panic!("Second argument must be a string");
    };
    let value = args.pop().unwrap();
    map.fields.insert(key, value);
    Value::Nil
}

fn make_list() -> Class {
    let mut methods = HashMap::new();
    methods.insert("init".to_string(), Value::Native(NativeFunction { function: list_init }));
    methods.insert("add".to_string(), Value::Native(NativeFunction { function: list_add }));
    methods.insert("get".to_string(), Value::Native(NativeFunction { function: list_get }));
    methods.insert("toString".to_string(), Value::Native(NativeFunction { function: list_to_string }));
    Class {
        name: "List".to_string(),
        methods,
    }
}

struct List {
    items: Vec<Value>,
}

impl Collectable for List {
    fn collect(&self) -> Vec<usize> {
        let mut ids = vec![];

        for item in &self.items {
            if let Value::Instance(id) = item {
                ids.push(*id);
            } else if let Value::Foreign(id) = item {
                ids.push(*id);
            }
        }

        ids
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

fn list_to_string(args: Vec<Value>, vm: &mut VM)-> Value {
    let mut args = args;
    let list = if let Value::Instance(list) = args.pop().unwrap() {
        vm.get_instance(list).unwrap()
    } else {
        panic!("First argument must be a list");
    };
    let list_items = if let Value::Foreign(id) = list.fields.get("items").unwrap() {
        vm.get_collectable::<List>(*id).unwrap().items.clone()
    } else {
        panic!("List must have a field called items");
    };
    let mut s = String::new();
    s.push_str("List([");
    for (index, item) in list_items.iter().enumerate() {
        match item {
            Value::Instance(id) => {
                let instance = vm.get_instance(*id).unwrap();
                if let Some(Value::Function(function)) = instance.class.methods.get("toString") {
                    let call_stack = vm.call_stack.clone();
                    vm.call_stack = vec![CallFrame {
                        function: function.clone(),
                        ip: 0,
                        base_pointer: 0,
                    }];
                    vm.stack.push(Value::Instance(*id));
                    let result = vm.run();
                    vm.call_stack = call_stack;
                    s.push_str(format!("{}", result).as_str());
                } else if let Some(Value::Native(NativeFunction { function })) = instance.class.methods.get("toString") {
                    let args = vec![Value::Instance(*id)];
                    let result = function(args, vm);
                    s.push_str(format!("{}", result).as_str());
                } else {
                    s.push_str(format!("{}", item).as_str());
                }
            }
            _ => {
                s.push_str(format!("{}", item).as_str());
            }
        }

        if index < list_items.len() - 1 {
            s.push_str(", ");
        }
    }
    s.push_str("])");
    Value::String(s)
}

fn list_init(mut args: Vec<Value>, vm: &mut VM) -> Value {
    let this = if let Value::Instance(this) = args.remove(0) {
        this
    } else {
        panic!("First argument must be a list");
    };
    let mut list = List { items: args };

    // Create a Foreign value first, before getting a mutable reference to the instance
    let foreign_value = Value::Foreign(vm.new_collectable(list));

    // Now, we can get the mutable reference to the instance, and insert the foreign value
    let instance = vm.get_instance_mut(this).unwrap();
    instance.fields.insert("items".to_string(), foreign_value);

    Value::Nil
}

fn list_add(args: Vec<Value>, vm: &mut VM) -> Value {
    let mut args = args;
    let this = if let Value::Instance(this) = args.pop().unwrap() {
        this
    } else {
        panic!("First argument must be a list");
    };

    let items_foreign_value = {
        let instance = vm.get_instance_mut(this).unwrap();
        if let Value::Foreign(items) = instance.fields.get("items").unwrap() {
            *items
        } else {
            panic!("List must have an items field");
        }
    };

    let items = vm.get_collectable_mut::<List>(items_foreign_value).unwrap();

    let items = if let Some(items) = items.as_any_mut().downcast_mut::<List>() {
        items
    } else {
        panic!("List must have an items field");
    };
    items.items.push(args.pop().unwrap());
    Value::Nil
}

fn list_get(args: Vec<Value>, vm: &mut VM) -> Value {
    let mut args = args;
    let this = if let Value::Instance(this) = args.remove(0) {
        this
    } else {
        panic!("First argument must be a list");
    };

    let items_foreign_value = {
        let instance = vm.get_instance_mut(this).unwrap();
        if let Value::Foreign(items) = instance.fields.get("items").unwrap() {
            *items
        } else {
            panic!("List must have an items field");
        }
    };

    let items = vm.get_collectable_mut::<List>(items_foreign_value).unwrap();

    let items = if let Some(items) = items.as_any_mut().downcast_mut::<List>() {
        items
    } else {
        panic!("List must have an items field");
    };
    let index = if let Value::Number(index) = args.remove(0) {
        index as usize
    } else {
        panic!("Second argument must be a number");
    };
    if let Some(item) = items.items.get(index) {
        item.clone()
    } else {
        Value::Nil
    }
}