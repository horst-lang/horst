use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use crate::class::{Class};
use crate::frame::CallFrame;
use crate::function::Function;
use crate::gc::{Gc, GcRef, GcTrace};
use crate::instance::Instance;
use crate::instruction::Instruction;
use crate::native_functions::{make_number, make_random, make_readln};
use crate::value::{InstanceRef, UpvalueRegistryRef, Value};

pub struct VM {
    stack: Vec<Value>,
    frames: Vec<CallFrame>,
    globals: HashMap<String, Value>,
    open_upvalues: Vec<UpvalueRegistryRef>,
    pub(crate) gc: Gc,
}

impl VM {
    pub fn new() -> VM {
        VM {
            stack: Vec::new(),
            frames: Vec::new(),
            globals: HashMap::new(),
            open_upvalues: Vec::new(),
            gc: Gc::new(),
        }
    }

    pub fn interpret(&mut self, function: Function) {
        let closure = Value::Closure(function, Vec::new());
        self.init_globals();
        self.push(closure.clone());
        self.call_value(closure, 0);
        self.run();
    }

    fn init_globals(&mut self) {
        self.globals.insert("readln".to_string(), Value::NativeFunction(make_readln()));
        self.globals.insert("number".to_string(), Value::NativeFunction(make_number()));
        self.globals.insert("random".to_string(), Value::NativeFunction(make_random()));
    }

