use std::collections::HashMap;
use crate::class::Class;
use crate::compiler::Program;
use crate::frame::CallFrame;
use crate::function::Function;
use crate::instance::Instance;
use crate::instruction::Instruction;
use crate::value::Value;
use core::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

type Heap = HashMap<usize, Box<dyn Collectable>>;

pub struct VM {
    pub(crate) call_stack: Vec<CallFrame>,
    pub(crate) stack: Vec<Value>,
    globals: Vec<Option<Value>>,
    constants: Vec<Value>,
    pub(crate) heap: Heap,
    next_id: usize,
}

pub trait Collectable: Any {
    fn collect(&self) -> Vec<usize>;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn to_string(&self, _: &VM) -> Option<String> {
        None
    }
}

impl Collectable for Instance {
    fn collect(&self) -> Vec<usize> {
        let mut ids = vec![];

        for (_, value) in &self.fields {
            if let Value::Instance(id) = value {
                ids.push(*id);
            }
        }

        ids
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl VM {
    pub fn new(program: Program) -> VM {
        let global_frame = CallFrame {
            function: Function {
                instructions: program.instructions,
                arity: 0,
                upvalues: HashMap::new(),
            },
            ip: 0,
            base_pointer: 0,
        };

        let mut vm = VM {
            call_stack: vec![global_frame],
            stack: vec![],
            globals: vec![],
            constants: program.constants,
            heap: HashMap::new(),
            next_id: 0,
        };

        vm.globals.resize(program.global_count, None);

        vm
    }

    pub fn run(&mut self) -> Value {
        macro_rules! binary_op {
            ($op:tt, $type:tt) => {
                let b = self.pop();
                let a = self.pop();

                if let (Value::Number(a), Value::Number(b)) = (a, b) {
                    self.push(Value::$type(a $op b ));
                } else {
                    panic!("Invalid operands for binary operation.");
                }
            };
        }

        loop {
            let frame = self.call_stack.last_mut().unwrap();

            let instruction = frame.function.instructions[frame.ip];

            frame.ip += 1;

            match instruction {
                Instruction::Constant(index) => {
                    self.push(self.constants[index].clone());
                }
                Instruction::Negate => {
                    let value = self.pop();

                    if let Value::Number(value) = value {
                        self.push(Value::Number(-value));
                    } else {
                        panic!("Invalid operand for negation.");
                    }
                }
                Instruction::Add => {
                    let b = self.pop();
                    let a = self.pop();

                    if let (Value::Number(a), Value::Number(b)) = (a.clone(), b.clone()) {
                        self.push(Value::Number(a + b));
                    } else if let Value::String(a) = a {
                        self.push(Value::String(a + &b.to_string(self)));
                    } else if let Value::String(b) = b {
                        self.push(Value::String(a.to_string(self) + &b));
                    } else {
                        panic!("Invalid operands for addition.");
                    }
                }
                Instruction::Subtract => {
                    binary_op!(-, Number);
                }
                Instruction::Multiply => {
                    binary_op!(*, Number);
                }
                Instruction::Divide => {
                    binary_op!(/, Number);
                }
                Instruction::Not => {
                    let value = self.pop();

                    if let Value::Boolean(value) = value {
                        self.push(Value::Boolean(!value));
                    } else {
                        panic!("Invalid operand for not operation.");
                    }
                }
                Instruction::Equal => {
                    let b = self.pop();
                    let a = self.pop();

                    self.push(Value::Boolean(a == b));
                }
                Instruction::NotEqual => {
                    let b = self.pop();
                    let a = self.pop();

                    self.push(Value::Boolean(a != b));
                }
                Instruction::Greater => {
                    binary_op!(>, Boolean);
                },
                Instruction::GreaterEqual => {
                    binary_op!(>=, Boolean);
                },
                Instruction::Less => {
                    binary_op!(<, Boolean);
                },
                Instruction::LessEqual => {
                    binary_op!(<=, Boolean);
                },
                Instruction::Jump(offset) => {
                    frame.ip += offset - 1;
                }
                Instruction::JumpIfFalse(offset) => {
                    let value = self.stack.pop().unwrap();

                    if value.is_falsey() {
                        frame.ip += offset - 1;
                    }
                }
                Instruction::JumpBack(offset) => {
                    frame.ip -= offset + 1;
                }
                Instruction::Pop => {
                    self.pop();
                },
                Instruction::GetGlobal(index) => {
                    let value = self.globals[index].clone();

                    if let Some(value) = value {
                        self.push(value);
                    } else {
                        panic!("Undefined variable.");
                    }
                },
                Instruction::SetGlobal(index) => {
                    let value = self.peek(1);

                    if self.globals[index].is_none() {
                        panic!("Undefined variable.");
                    } else {
                        self.globals[index] = Some(value);
                    }
                },
                Instruction::DefineGlobal(index) => {
                    let value = self.pop();

                    self.globals[index] = Some(value);
                },
                Instruction::GetLocal(index) => {
                    let value = self.stack[frame.base_pointer + index].clone();

                    self.push(value);
                },
                Instruction::SetLocal(index) => {
                    let value = self.stack.last().unwrap().clone();

                    self.stack[frame.base_pointer + index] = value;
                },
                Instruction::GetProperty(index) => {
                    let name = self.constants[index].clone();
                    let instance = self.pop();

                    if let (Value::Instance(instance), Value::String(name)) = (instance.clone(), name) {
                        let instance = self.get_instance(instance).unwrap();
                        if let Some(value) = instance.fields.get(&name) {
                            self.push(value.clone());
                        } else {
                            let method = self.get_method(instance, name);
                            self.push(method);
                        }
                    } else {
                        panic!("Cannot get property of non-object.");
                    }
                },
                Instruction::SetProperty(index) => {
                    let name = self.constants[index].clone();
                    let value = self.pop();
                    let instance = self.peek(1);

                    if let (Value::Instance(instance), Value::String(name)) = (instance, name) {
                        let instance = self.get_instance_mut(instance).unwrap();
                        instance.fields.insert(name, value);
                    } else {
                        panic!("Cannot set property of non-object.");
                    }
                },
                Instruction::MakeUpvalue(upvalue_index, local_index) => {
                    let value = self.stack[frame.base_pointer + local_index].clone();
                    frame.function.upvalues.insert(upvalue_index, self.next_id);
                    self.new_collectable(value);
                },
                Instruction::GetUpvalue(upvalue_index) => {
                    let closure = frame.function.clone();
                    let upvalue = closure.upvalues[&upvalue_index].clone();
                    let value = self.get_collectable::<Value>(upvalue).unwrap();
                    self.push(value.clone());
                },
                Instruction::SetUpvalue(upvalue_index) => {
                    let value = self.stack.last().unwrap().clone();
                    let closure = frame.function.clone();
                    let upvalue = closure.upvalues[&upvalue_index].clone();
                    self.set_collectable(upvalue, value);
                },
                Instruction::MakeClosure => {
                    let function = self.pop();
                    let frame_closure = self.call_stack.last().unwrap().function.clone();
                    if let Value::Function(mut function) = function {
                        for (index, upvalue) in frame_closure.upvalues.iter() {
                            function.upvalues.insert(*index,*upvalue);
                        }

                        self.push(Value::Function(function));
                    } else {
                        panic!("Cannot make closure of non-function.");
                    }
                },
                Instruction::Call(arg_count) => {
                    let function = self.peek(arg_count + 1);
                    if let Value::Function(function) = function {
                        if function.arity != arg_count {
                            panic!("Expected {} arguments but got {}.", function.arity, arg_count);
                        }

                        let base_pointer = self.stack.len() - arg_count;
                        self.call_stack.push(CallFrame {
                            function,
                            base_pointer,
                            ip: 0,
                        });
                    } else if let Value::Native(function) = function {
                        let mut args = Vec::with_capacity(arg_count);
                        for _ in 0..arg_count {
                            args.push(self.pop());
                        }
                        self.pop();
                        let result = (function.function)(args.into_iter().rev().collect(), self);
                        self.push(result);
                    } else if let Value::Class(class) = function {
                        let value = self.new_instance(Instance::new(class.clone()));
                        let l = self.stack.len();
                        self.stack[l - arg_count - 1] = value.clone();
                        let init = class.methods.get("init");
                        if let Some(Value::Function(init)) = init {
                            let args = self.stack.split_off(l - arg_count);
                            self.push(value.clone());
                            self.stack.extend(args);
                            self.call_stack.push(CallFrame {
                                function: init.clone(),
                                base_pointer: self.stack.len() - arg_count - 1,
                                ip: 0,
                            });
                        } else if let Some(Value::Native(init)) = init {
                            let mut args = Vec::with_capacity(arg_count + 1);
                            for _ in 0..arg_count {
                                args.push(self.pop());
                            }
                            args.push(value.clone());
                            self.pop();
                            (init.function)(args.into_iter().rev().collect(), self);
                            self.push(value);
                        } else if arg_count != 0 {
                            panic!("Expected 0 arguments, got {}.", arg_count);
                        }
                    } else {
                        panic!("Cannot call non-function.");
                    }
                },
                Instruction::Return => {
                    let return_value = self.pop();
                    let call_frame = self.call_stack.pop().unwrap();
                    self.stack.truncate(call_frame.base_pointer);
                    if !self.call_stack.is_empty() {
                        let function = self.pop();
                        if let Value::Function(function) = function {
                            self.push(return_value);
                        } else if let Value::Instance(instance) = function {
                            self.push(Value::Instance(instance));
                        } else {
                            panic!("Cannot return from non-function.");
                        }

                    } else {
                        return return_value;
                    }
                },
                Instruction::Invoke(arg_count) => {
                    let name = self.pop();
                    let instance = self.pop();

                    if let (Value::String(name), Value::Instance(i)) = (name, instance) {
                        let instance = self.get_instance(i).unwrap();
                        let method = if let Value::Function(method) = self.get_method(instance, name) {
                            method
                        } else {
                            panic!("Undefined method.");
                        };
                        self.push(Value::Function(method.clone()));
                        self.push(Value::Instance(i));
                        self.call_stack.push(CallFrame {
                            function: method,
                            base_pointer: self.stack.len() - arg_count - 1,
                            ip: 0,
                        });
                    } else {
                        panic!("Cannot invoke non-method.");
                    }
                },
                Instruction::GetSuper(index) => {
                    let name = self.constants[index].clone();
                    let superclass = self.pop();

                    if let (Value::String(name), Value::Class(superclass)) = (name, superclass) {
                        let method = if let Some(Value::Function(method)) = superclass.methods.get(&name) {
                            method.clone()
                        } else {
                            panic!("Undefined method '{}'.", name);
                        };
                        self.push(Value::Function(method.clone()));
                    } else {
                        panic!("Cannot get super of non-class.");
                    }
                },
                Instruction::False => {
                    self.push(Value::Boolean(false));
                },
                Instruction::True => {
                    self.push(Value::Boolean(true));
                },
                Instruction::Nil => {
                    self.push(Value::Nil);
                },
                Instruction::And => {
                    let right = self.pop();
                    let left = self.pop();
                    if left.is_falsey() {
                        self.push(left);
                    } else {
                        self.push(right);
                    }
                },
                Instruction::Or => {
                    let right = self.pop();
                    let left = self.pop();

                    if left.is_truthy() {
                        self.push(left);
                    } else {
                        self.push(right);
                    }
                },
                Instruction::Print => {
                    let value = self.pop();
                    println!("{}", value.to_string(self));
                },
                Instruction::Halt => {
                    return Value::Nil;
                },
                Instruction::Inherit => {
                    let subclass = self.pop();
                    let superclass = self.pop();

                    if let (Value::Class(superclass), Value::Class(mut subclass)) = (superclass, subclass) {
                        for method in superclass.methods {
                            subclass.methods.insert(method.0, method.1);
                        }
                        self.push(Value::Class(subclass));
                    } else {
                        panic!("Cannot inherit from non-class.");
                    }
                },
            }
        }

    }

    fn peek(&self, distance: usize) -> Value {
        self.stack[self.stack.len() - distance].clone()
    }

    fn pop(&mut self) -> Value {
        self.stack.pop().unwrap()
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    fn get_method(&self, instance: &Instance, name: String) -> Value {
        if let Some(value) = instance.class.methods.get(&name) {
            return value.clone();
        } else {
            panic!("Undefined property {}.", name);
        }
    }


    pub fn mark_and_sweep(&mut self) {
        // Step 1: Mark
        let mut marked = vec![];
        for value in self.stack.clone() {
            if let Value::Instance(id) = value {
                marked.push(id);
            }
        }
        let mut swap = marked.clone();
        let mut current = vec![];
        loop {
            for a in &swap {
                if let Some(instance) = self.heap.get(a) {
                    match self.get_instance(*a) {
                        Some(instance) => {
                            for (_, value) in &instance.fields {
                                if let Value::Instance(id) = value {
                                    if !marked.contains(id) {
                                        current.push(*id);
                                    }
                                }
                            }
                        },
                        None => (),
                    }
                }
            }
            if !current.is_empty() {
                marked.append(&mut current);
                swap = current.clone();
                current.clear();
            } else {
                break;
            }
        }


        // Step 2: Sweep
        self.heap.retain(|id, _| marked.contains(id));
    }

    pub fn new_instance(&mut self, instance: Instance) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.heap.insert(id, Box::new(instance));
        Value::Instance(id)
    }

    pub fn get_instance(&self, id: usize) -> Option<&Instance> {
        match self.heap.get(&id) {
            Some(collectable) => collectable.as_any().downcast_ref::<Instance>(),
            None => None,
        }
    }

    pub(crate) fn get_instance_mut(&mut self, id: usize) -> Option<&mut Instance> {
        match self.heap.get_mut(&id) {
            Some(collectable) => collectable.as_any_mut().downcast_mut::<Instance>(),
            None => None,
        }
    }

    pub fn get_collectable<T: Collectable>(&self, id: usize) -> Option<&T> {
        match self.heap.get(&id) {
            Some(collectable) => collectable.as_any().downcast_ref::<T>(),
            None => None,
        }
    }

    pub fn get_collectable_mut<T: Collectable>(&mut self, id: usize) -> Option<&mut T> {
        match self.heap.get_mut(&id) {
            Some(collectable) => collectable.as_any_mut().downcast_mut::<T>(),
            None => None,
        }
    }

    pub fn set_collectable<T: Collectable>(&mut self, id: usize, collectable: T) {
        self.heap.insert(id, Box::new(collectable));
    }

    pub fn new_collectable<T: Collectable>(&mut self, collectable: T) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.heap.insert(id, Box::new(collectable));
        id
    }
}