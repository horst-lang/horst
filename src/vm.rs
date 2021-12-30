use crate::compiler::Program;
use crate::frame::CallFrame;
use crate::function::Function;
use crate::instruction::Instruction;
use crate::value::Value;

pub struct VM {
    call_stack: Vec<CallFrame>,
    stack: Vec<Value>,
    globals: Vec<Option<Value>>,
    constants: Vec<Value>,
}

impl VM {
    pub fn new(program: Program) -> VM {
        let global_frame = CallFrame {
            function: Function {
                instructions: program.instructions,
                arity: 0,
            },
            ip: 0,
            base_pointer: 0,
        };

        let mut vm = VM {
            call_stack: vec![global_frame],
            stack: vec![],
            globals: vec![],
            constants: program.constants,
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
                        self.push(Value::String(a + &b.to_string()));
                    } else if let Value::String(b) = b {
                        self.push(Value::String(a.to_string() + &b));
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
                Instruction::Call(arg_count) => {
                    let function = self.peek(arg_count + 1);

                    if let Value::Function(function) = function {
                        let base_pointer = self.stack.len() - arg_count;

                        assert_eq!(arg_count, function.arity, "Invalid arity for function call. Expected {}, got {}.", function.arity, arg_count);

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
                        let result = (function.function)(args);
                        self.push(result);
                    } else {
                        panic!("Cannot call non-function. Got {}.", function);
                    }
                },
                Instruction::Return => {
                    let return_value = self.pop();
                    let call_frame = self.call_stack.pop().unwrap();
                    self.stack.truncate(call_frame.base_pointer);

                    if !self.call_stack.is_empty() {
                        let function = self.pop();
                        assert_eq!(function, Value::Function(call_frame.function), "Return value does not match function.");
                        self.push(return_value);
                    } else {
                        return return_value;
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

                    println!("{}", value);
                },
                Instruction::Halt => {
                    return Value::Nil;
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
}