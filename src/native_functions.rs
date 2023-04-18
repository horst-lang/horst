use std::collections::HashMap;
use lazy_static::lazy_static;
use crate::class::Class;
use crate::frame::CallFrame;
use crate::function::NativeFunction;
use crate::value::Value;
use crate::vm::VM;

lazy_static!(
    pub static ref NATIVE_FUNCTIONS: HashMap<String, NativeFunction> = {
        let mut map = HashMap::new();
        map.insert("callback".to_string(), NativeFunction { function: callback });
        map.insert("readln".to_string(), NativeFunction { function: readln });
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

fn callback(args: Vec<Value>, vm: &mut VM) -> Value {
    let mut args = args;
    let callback = if let Value::Function(callback) = args.remove(0) {
        callback
    } else {
        panic!("First argument must be a function");
    };
    let mut callback_args = Vec::new();
    for arg in args {
        callback_args.push(arg);
    }
    let call_stack = vm.call_stack.clone();
    vm.call_stack = vec![CallFrame {
        function: callback,
        ip: 0,
        base_pointer: 0,
    }];
    let result = vm.run();
    vm.call_stack = call_stack;
    result
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
    dbg!(args.clone());
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