    fn run(&mut self) -> Value {
        macro_rules! binary_op {
            ($op:tt, $type:tt) => {
                let b = self.pop();
                let a = self.pop();

                if let (Value::Number(a), Value::Number(b)) = (a.clone(), b.clone()) {
                    self.push(Value::$type(a $op b ));
                } else {
                    println!("{} {}", a, b);
                    // Print stack trace
                    for frame in self.frames.iter().rev() {
                        let function = frame.function.clone();
                        let chunk = function.chunk.clone();
                        let line = chunk.find_line(frame.ip).0;
                        println!("[line {}] in {}()", line, function.name);
                    }
                    panic!("Invalid operands for binary operation.");
                }
            };
        }

        loop {
            let instruction: Instruction = self.get_current_instruction();
            //dbg!(self.stack.clone());
            //dbg!(instruction.clone());
            self.frame_mut().ip += 1;

            match instruction {
                Instruction::Constant(index) => {
                    let constant = self.read_constant(index).clone();
                    self.stack.push(constant);
                }
                Instruction::Nil => self.stack.push(Value::Nil),
                Instruction::True => self.stack.push(Value::Boolean(true)),
                Instruction::False => self.stack.push(Value::Boolean(false)),
                Instruction::Pop => { self.stack.pop(); },
                Instruction::GetGlobal(index) => self.get_global(index),
                Instruction::DefineGlobal(index) => self.define_global(index),
                Instruction::SetGlobal(index) => self.set_global(index),
                Instruction::GetLocal(index) => self.get_local(index),
                Instruction::SetLocal(index) => self.set_local(index),
                Instruction::GetUpvalue(index) => self.get_upvalue(index),
                Instruction::SetUpvalue(index) => self.set_upvalue(index),
                Instruction::GetProperty(index) => self.get_property(index),
                Instruction::SetProperty(index) => self.set_property(index),
                Instruction::GetSuper(index) => self.get_super(index),
                Instruction::Equal => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(Value::Boolean(a == b));
                }
                Instruction::Greater => { binary_op!(>, Boolean); },
                Instruction::Less => { binary_op!(<, Boolean); },
                Instruction::Subtract => { binary_op!(-, Number); },
                Instruction::Multiply => { binary_op!(*, Number); },
                Instruction::Divide => { binary_op!(/, Number); },
                Instruction::Modulo => { binary_op!(%, Number); },
                Instruction::Add => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    match (a, b) {
                        (Value::Number(a), Value::Number(b)) => self.stack.push(Value::Number(a + b)),
                        (Value::String(a), Value::String(b)) => self.stack.push(Value::String(a + &b)),
                        (Value::String(a), b) => self.stack.push(Value::String(a + &b.to_string())),
                        _ => panic!("Operands must be two numbers or two strings."),
                    }
                }
                Instruction::Not => {
                    let value = self.stack.pop().unwrap();
                    self.stack.push(Value::Boolean(value.is_falsey()));
                }
                Instruction::Negate => {
                    let value = self.stack.pop().unwrap();
                    if let Value::Number(value) = value {
                        self.stack.push(Value::Number(-value));
                    } else {
                        panic!("Operand must be a number.");
                    }
                }
                Instruction::Print => {
                    let value = self.stack.pop().unwrap();
                    if let Value::Instance(i) = value {
                        let methods = &self.gc.deref(self.gc.deref(i).class).methods;
                        if methods.contains_key( "toString") {
                            let frames = self.frames.clone();
                            self.frames = vec![];
                            self.stack.push(value);
                            self.invoke("toString".to_string(), 0);
                            let result = self.run();
                            self.frames = frames;
                            println!("{}", result);
                        }
                    } else {
                        println!("{}", value);
                    }
                }
                Instruction::Jump(offset) => {
                    self.frame_mut().ip += offset;
                }
                Instruction::JumpIfFalse(offset) => {
                    let value = self.peek(0).unwrap();
                    if value.is_falsey() {
                        self.frame_mut().ip += offset;
                    }
                }
                Instruction::Loop(offset) => {
                    self.frame_mut().ip -= offset;
                }
                Instruction::Call(arg_count) => {
                    self.call_value_from_stack(arg_count);
                }
                Instruction::Invoke(index, arg_count) => {
                    let name = self.read_string(index);
                    self.invoke(name, arg_count);
                }
                Instruction::SuperInvoke(index, arg_count) => {
                    let name = self.read_string(index);
                    let superclass = self.stack.pop().unwrap();
                    match superclass {
                        Value::Class(class) => {
                            self.invoke_from_class(class, name, arg_count);
                        }
                        _ => panic!("Only classes have superclass."),
                    }
                }
                Instruction::Closure(index) => self.make_closure(index),
                Instruction::CloseUpvalue => {
                    let index = self.stack.len().checked_sub(1).unwrap();
                    self.close_upvalues(index);
                    self.stack.pop();
                }
                Instruction::Return => {
                    let base = self.frame().base;
                    let result = self.stack.pop().unwrap();
                    self.close_upvalues(base);
                    self.frames.pop();
                    if self.frames.is_empty() {
                        self.stack.pop();
                        return result;
                    }
                    self.stack.truncate(base);
                    self.stack.push(result);
                }
                Instruction::Class(index) => {
                    let name = self.read_string(index);
                    let class = Class::new(name);
                    let value = Value::Class(self.alloc(class));
                    self.stack.push(value);
                }
                Instruction::Inherit => {
                    if let (Value::Class(mut subclass_ref), Value::Class(superclass_ref)) =
                        (self.stack.pop().unwrap(), self.peek(0).unwrap()) {
                        let superclass = self.gc.deref(*superclass_ref).clone();
                        let mut subclass = self.gc.deref_mut(subclass_ref);
                        for (name, method) in &superclass.methods {
                            subclass.methods.insert(name.clone(), method.clone());
                        }
                    } else {
                        panic!("Superclass must be a class.");
                    }
                }
                Instruction::Method(index) => self.define_method(index),
            }
        }
    }

    fn get_current_instruction(&self) -> Instruction {
        let frame = self.frame();
        frame.chunk().code[frame.ip].clone()
    }

    fn frame(&self) -> &CallFrame {
        self.frames.last().unwrap()
    }

    fn frame_mut(&mut self) -> &mut CallFrame {
        self.frames.last_mut().unwrap()
    }

    fn read_constant(&self, index: usize) -> &Value {
        let frame = self.frame();
        frame.chunk().read_constant(index)
    }

    fn peek(&self, distance: usize) -> Option<&Value> {
        let index = self.stack.len().checked_sub(1 + distance)?;
        self.stack.get(index)
    }

    fn read_string(&self, index: usize) -> String {
        let value = self.read_constant(index);
        match value {
            Value::String(string) => string.clone(),
            _ => panic!("Value is not a string."),
        }
    }

    fn get_global(&mut self, index: usize) {
        let name = self.read_string(index);
        if let Some(value) = self.globals.get(&name) {
            self.stack.push(value.clone());
        } else {
            panic!("Undefined variable '{}'.", name);
        }
    }

    fn define_global(&mut self, index: usize) {
        let name = self.read_string(index);
        let value = self.stack.pop().unwrap();
        self.globals.insert(name, value);
    }

    fn set_global(&mut self, index: usize) {
        let name = self.read_string(index);
        if self.globals.contains_key(&name) {
            let value = self.peek(0).unwrap().clone();
            self.globals.insert(name, value);
        } else {
            panic!("Undefined variable '{}'.", name);
        }
    }

    fn get_local(&mut self, index: usize) {
        let base = self.frame().base;
        let value = self.stack[base + index].clone();
        self.stack.push(value);
    }

    fn set_local(&mut self, index: usize) {
        let base = self.frame().base;
        let value = self.peek(0).unwrap().clone();
        self.stack[base + index] = value;
    }

    fn make_closure(&mut self, index: usize) {
        let constant = self.read_constant(index).clone();
        if let Value::Function(function) = constant {
            let mut upvalues = Vec::new();
            let upvals = function.upvalues.clone();
            for FunctionUpvalue { is_local, index } in upvals {
                upvalues.push(if is_local {
                    self.capture_upvalue(self.frame().base + index)
                } else {
                    self.frame().get_upvalue(index)
                });
            }

            let closure = Value::Closure(function, upvalues);
            self.stack.push(closure);
        } else {
            panic!("Value is not a function.");
        }
    }

    fn capture_upvalue(&mut self, index: usize) -> UpvalueRegistryRef {
        if let Some(upvalue) = self.open_upvalues.iter().find(|upvalue| matches!(self.gc.deref(upvalue.clone().clone()), UpvalueRegistry::Open(index))) {
            upvalue.clone()
        } else {
            let upvalue = UpvalueRegistry::Open(index);
            let upvalue_ref = self.alloc(upvalue);
            self.open_upvalues.push(upvalue_ref);
            upvalue_ref
        }
    }

    fn get_upvalue(&mut self, index: usize) {
        let upvalue_ref = self.frame().get_upvalue(index);
        let upvalue = self.gc.deref(upvalue_ref).clone();
        match upvalue {
            UpvalueRegistry::Open(index) => {
                let value = self.stack[index].clone();
                self.stack.push(value);
            }
            UpvalueRegistry::Closed(value) => {
                self.stack.push(value);
            }
        }
    }

    fn set_upvalue(&mut self, index: usize) {
        let value = self.peek(0).unwrap().clone();
        let upvalue_ref = self.frame_mut().get_upvalue(index);
        let upvalue = self.gc.deref_mut(upvalue_ref);
        match upvalue {
            UpvalueRegistry::Open(index) => {
                let index = *index;
                self.stack[index] = value;
            }
            UpvalueRegistry::Closed(ref mut cell) => {
                *cell = value;
            }
        }
    }

    fn close_upvalues(&mut self, index: usize) {
        while let Some(upvalue_ref) = self.open_upvalues.last() {
            let upvalue = self.gc.deref(*upvalue_ref);
            let slot = if let UpvalueRegistry::Open(slot) = upvalue {
                if *slot <= index {
                    break;
                }
                *slot
            } else {
                panic!("Expected open upvalue.");
            };
            let upvalue_ref = self.open_upvalues.pop().unwrap();
            let value = self.stack[slot].clone();
            let upvalue = self.gc.deref_mut(upvalue_ref);
            upvalue.close(value);
        }
    }

    fn get_property(&mut self, index: usize) {
        let name = self.read_string(index);
        let instance = self.stack.pop().unwrap();
        match instance {
            Value::Instance(instance_ref) => {
                let instance = self.gc.deref(instance_ref).clone();
                if let Some(value) = instance.fields.get(&name) {
                    self.stack.push(value.clone());
                } else {
                    self.bind_method(instance.class, instance_ref, name);
                }
            }
            _ => {
                // Print stack trace
                for frame in self.frames.iter().rev() {
                    let function = frame.function.clone();
                    let chunk = function.chunk.clone();
                    let line = chunk.find_line(frame.ip).0;
                    println!("[line {}] in {}()", line, function.name);
                }
                panic!("Only instances have properties. { }", instance)
            }
        }
    }

    fn bind_method(&mut self, class: GcRef<Class>, instance: InstanceRef, name: String) {
        let class = self.gc.deref(class).clone();
        if let Some(method) = class.methods.get(&name) {
            let (function, upvalues) = match method {
                Value::Function(f) => (f, Vec::new()),
                Value::Closure(f, u) => (f, u.clone()),
                _ => panic!("Expected function or closure."),
            };

            self.stack.push(Value::BoundMethod {
                receiver: instance,
                function: function.clone(),
                upvalues,
            });
        } else {
            panic!("Undefined property '{}'.", name);
        }
    }

    fn set_property(&mut self, index: usize) {
        let name = self.read_string(index);
        let value = self.pop().clone();
        let instance = self.pop().clone();

        match instance {
            Value::Instance(instance_ref) => {
                let mut instance = self.gc.deref_mut(instance_ref);
                instance.fields.insert(name, value.clone());
            }
            _ => panic!("Only instances have fields."),
        }
        self.push(value);
    }

    fn get_super(&mut self, index: usize) {
        let (this_val, super_val) = (self.stack.pop().unwrap(), self.stack.pop().unwrap());
        if let (Value::Class(super_class), Value::Instance(this)) = (super_val, this_val) {
            let name = self.read_string(index);
            self.bind_method(super_class, this, name);
        } else {
            panic!("Superclass must be a class.")
        }
    }

    fn define_method(&mut self, index: usize) {
        let method = self.stack.pop().unwrap();
        let class = self.peek(0).unwrap().clone();
        if let Value::Class(class) = class {
            let name = self.read_string(index);
            let class = self.gc.deref_mut(class);
            class.methods.insert(name, method);
        } else {
            panic!("Expected class.");
        }
    }

    fn invoke(&mut self, method: String, arg_count: usize) {
        let receiver = self.peek(arg_count).unwrap().clone();
        match receiver {
            Value::Instance(instance_ref) => {
                let instance = self.gc.deref(instance_ref).clone();
                if let Some(method) = instance.fields.get(&method) {
                    let l = self.stack.len();
                    self.stack[l - arg_count - 1] = method.clone();
                    self.call_value_from_stack(arg_count);
                } else {
                    let class = instance.class;
                    self.invoke_from_class(class, method, arg_count);
                }
            }
            _ => panic!("Only instances have methods."),
        }
    }

    fn invoke_from_class(&mut self, class: GcRef<Class>, method: String, arg_count: usize) {
        let class = self.gc.deref(class).clone();
        if let Some(method) = class.methods.get(&method) {
            self.call_value(method.clone(), arg_count);
        } else {
            panic!("Undefined property '{}'.", method);
        }
    }

    fn call_value(&mut self, callee: Value, arg_count: usize) {
        match callee {
            Value::Closure(function, upvalues) => {
                self.call(function, upvalues, arg_count);
            }
            Value::Function(function) => {
                self.call(function, Vec::new(), arg_count);
            }
            Value::Class(class) => {
                let instance = Instance::new(class.clone());
                let instance_ref = self.alloc(instance);
                let value = Value::Instance(instance_ref);
                let l = self.stack.len();
                self.stack[l - arg_count - 1] = value;

                let class = self.gc.deref(class).clone();
                if let Some(init) = class.methods.get("init") {
                    match init {
                        Value::Closure(function, upvalues) => {
                            self.call(function.clone(), upvalues.clone(), arg_count);
                        }
                        Value::Function(function) => {
                            self.call(function.clone(), Vec::new(), arg_count);
                        }
                        _ => panic!("Expected function."),
                    }
                } else if arg_count != 0 {
                    panic!("Expected 0 arguments but got {}.", arg_count);
                }
            }
            Value::BoundMethod {
                receiver,
                function,
                upvalues,
            } => {
                let l = self.stack.len();
                self.stack[l - arg_count - 1] = Value::Instance(receiver);
                self.call(function, upvalues, arg_count);
            }
            Value::NativeFunction(function) => {
                let from = self.stack.len() - arg_count;
                let args = self.stack[from..].to_vec();
                let result = (function.function)(args, self);
                self.pop_many(arg_count + 1);
                self.stack.push(result);
            }
            _ => panic!("Can only call functions and classes."),
        }
    }

    fn call_value_from_stack(&mut self, arg_count: usize) {
        let callee = self.peek(arg_count).unwrap().clone();
        self.call_value(callee, arg_count);
    }

    fn call(&mut self, function: Function, upvalues: Vec<UpvalueRegistryRef>, arg_count: usize) {
        if arg_count != function.arity {
            panic!(
                "Expected {} arguments but got {}.",
                function.arity, arg_count
            );
        }
        self.frames.push(CallFrame {
            function,
            ip: 0,
            base: self.stack.len() - arg_count - 1,
            upvalues,
        });
    }

    fn pop_many(&mut self, count: usize) {
        for _ in 0..count {
            self.stack.pop();
        }
    }

    fn pop(&mut self) -> Value {
        self.stack.pop().unwrap()
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    pub fn alloc<T: GcTrace + 'static>(&mut self, obj: T) -> GcRef<T> {
        self.gc.alloc(obj)
    }

    pub fn deref<T: GcTrace + 'static>(&self, r: GcRef<T>) -> &T {
        self.gc.deref(r)
    }

    pub fn deref_mut<T: GcTrace + 'static>(&mut self, r: GcRef<T>) -> &mut T {
        self.gc.deref_mut(r)
    }

}

#[derive(Clone)]
pub enum UpvalueRegistry {
    Open(usize),
    Closed(Value),
}
impl GcTrace for UpvalueRegistry {
    fn size(&self) -> usize {
        std::mem::size_of::<Self>()
    }

    fn trace(&self, _: &mut Gc) {}

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl UpvalueRegistry {
    fn close(&mut self, value: Value) {
        *self = UpvalueRegistry::Closed(value);
    }
}

impl fmt::Debug for UpvalueRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UpvalueRegistry::Open(index) => write!(f, "UpvalueRef::Open({})", index),
            UpvalueRegistry::Closed(value) => write!(f, "UpvalueRef::Closed({:?})", value),
        }
    }
}

impl PartialEq for UpvalueRegistry {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (UpvalueRegistry::Open(index1), UpvalueRegistry::Open(index2)) => index1 == index2,
            (UpvalueRegistry::Closed(value1), UpvalueRegistry::Closed(value2)) => value1 == value2,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FunctionUpvalue {
    pub index: usize,
    pub is_local: bool,
}