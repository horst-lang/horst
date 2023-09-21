use std::collections::HashMap;
use crate::class::Class;
use crate::function::Function;
use crate::instruction::Instruction;
use crate::native_functions::{NATIVE_FUNCTIONS, NATIVE_CLASSES};
use crate::token::Token;
use crate::value::Value;

#[derive(PartialEq)]
enum FunctionKind {
    Function,
    Method,
    Anonymous,
}

#[derive(Debug, PartialEq)]
pub struct Program {
    pub instructions: Vec<Instruction>,
    pub constants: Vec<Value>,
    pub global_count: usize,
}

pub struct Compiler {
    tokens: Vec<Token>,
    current: usize,
    constants: Vec<Value>,
    globals: HashMap<String, usize>,
    scopes: Vec<Scope>,
    in_function: bool,
    current_super: Option<Instruction>,
    upvalue_count: usize,
}

#[derive(Debug, Clone)]
struct Scope {
    locals: HashMap<String, usize>,
    upvalues: HashMap<String, Upvalue>,
}

impl Scope {
    pub fn new() -> Scope {
        Self {
            locals: HashMap::new(),
            upvalues: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct Upvalue {
    is_local: bool,
    local_index: usize,
    upvalue_index: usize,
}

impl Compiler {
    pub fn new(tokens: Vec<Token>) -> Compiler {
        Compiler {
            tokens,
            current: 0,
            constants: vec![],
            globals: HashMap::new(),
            scopes: vec![Scope::new()],
            in_function: false,
            current_super: None,
            upvalue_count: 0,
        }
    }

    pub fn compile(&mut self) -> Program {
        let mut instructions = vec![];

        while !self.is_at_end() {
            instructions.extend(self.declaration());
        }

        instructions.push(Instruction::Halt);

        Program {
            instructions,
            constants: self.constants.clone(),
            global_count: self.global_count(),
        }
    }

    fn declaration(&mut self) -> Vec<Instruction> {
        if self.match_token(Token::Let) {
            self.let_declaration()
        } else if self.match_token(Token::Fn) {
            self.function_declaration()
        } else if self.match_token(Token::Class) {
            self.class_declaration()
        } else {
            self.statement()
        }
    }

    fn let_declaration(&mut self) -> Vec<Instruction> {
        let name = self.consume_identifier("Expect variable name.");
        let global = self.scopes.len() == 1;

        let initializer = if self.match_token(Token::Equal) {
            self.expression()
        } else {
            vec![Instruction::Nil]
        };

        self.match_token(Token::Semicolon);

        if global {
            self.define_global(name, initializer)
        } else {
            self.define_local(name, initializer)
        }
    }

    fn define_global(&mut self, name: String, mut initializer: Vec<Instruction>) -> Vec<Instruction> {
        let mut index = self.global_count();
        if self.globals.contains_key(&name) {
            index = self.globals[&name];
        }
        self.globals.insert(name, index);

        initializer.push(Instruction::DefineGlobal(index));
        initializer
    }

    fn define_local(&mut self, name: String, initializer: Vec<Instruction>) -> Vec<Instruction> {
        assert!(!self.current_scope().locals.contains_key(&name), "Variable with this name already defined in the same scope: {}", name);

        let index = self.local_count();
        self.current_scope_mut().locals.insert(name, index);
        initializer
    }

    fn function_declaration(&mut self) -> Vec<Instruction> {
        let name = self.consume_identifier("Expect function name.");
        let function = self.function(FunctionKind::Function);
        self.define_global(name, function)
    }

    fn function(&mut self, kind: FunctionKind) -> Vec<Instruction> {
        self.in_function = true;
        self.begin_scope();
        self.consume_token(Token::LeftParen, "Expect '(' after function name.");

        let mut parameters = vec![];
        if kind == FunctionKind::Method {
            self.define_local("this".to_string(), vec![]);
        }

        if !self.check(&Token::RightParen) {
            loop {
                let param = self.consume_identifier("Expect parameter name.");

                assert!(!self.current_scope().locals.contains_key(&param), "Cannot have two parameters with the same name: {}", param);

                self.define_local(param.clone(), vec![]);

                parameters.push(param);

                if !self.match_token(Token::Comma) {
                    break;
                }
            }
        }

        self.consume_token(Token::RightParen, "Expect ')' after parameters.");

        let mut body = self.block();

        let mut upvalues = vec![];
        for (key, upvalue) in self.current_scope().upvalues.iter() {
            if !upvalue.is_local && self.scopes[self.scopes.len() - 2].upvalues[key].is_local {
                upvalues.push(Instruction::MakeUpvalue(upvalue.upvalue_index, upvalue.local_index));
            }
        }

        body.extend(self.end_scope());
        if body.last() != Some(&Instruction::Return) {
            body.push(Instruction::Nil);
            body.push(Instruction::Return);
        }

        let index = self.add_constant(Value::Function(Function::new(
            body,
            parameters.len(),
        )));

        self.in_function = false;

        upvalues.extend(vec![Instruction::Constant(index), Instruction::MakeClosure]);
        upvalues
    }

    fn begin_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    fn end_scope(&mut self) -> Vec<Instruction> {
        let scope = self.scopes.pop().unwrap();
        scope
            .locals
            .into_iter()
            .map(|_| Instruction::Pop)
            .collect()
    }

    fn current_scope(&self) -> &Scope {
        self.scopes.last().unwrap()
    }

    fn current_scope_mut(&mut self) -> &mut Scope {
        self.scopes.last_mut().unwrap()
    }

    fn local_count(&self) -> usize {
        self.scopes.last().unwrap().locals.len()
    }

    fn global_count(&self) -> usize {
        self.globals.len()
    }

    fn add_constant(&mut self, value: Value) -> usize {
        for (index, constant) in self.constants.iter().enumerate() {
            if *constant == value {
                return index;
            }
        }

        self.constants.push(value);
        self.constants.len() - 1
    }

    fn class_declaration(&mut self) -> Vec<Instruction> {
        let name = self.consume_identifier("Expect class name.");
        let superclass = if self.match_token(Token::Less) {
            let name = self.consume_identifier("Expect superclass name.");
            Some(self.get_variable(&name))
        } else {
            None
        };
        self.current_super = superclass.clone();

        self.consume_token(Token::LeftBrace, "Expect '{' before class body.");

        let mut methods = HashMap::new();

        while !self.check(&Token::RightBrace) && !self.is_at_end() {
            let name = self.consume_identifier("Expect function name.");
            let function = self.function(FunctionKind::Method);
            let function = self.remove_constant(function[0]);
            if let Value::Function(function) = function {
                methods.insert(name, function);
            } else {
                panic!("Expected function.");
            }
        }
        self.current_super = None;

        self.consume_token(Token::RightBrace, "Expect '}' after class body.");

        let index = self.add_constant(Value::Class(Class::new(name.clone(), methods)));

        let mut instructions = vec![Instruction::Constant(index)];
        if let Some(superclass) = superclass {
            instructions.push(superclass);
            instructions.push(Instruction::Inherit);
        }

        self.define_global(name, instructions)
    }

    fn remove_constant(&mut self, instruction: Instruction) -> Value {
        match instruction {
            Instruction::Constant(index) => self.constants.remove(index),
            _ => panic!("Expected constant instruction."),
        }
    }

    fn statement(&mut self) -> Vec<Instruction> {
        if self.match_token(Token::Print) {
            self.print_statement()
        } else if self.check(&Token::LeftBrace) {
            self.block_statement()
        } else if self.match_token(Token::If) {
            self.if_statement()
        } else if self.match_token(Token::While) {
            self.while_statement()
        } else if self.match_token(Token::Return) {
            self.return_statement()
        } else {
            self.expression_statement()
        }
    }

    fn block_statement(&mut self) -> Vec<Instruction> {
        self.begin_scope();
        let mut instructions = self.block();
        instructions.extend(self.end_scope());

        instructions
    }

    fn if_statement(&mut self) -> Vec<Instruction> {
        let mut instructions = vec![];

        self.consume_token(Token::LeftParen, "Expect '(' after 'if'.");
        instructions.extend(self.expression());
        self.consume_token(Token::RightParen, "Expect ')' after condition.");

        let then_instructions = self.block_statement();
        let mut else_instructions = vec![];

        if self.match_token(Token::Else) {
            else_instructions = self.block_statement();
        }

        instructions.push(Instruction::JumpIfFalse(then_instructions.len() + 2));
        instructions.extend(then_instructions);
        instructions.push(Instruction::Jump(else_instructions.len() + 1));
        instructions.extend(else_instructions);

        instructions
    }

    fn return_statement(&mut self) -> Vec<Instruction> {
        let mut instructions = vec![];

        if self.check(&Token::Semicolon) {
            instructions.push(Instruction::Nil);
        } else {
            instructions.extend(self.expression());
        }

        self.match_token(Token::Semicolon);

        instructions.push(Instruction::Return);

        instructions
    }

    fn while_statement(&mut self) -> Vec<Instruction> {
        let mut instructions = vec![];

        self.consume_token(Token::LeftParen, "Expect '(' after 'while'.");
        let condition = self.expression();
        let condition_length = condition.len();
        instructions.extend(condition);
        self.consume_token(Token::RightParen, "Expect ')' after condition.");

        let body = self.block_statement();
        let body_length = body.len();

        // Example Instructions:
        // [ True, JumpIfFalse(3), Constant(0), Print, Pop, JumpBack(4)

        instructions.push(Instruction::JumpIfFalse(body_length + 2));
        instructions.extend(body);
        instructions.push(Instruction::JumpBack(body_length + condition_length + 1));

        instructions
    }

    fn print_statement(&mut self) -> Vec<Instruction> {
        let mut instructions = self.expression();
        instructions.push(Instruction::Print);

        self.match_token(Token::Semicolon);

        instructions
    }

    fn expression_statement(&mut self) -> Vec<Instruction> {
        let mut instructions = self.expression();

        self.match_token(Token::Semicolon);

        instructions.push(Instruction::Pop);

        instructions
    }

    fn expression(&mut self) -> Vec<Instruction> {
        let mut instructions = self.assignment();

        while self.match_token(Token::Equal) {
            instructions.extend(self.assignment());
        }

        instructions
    }

    fn assignment(&mut self) -> Vec<Instruction> {
        let mut instructions = self.or();

        if self.match_token(Token::Equal) {
            let value = self.assignment();

            if let Some(name) = instructions.pop() {
                if let Instruction::GetGlobal(index) = name {
                    instructions.extend(value);
                    instructions.push(Instruction::SetGlobal(index));
                } else if let Instruction::GetLocal(index) = name {
                    instructions.extend(value);
                    instructions.push(Instruction::SetLocal(index));
                } else if let Instruction::GetProperty(index) = name {
                    instructions.extend(value);
                    instructions.push(Instruction::SetProperty(index));
                } else if let Instruction::GetUpvalue(index) = name {
                    instructions.extend(value);
                    instructions.push(Instruction::SetUpvalue(index));
                } else {
                    panic!("Invalid assignment target.");
                }
            } else {
                panic!("Invalid assignment target.");
            }

        }

        instructions
    }

    fn or(&mut self) -> Vec<Instruction> {
        let mut instructions = self.and();

        while self.match_token(Token::Or) {
            instructions.extend(self.and());
            instructions.push(Instruction::Or);
        }

        instructions
    }

    fn and(&mut self) -> Vec<Instruction> {
        let mut instructions = self.equality();

        while self.match_token(Token::And) {
            instructions.extend(self.equality());
            instructions.push(Instruction::And);
        }

        instructions
    }

    fn equality(&mut self) -> Vec<Instruction> {
        let mut instructions = self.comparison();

        while self.match_token(Token::BangEqual) {
            instructions.extend(self.comparison());
            instructions.push(Instruction::NotEqual);
        }

        while self.match_token(Token::EqualEqual) {
            instructions.extend(self.comparison());
            instructions.push(Instruction::Equal);
        }

        instructions
    }

    fn comparison(&mut self) -> Vec<Instruction> {
        let mut instructions = self.addition();

        while self.match_token(Token::Greater) {
            instructions.extend(self.addition());
            instructions.push(Instruction::Greater);
        }

        while self.match_token(Token::GreaterEqual) {
            instructions.extend(self.addition());
            instructions.push(Instruction::GreaterEqual);
        }

        while self.match_token(Token::Less) {
            instructions.extend(self.addition());
            instructions.push(Instruction::Less);
        }

        while self.match_token(Token::LessEqual) {
            instructions.extend(self.addition());
            instructions.push(Instruction::LessEqual);
        }

        instructions
    }

    fn addition(&mut self) -> Vec<Instruction> {
        let mut instructions = self.multiplication();

        while self.match_token(Token::Minus) {
            instructions.extend(self.multiplication());
            instructions.push(Instruction::Subtract);
        }

        while self.match_token(Token::Plus) {
            instructions.extend(self.multiplication());
            instructions.push(Instruction::Add);
        }

        instructions
    }

    fn multiplication(&mut self) -> Vec<Instruction> {
        let mut instructions = self.unary();

        while self.match_token(Token::Slash) {
            instructions.extend(self.unary());
            instructions.push(Instruction::Divide);
        }

        while self.match_token(Token::Star) {
            instructions.extend(self.unary());
            instructions.push(Instruction::Multiply);
        }

        instructions
    }

    fn unary(&mut self) -> Vec<Instruction> {
        let mut instructions = vec![];

        if self.match_token(Token::Bang) {
            instructions.extend(self.unary());
            instructions.push(Instruction::Not);
        } else if self.match_token(Token::Minus) {
            instructions.extend(self.unary());
            instructions.push(Instruction::Negate);
        } else {
            instructions.extend(self.call());
        }

        instructions
    }

    fn call(&mut self) -> Vec<Instruction> {
        let mut instructions = vec![];

        instructions.extend(self.primary());

        loop {
            match self.peek().clone() {
                Token::LeftParen => instructions.extend(self.finish_call()),
                Token::Dot => instructions.extend(self.finish_get(instructions.clone())),
                _ => break,
            }
        }

        instructions
    }

    fn finish_call(&mut self) -> Vec<Instruction> {
        let mut instructions = vec![];

        self.consume_token(Token::LeftParen, "Expect '(' after function name.");

        let mut arguments: usize = 0;

        while !self.match_token(Token::RightParen) {
            if arguments > 0 {
                self.consume_token(Token::Comma, "Expect ',' after function argument.");
            }

            instructions.extend(self.expression());
            arguments += 1;
        }

        instructions.push(Instruction::Call(arguments));

        instructions
    }

    fn finish_get(&mut self, vec1: Vec<Instruction>) -> Vec<Instruction> {
        let mut instructions = vec![];

        self.consume_token(Token::Dot, "Expect '.' after object.");
        let name = self.consume_identifier("Expect property name after '.'.");
        let index = self.add_constant(Value::String(name.clone()));
        if self.peek() == &Token::LeftParen {
            let mut call = self.finish_call();
            if let Some(Instruction::Call(arguments)) = call.pop() {
                instructions.push(Instruction::GetProperty(index));
                instructions.extend(vec1);
                instructions.extend(call);
                instructions.push(Instruction::Call(arguments + 1));
            }
        } else {
            instructions.push(Instruction::GetProperty(index));
        }

        instructions
    }

    fn primary(&mut self) -> Vec<Instruction> {
        let mut instructions = vec![];

        match self.peek().clone() {
            Token::False => {
                instructions.push(Instruction::False);
                self.advance();
            },
            Token::True => {
                instructions.push(Instruction::True);
                self.advance();
            },
            Token::Nil => {
                instructions.push(Instruction::Nil);
                self.advance();
            },
            Token::Number(value) => {
                let index = self.add_constant(Value::Number(value));
                instructions.push(Instruction::Constant(index));
                self.advance();
            },
            Token::String(s) => {
                let index = self.add_constant(Value::String(s));
                instructions.push(Instruction::Constant(index));
                self.advance();
            },
            Token::Identifier(name) => {
                instructions.push(self.get_variable(&name));
                self.advance();
            },
            Token::LeftParen => {
                self.advance();
                instructions.extend(self.expression());
                self.consume_token(Token::RightParen, "Expect ')' after expression.");
            },
            Token::Fn => {
                self.advance();
                instructions.extend(self.function(FunctionKind::Anonymous));
            },
            Token::Super => {
                if let Some(superclass) = self.current_super {
                    self.advance();
                    self.consume_token(Token::Dot, "Expect '.' after 'super'.");
                    let method = self.consume_identifier("Expect superclass method name.");
                    let index = self.add_constant(Value::String(method));
                    let mut call = self.finish_call();
                    if let Some(Instruction::Call(arguments)) = call.pop() {
                        instructions.push(superclass);
                        instructions.push(Instruction::GetSuper(index));
                        instructions.push(Instruction::GetLocal(0));
                        instructions.extend(call);
                        instructions.push(Instruction::Call(arguments + 1));
                    }

                } else {
                    panic!("No superclass defined.");
                }
            },
            Token::This => {
                instructions.push(Instruction::GetLocal(0));
                self.advance();
            },
            _ => {
                panic!("Expected expression, got {:?}", self.peek());
            }
        }

        instructions
    }

    fn get_native(&mut self, name: &str) -> Instruction {
        let index = self.add_constant(Value::Native(NATIVE_FUNCTIONS[name].clone()));
        Instruction::Constant(index)
    }

    fn get_native_class(&mut self, name: &str) -> Instruction {
        let index = self.add_constant(Value::Class(NATIVE_CLASSES[name].clone()));
        Instruction::Constant(index)
    }

    fn get_variable(&mut self, name: &str) -> Instruction {

        if NATIVE_FUNCTIONS.contains_key(name) {
            self.get_native(&name)
        } else if NATIVE_CLASSES.contains_key(name) {
            self.get_native_class(&name)
        } else {
            let local_index = self.get_local_index(name);
            let upvalue_index = self.get_upvalue_index(name);
            return if let Some(upvalue_index) = upvalue_index {
                Instruction::GetUpvalue(upvalue_index)
            } else if let Some(local_index) = local_index {
                Instruction::GetLocal(local_index)
            } else {
                if !self.globals.contains_key(name) {
                    self.globals.insert(name.to_string(), self.global_count());
                }

                let index = self.globals.get(name).unwrap();
                Instruction::GetGlobal(*index)
            }
        }
    }

    fn set_variable<S: ToString>(&mut self, name: S) -> Instruction {
        let local_index = self.get_local_index(&name.to_string());
        let upvalue_index = self.get_upvalue_index(&name.to_string());
        return if let Some(upvalue_index) = upvalue_index {
            Instruction::SetUpvalue(upvalue_index)
        } else if let Some(local_index) = local_index {
            Instruction::SetLocal(local_index)
        } else {
            let global_index = self.global_count();
            self.globals.entry(name.to_string()).or_insert_with(|| global_index);

            let index = self.globals.get(&name.to_string()).unwrap();
            Instruction::SetGlobal(*index)
        }
    }

    fn get_local_index(&mut self, name: &str) -> Option<usize> {
        if self.in_function {
            return self.scopes.last().unwrap().locals.get(name).copied()
        }
        for scope in &self.scopes {
            if scope.locals.contains_key(name) {
                return Some(scope.locals[name]);
            }
        }
        None
    }

    fn get_upvalue_index(&mut self, name: &str) -> Option<usize> {
        for (i, scope) in self.scopes.iter_mut().enumerate().rev().skip(1) {
            if scope.upvalues.contains_key(name) {
                return Some(scope.upvalues[name].upvalue_index);
            }
        }

        for (i, scope) in self.scopes.iter_mut().enumerate().rev().skip(1) {
            if scope.locals.contains_key(name) {
                let local_index = scope.locals[name];
                scope.upvalues.insert(name.to_string(), Upvalue {
                    is_local: true,
                    local_index: scope.locals[name],
                    upvalue_index: self.upvalue_count,
                });
                self.upvalue_count += 1;
                for scope in &mut self.scopes[i + 1..] {
                    scope.upvalues.insert(name.to_string(), Upvalue {
                        is_local: false,
                        local_index,
                        upvalue_index: self.upvalue_count - 1,
                    });
                }
                return Some(self.upvalue_count - 1);
            }
        }
        None
    }

    fn is_at_end(&self) -> bool {
        self.peek() == &Token::Eof
    }

    fn advance(&mut self) {
        self.current += 1;
    }

    fn check(&self, token: &Token) -> bool {
        self.peek() == token
    }

    fn match_token(&mut self, token: Token) -> bool {
        if self.check(&token) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn peek_next(&self) -> &Token {
        &self.tokens[self.current + 1]
    }

    fn consume_token(&mut self, token: Token, message: &str) {
        if self.check(&token) {
            self.advance();
        } else {
            panic!("{} Expected {:?}. Got {:?}", message, token, self.peek());
        }
    }

    fn block(&mut self) -> Vec<Instruction> {
        let mut instructions = vec![];

        self.consume_token(Token::LeftBrace, "Expect '{' before block.");

        while !self.check(&Token::RightBrace) && !self.is_at_end() {
            instructions.extend(self.declaration());
        }

        self.consume_token(Token::RightBrace, "Expect '}' after block.");

        instructions
    }

    fn consume_identifier(&mut self, message: &str) -> String {
        let identifier = self.peek();

        if let Token::Identifier(name) = identifier.clone() {
            self.advance();
            name
        } else {
            panic!("{} Expected identifier.", message);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::scanner::Scanner;
    use super::*;

    fn compile(source: &str) -> Program {
        let mut scanner = Scanner::new(source);
        scanner.scan_tokens();
        let mut compiler = Compiler::new(scanner.tokens);
        compiler.compile()
    }

    #[test]
    fn test_let_statements() {
        let program = compile("let x = 5;");
        assert_eq!(program, Program {
            instructions: vec![
                Instruction::Constant(0),
                Instruction::DefineGlobal(0),
                Instruction::Halt,
            ],
            constants: vec![
                Value::Number(5.0),
            ],
            global_count: 1
        });
    }

    #[test]
    fn test_let_statements_2() {
        let program = compile("let x = 5; let y = 10;");
        assert_eq!(program, Program {
            instructions: vec![
                Instruction::Constant(0),
                Instruction::DefineGlobal(0),
                Instruction::Constant(1),
                Instruction::DefineGlobal(1),
                Instruction::Halt,
            ],
            constants: vec![
                Value::Number(5.0),
                Value::Number(10.0),
            ],
            global_count: 2
        });
    }
}