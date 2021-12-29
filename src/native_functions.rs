use std::collections::HashMap;
use lazy_static::lazy_static;
use crate::function::NativeFunction;
use crate::value::Value;

lazy_static!(
    pub static ref NATIVE_FUNCTIONS: HashMap<String, NativeFunction> = {
        let mut map = HashMap::new();
        map.insert("print".to_string(), NativeFunction { function: print });
        map.insert("println".to_string(), NativeFunction { function: println });
        map.insert("readln".to_string(), NativeFunction { function: readln });
        map
    };
);

fn print(args: Vec<Value>) -> Value {
    let mut s = String::new();
    for arg in args {
        s.push_str(arg.to_string().as_str());
    }
    print!("{}", s);
    Value::Nil
}

fn println(args: Vec<Value>) -> Value {
    let mut s = String::new();
    for arg in args {
        s.push_str(arg.to_string().as_str());
    }
    println!("{}", s);
    Value::Nil
}

fn readln(_: Vec<Value>) -> Value {
    let mut s = String::new();
    std::io::stdin().read_line(&mut s).unwrap();
    s.pop();
    Value::String(s)
}
