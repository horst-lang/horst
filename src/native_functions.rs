use std::collections::HashMap;
use std::io::Read;
use std::ops::{Add, Deref};
use lazy_static::lazy_static;
use crate::class::Class;
use crate::frame::CallFrame;
use crate::function::{Function, NativeFunction};
use crate::instance::Instance;
use crate::value::Value;
use crate::vm::{VM};
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
    methods.insert("get".to_string(), Value::NativeFunction(NativeFunction { function: map_get }));
    methods.insert("set".to_string(), Value::NativeFunction(NativeFunction { function: map_set }));
    methods.insert("toString".to_string(), Value::NativeFunction(NativeFunction { function: map_to_string }));
    Class {
        name: "Map".to_string(),
        methods,
    }
}

fn map_get(args: Vec<Value>, vm: &mut VM) -> Value {
    let mut args = args;
    let map = if let Value::Instance(map) = args.remove(0) {
        vm.gc.deref(map)
    } else {
        panic!("First argument must be a map");
    };
    let key = if let Value::String(key) = args.remove(0) {
        key
    } else {
        panic!("Second argument must be a string");
    };
    map.fields.get(&key).unwrap_or(&Value::Nil).clone()
}

fn map_set(args: Vec<Value>, vm: &mut VM) -> Value {
    println!("{:?}", args);
    let mut args = args;
    let mut map = if let Value::Instance(map) = args.remove(0) {
        vm.gc.deref_mut(map)
    } else {
        panic!("First argument must be a map");
    };
    let key = if let Value::String(key) = args.remove(0) {
        key
    } else {
        panic!("Second argument must be a string");
    };
    let value = args.pop().unwrap();
    map.fields.insert(key, value);
    Value::Nil
}

fn map_to_string(args: Vec<Value>, vm: &mut VM) -> Value {
    let mut args = args;
    let map = if let Value::Instance(map) = args.pop().unwrap() {
        vm.gc.deref(map)
    } else {
        panic!("First argument must be a map");
    };
    let mut s = "{".to_string();
    for (i, (key, value)) in map.fields.iter().enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        if let Value::String(value) = value {
            s.push_str(&format!("{}: \"{}\"", key, value));
        } else {
            s.push_str(&format!("{}: {}", key, value));
        }
    }
    s.push('}');
    Value::String(s)
}