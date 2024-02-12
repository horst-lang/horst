use std::collections::HashMap;
use std::mem;
use crate::frame::Chunk;
use crate::function::Function;
use crate::gc::GcRef;
use crate::instruction::Instruction;
use crate::scanner::{Scanner, Token, TokenType};
use crate::value::Value;
use crate::vm::{FunctionUpvalue, Module};

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
enum Precedence {
    None,
    Assignment,  // =
    Or,          // or
    And,         // and
    Equality,    // == !=
    Comparison,  // < > <= >=
    Term,        // + -
    Factor,      // * /
    Unary,       // ! -
    Call,        // . () []
    Primary
}

impl Precedence {
    fn next(&self) -> Precedence {
        match self {
            Precedence::None => Precedence::Assignment,
            Precedence::Assignment => Precedence::Or,
            Precedence::Or => Precedence::And,
            Precedence::And => Precedence::Equality,
            Precedence::Equality => Precedence::Comparison,
            Precedence::Comparison => Precedence::Term,
            Precedence::Term => Precedence::Factor,
            Precedence::Factor => Precedence::Unary,
            Precedence::Unary => Precedence::Call,
            Precedence::Call => Precedence::Primary,
            Precedence::Primary => Precedence::None,
        }
    }
}

type ParseFn<'src> = fn(&mut Parser<'src>, can_assign: bool) -> ();

#[derive(Copy, Clone)]
struct ParseRule<'src> {
    prefix: Option<ParseFn<'src>>,
    infix: Option<ParseFn<'src>>,
    precedence: Precedence,
}

impl<'src> ParseRule<'src> {
    fn new(prefix: Option<ParseFn<'src>>, infix: Option<ParseFn<'src>>, precedence: Precedence) -> Self {
        ParseRule {
            prefix,
            infix,
            precedence,
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct Local<'src> {
    name: Token<'src>,
    depth: i32,
    is_captured: bool,
}

impl<'src> Local<'src> {
    fn new(name: Token<'src>, depth: i32) -> Self {
        Local {
            name,
            depth,
            is_captured: false,
        }
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum FunctionType {
    Function,
    Method,
    Initializer,
    Script,
}

struct Compiler<'src> {
    enclosing: Option<Box<Compiler<'src>>>,
    function: Function,
    function_type: FunctionType,
    locals: Vec<Local<'src>>,
    scope_depth: i32,
}

impl<'src> Compiler<'src> {
    fn new(function_name: &'src str, function_type: FunctionType, module: GcRef<Module>) -> Box<Self> {
        let mut compiler = Compiler {
            enclosing: None,
            function: Function::new(function_name, 0, Chunk::new(""), module),
            function_type,
            locals: Vec::new(),
            scope_depth: 0,
        };
        let token = match function_type {
            FunctionType::Method | FunctionType::Initializer => Token::synthetic("this"),
            _ => Token::synthetic(""),
        };
        compiler.locals.push(Local::new(token, 0));
        Box::new(compiler)
    }

    fn resolve_local(&self, name: Token<'src>, errors: &mut Vec<&'static str>) -> Option<usize> {
        for (i, local) in self.locals.iter().enumerate().rev() {
            if local.name.lexeme == name.lexeme {
                if local.depth == -1 {
                    errors.push("Cannot read local variable in its own initializer.");
                }
                return Some(i);
            }
        }
        None
    }

    fn resolve_upvalue(&mut self, name: Token<'src>, errors: &mut Vec<&'static str>) -> Option<usize> {
        if let Some(enclosing) = &mut self.enclosing {
            if let Some(local) = enclosing.resolve_local(name, errors) {
                enclosing.locals[local].is_captured = true;
                return Some(self.add_upvalue(local, true, errors));
            }
            if let Some(upvalue) = enclosing.resolve_upvalue(name, errors) {
                return Some(self.add_upvalue(upvalue, false, errors));
            }
        }
        None
    }

    fn add_upvalue(&mut self, index: usize, is_local: bool, errors: &mut Vec<&'static str>) -> usize {
        for (i, upvalue) in self.function.upvalues.iter().enumerate() {
            if upvalue.index == index && upvalue.is_local == is_local {
                return i;
            }
        }
        let upvalue = FunctionUpvalue { index, is_local };
        self.function.upvalues.push(upvalue);
        self.function.upvalues.len() - 1
    }

    fn is_local_declared(&self, name: Token<'src>) -> bool {
        for local in self.locals.iter().rev() {
            if local.depth != -1 && local.depth < self.scope_depth {
                break;
            }
            if local.name.lexeme == name.lexeme {
                return true;
            }
        }
        false
    }
}

struct ClassCompiler {
    enclosing: Option<Box<ClassCompiler>>,
    has_superclass: bool,
}

impl ClassCompiler {
    fn new(enclosing: Option<Box<ClassCompiler>>) -> Box<Self> {
        Box::new(ClassCompiler {
            enclosing,
            has_superclass: false,
        })
    }
}

pub struct Parser<'src> {
    scanner: Scanner<'src>,
    compiler: Box<Compiler<'src>>,
    class_compiler: Option<Box<ClassCompiler>>,
    current: Token<'src>,
    previous: Token<'src>,
    had_error: bool,
    panic_mode: bool,
    resolver_errors: Vec<&'static str>,
    rules: HashMap<TokenType, ParseRule<'src>>,
    pub module: GcRef<Module>,
}

impl<'src> Parser<'src> {
    fn new(source: &'src str, module: GcRef<Module>) -> Self {
        let rules = Parser::rules();
        Parser {
            scanner: Scanner::new(source),
            compiler: Compiler::new("script", FunctionType::Script, module),
            class_compiler: None,
            current: Token::synthetic(""),
            previous: Token::synthetic(""),
            had_error: false,
            panic_mode: false,
            resolver_errors: Vec::new(),
            rules,
            module
        }
    }

    fn rules() -> HashMap<TokenType, ParseRule<'src>> {
        let mut rules = HashMap::new();

        let mut rule = |kind, prefix, infix, precedence| {
            rules.insert(kind, ParseRule::new(prefix, infix, precedence));
        };

        use Precedence as P;
        use TokenType::*;

        rule(LeftParen, Some(Parser::grouping), Some(Parser::call), P::Call);
        rule(RightParen, None, None, P::None);
        rule(LeftBrace, None, None, P::None);
        rule(RightBrace, None, None, P::None);
        rule(LeftBracket, Some(Parser::array), Some(Parser::index), P::Call);
        rule(RightBracket, None, None, P::None);
        rule(Comma, None, None, P::None);
        rule(Dot, None, Some(Parser::dot), P::Call);
        rule(Minus, Some(Parser::unary), Some(Parser::binary), P::Term);
        rule(Plus, None, Some(Parser::binary), P::Term);
        rule(Semicolon, None, None, P::None);
        rule(Slash, None, Some(Parser::binary), P::Factor);
        rule(Star, None, Some(Parser::binary), P::Factor);
        rule(Percent, None, Some(Parser::binary), P::Factor);
        rule(Bang, Some(Parser::unary), None, P::None);
        rule(BangEqual, None, Some(Parser::binary), P::Equality);
        rule(Equal, None, None, P::None);
        rule(EqualEqual, None, Some(Parser::binary), P::Equality);
        rule(Greater, None, Some(Parser::binary), P::Comparison);
        rule(GreaterEqual, None, Some(Parser::binary), P::Comparison);
        rule(Less, None, Some(Parser::binary), P::Comparison);
        rule(LessEqual, None, Some(Parser::binary), P::Comparison);
        rule(Identifier, Some(Parser::variable), None, P::None);
        rule(String, Some(Parser::string), None, P::None);
        rule(Number, Some(Parser::number), None, P::None);
        rule(And, None, Some(Parser::and_op), P::And);
        rule(Class, None, None, P::None);
        rule(Else, None, None, P::None);
        rule(False, Some(Parser::literal), None, P::None);
        rule(For, None, None, P::None);
        rule(Fn, None, None, P::None);
        rule(If, None, None, P::None);
        rule(Import, None, None, P::None);
        rule(Nil, Some(Parser::literal), None, P::None);
        rule(Or, None, Some(Parser::or_op), P::Or);
        rule(Print, None, None, P::None);
        rule(Return, None, None, P::None);
        rule(Super, Some(Parser::super_), None, P::None);
        rule(This, Some(Parser::this), None, P::None);
        rule(True, Some(Parser::literal), None, P::None);
        rule(Let, None, None, P::None);
        rule(While, None, None, P::None);
        rule(Error, None, None, P::None);
        rule(Eof, None, None, P::None);

        rules
    }

    fn compile(mut self) -> Result<Function, ()> {
        self.advance();

        while !self.matches(TokenType::Eof) {
            self.declaration();
        }

        self.emit_return();

        if self.had_error {
            Err(())
        } else {
            Ok(self.compiler.function)
        }
    }

    fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    fn expression_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expect ';' after expression.");
        self.emit(Instruction::Pop);
    }

    fn declaration(&mut self) {
        if self.matches(TokenType::Class) {
            self.class_declaration();
        } else if self.matches(TokenType::Fn) {
            self.fun_declaration();
        } else if self.matches(TokenType::Let) {
            self.var_declaration();
        } else {
            self.statement();
        }

        if self.panic_mode {
            self.synchronize();
        }
    }

    fn class_declaration(&mut self) {
        self.consume(TokenType::Identifier, "Expect class name.");
        let class_name = self.previous;
        let name_constant = self.identifier_constant(class_name);
        self.declare_variable();
        self.emit(Instruction::Class(name_constant));
        self.define_variable(name_constant);

        let old_class_compiler = self.class_compiler.take();
        let new_class_compiler = ClassCompiler::new(old_class_compiler);
        self.class_compiler.replace(new_class_compiler);

        if self.matches(TokenType::Less) {
            self.consume(TokenType::Identifier, "Expect superclass name.");
            self.variable(false);
            if class_name.lexeme == self.previous.lexeme {
                self.error("A class can't inherit from itself.");
            }
            self.begin_scope();
            self.add_local(Token::synthetic("super"));
            self.define_variable(0);
            self.named_variable(class_name, false);
            self.emit(Instruction::Inherit);
            self.class_compiler.as_mut().unwrap().has_superclass = true;
        }

        self.named_variable(class_name, false);
        self.consume(TokenType::LeftBrace, "Expect '{' before class body.");
        while !self.check(TokenType::RightBrace) && !self.check(TokenType::Eof) {
            self.method();
        }
        self.consume(TokenType::RightBrace, "Expect '}' after class body.");
        self.emit(Instruction::Pop);
        if self.class_compiler.as_ref().unwrap().has_superclass {
            self.end_scope();
        }

        match self.class_compiler.take() {
            Some(c) => self.class_compiler = c.enclosing,
            None => self.class_compiler = None,
        }

    }

    fn fun_declaration(&mut self) {
        let global = self.parse_variable("Expect function name.");
        self.mark_initialized();
        self.function(FunctionType::Function);
        self.define_variable(global);
    }

    fn push_compiler(&mut self, kind: FunctionType) {
        let function_name = self.previous.lexeme;
        let new_compiler = Compiler::new(function_name, kind, self.module);
        let old_compiler = mem::replace(&mut self.compiler, new_compiler);
        self.compiler.enclosing = Some(old_compiler);
    }

    fn pop_compiler(&mut self) -> Function {
        self.emit_return();
        match self.compiler.enclosing.take() {
            Some(enclosing) => {
                let compiler = mem::replace(&mut self.compiler, enclosing);
                compiler.function
            }
            None => panic!("Did not find enclosing compiler."),
        }
    }

    fn function(&mut self, kind: FunctionType) {
        self.push_compiler(kind);
        self.begin_scope();
        self.consume(TokenType::LeftParen, "Expect '(' after function name.");
        if !self.check(TokenType::RightParen) {
            loop {
                self.compiler.function.arity += 1;
                let param_constant = self.parse_variable("Expect parameter name.");
                self.define_variable(param_constant);
                if !self.matches(TokenType::Comma) {
                    break;
                }
            }
        }
        self.consume(TokenType::RightParen, "Expect ')' after parameters.");
        self.consume(TokenType::LeftBrace, "Expect '{' before function body.");
        self.block();
        let function = self.pop_compiler();
        let index = self.make_constant(Value::Function(function));
        self.emit(Instruction::Closure(index));
    }

    fn method(&mut self) {
        self.consume(TokenType::Identifier, "Expect method name.");
        let constant = self.identifier_constant(self.previous);
        let function_type = if self.previous.lexeme == "init" {
            FunctionType::Initializer
        } else {
            FunctionType::Method
        };
        self.function(function_type);
        self.emit(Instruction::Method(constant));
    }

    fn var_declaration(&mut self) {
        let index = self.parse_variable("Expect variable name.");

        if self.matches(TokenType::Equal) {
            self.expression();
        } else {
            self.emit(Instruction::Nil);
        }
        self.consume(TokenType::Semicolon, "Expect ';' after variable declaration.");
        self.define_variable(index);
    }

    fn define_variable(&mut self, index: usize) {
        if self.compiler.scope_depth > 0 {
            self.mark_initialized();
            return;
        }
        self.emit(Instruction::DefineGlobal(index));
    }

    fn mark_initialized(&mut self) {
        if self.compiler.scope_depth == 0 {
            return;
        }
        let local = self.compiler.locals.last_mut().unwrap();
        local.depth = self.compiler.scope_depth;
    }

    fn statement(&mut self) {
        if self.matches(TokenType::Print) {
            self.print_statement();
        } else if self.matches(TokenType::If) {
            self.if_statement();
        } else if self.matches(TokenType::Import) {
            self.import_statement();
        } else if self.matches(TokenType::Return) {
            self.return_statement();
        } else if self.matches(TokenType::Do) {
            self.do_while_statement();
        } else if self.matches(TokenType::While) {
            self.while_statement();
        } else if self.matches(TokenType::For) {
            self.for_statement();
        } else if self.matches(TokenType::LeftBrace) {
            self.begin_scope();
            self.block();
            self.end_scope();
        } else {
            self.expression_statement();
        }
    }

    fn return_statement(&mut self) {
        if self.compiler.function_type == FunctionType::Script {
            self.error("Can't return from top-level code.");
        }
        if self.matches(TokenType::Semicolon) {
            self.emit_return();
        } else {
            if self.compiler.function_type == FunctionType::Initializer {
                self.error("Can't return a value from an initializer.");
            }
            self.expression();
            self.consume(TokenType::Semicolon, "Expect ';' after return value.");
            self.emit(Instruction::Return);
        }
    }

    fn if_statement(&mut self) {
        self.consume(TokenType::LeftParen, "Expect '(' after 'if'.");
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after condition.");

        let then_jump = self.emit(Instruction::JumpIfFalse(0));
        self.emit(Instruction::Pop);
        self.statement();
        let else_jump = self.emit(Instruction::Jump(0));

        self.patch_jump(then_jump);
        self.emit(Instruction::Pop);

        if self.matches(TokenType::Else) {
            self.statement();
        }
        self.patch_jump(else_jump);
    }

    fn import_statement(&mut self) {
        self.consume(TokenType::String, "Expect a string after 'import'.");
        let module_name = self.previous;
        let module_constant = self.make_constant(Value::String(module_name.lexeme[1..module_name.lexeme.len() - 1].to_string()));
        self.emit(Instruction::ImportModule(module_constant));

        if !self.matches(TokenType::For) {
            self.consume(TokenType::Semicolon, "Expect ';' after module name.");
            return;
        }

        loop {
            self.consume(TokenType::Identifier, "Expect variable name.");
            let variable = self.previous;
            let name_constant = self.identifier_constant(variable);
            let slot;
            if self.matches(TokenType::As) {
                self.consume(TokenType::Identifier, "Expect variable name.");
                let variable_name = self.previous;
                slot = self.identifier_constant(variable_name);
                self.declare_variable();
            } else {
                slot = name_constant;
                self.declare_variable();
            }
            self.emit(Instruction::ImportVariable(name_constant));
            self.define_variable(slot);

            if !self.matches(TokenType::Comma) {
                break;
            }
        }
        self.consume(TokenType::Semicolon, "Expect ';' after import statement.");
    }

    fn while_statement(&mut self) {
        let loop_start = self.start_loop();
        self.consume(TokenType::LeftParen, "Expect '(' after 'while'.");
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after condition.");

        // Condition over

        let exit_jump = self.emit(Instruction::JumpIfFalse(0));
        self.emit(Instruction::Pop);
        self.statement();
        self.emit_loop(loop_start);

        self.patch_jump(exit_jump);
        self.emit(Instruction::Pop);
    }

    fn do_while_statement(&mut self) {
        let loop_start = self.start_loop();

        self.statement();

        self.consume(TokenType::While, "Expect 'while' in 'do while'");
        self.consume(TokenType::LeftParen, "Expect '(' after 'while'.");
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after condition.");

        let exit_jump = self.emit(Instruction::JumpIfFalse(2));
        self.emit(Instruction::Pop);
        self.emit_loop(loop_start);
        self.emit(Instruction::Pop);
    }

    // Either normal or for-in loop.
    fn for_statement(&mut self) {
        self.begin_scope();
        self.consume(TokenType::LeftParen, "Expect '(' after 'for'.");

        // Initializer clause.
        if self.matches(TokenType::Semicolon) {
            // No initializer.
        } else if self.matches(TokenType::Let) {
            self.consume(TokenType::Identifier, "Expect variable name.");

            if self.check(TokenType::In) {
                self.for_in_statement();
                self.end_scope();
                return;
            }

            self.declare_variable();
            let mut index;
            if self.compiler.scope_depth > 0 {
                index = 0;
            }
            index = self.identifier_constant(self.previous);

            if self.matches(TokenType::Equal) {
                self.expression();
            } else {
                self.emit(Instruction::Nil);
            }
            self.consume(TokenType::Semicolon, "Expect ';' after variable declaration.");
            self.define_variable(index);
        } else {
            self.expression_statement();
        }

        let mut loop_start = self.start_loop();

        // Condition clause.
        let mut exit_jump = None;
        if !self.matches(TokenType::Semicolon) {
            self.expression();
            self.consume(TokenType::Semicolon, "Expect ';' after loop condition.");

            exit_jump = Some(self.emit(Instruction::JumpIfFalse(0)));
            self.emit(Instruction::Pop);
        }

        // Increment clause.
        if !self.matches(TokenType::RightParen) {
            let body_jump = self.emit(Instruction::Jump(0));
            let increment_start = self.start_loop();
            self.expression();
            self.emit(Instruction::Pop);
            self.consume(TokenType::RightParen, "Expect ')' after for clauses.");

            self.emit_loop(loop_start);
            loop_start = increment_start;
            self.patch_jump(body_jump);
        }

        self.statement();
        self.emit_loop(loop_start);

        if let Some(exit_jump) = exit_jump {
            self.patch_jump(exit_jump);
            self.emit(Instruction::Pop);
        }

        self.end_scope();
    }

    fn for_in_statement(&mut self) {
        let identifier = self.previous;
        self.consume(TokenType::In, "Expect 'in' after loop variable.");
        self.expression();
        let iterator_const = self.identifier_constant(Token::synthetic("iterator"));
        self.emit(Instruction::Invoke(iterator_const, 0));
        self.add_local(Token::synthetic("$iterator"));
        let iterator = self.compiler.locals.len() - 1;
        self.define_variable(iterator);
        self.consume(TokenType::RightParen, "Expect ')' after expression.");


        let loop_start = self.start_loop();
        self.emit(Instruction::GetLocal(iterator));
        let has_next = self.identifier_constant(Token::synthetic("hasNext"));
        self.emit(Instruction::Invoke(has_next, 0));

        let exit_jump = self.emit(Instruction::JumpIfFalse(0));
        self.emit(Instruction::Pop);

        self.emit(Instruction::GetLocal(iterator));
        let next = self.identifier_constant(Token::synthetic("next"));
        self.emit(Instruction::Invoke(next, 0));
        self.add_local(identifier);
        self.define_variable(self.compiler.locals.len() - 1);

        self.statement();

        self.emit(Instruction::Pop);
        self.emit_loop(loop_start);

        self.patch_jump(exit_jump);
    }

    fn begin_scope(&mut self) {
        self.compiler.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.compiler.scope_depth -= 1;
        for i in (0..self.compiler.locals.len()).rev() {
            if self.compiler.locals[i].depth > self.compiler.scope_depth {
                if self.compiler.locals[i].is_captured {
                    self.emit(Instruction::CloseUpvalue);
                } else {
                    self.emit(Instruction::Pop);
                }
                self.compiler.locals.pop();
            }
        }
    }

    fn block(&mut self) {
        while !self.check(TokenType::RightBrace) && !self.check(TokenType::Eof) {
            self.declaration();
        }
        self.consume(TokenType::RightBrace, "Expect '}' after block.");
    }

    fn print_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expect ';' after value.");
        self.emit(Instruction::Print);
    }

    fn number(&mut self, _can_assign: bool) {
        let value = self.previous.lexeme.parse::<f64>().unwrap();
        self.emit_constant(Value::Number(value));
    }

    fn string(&mut self, _can_assign: bool) {
        let lexeme = self.previous.lexeme;
        let value = lexeme[1..lexeme.len() - 1].to_string();
        self.emit_constant(Value::String(value));
    }

    fn literal(&mut self, _can_assign: bool) {
        match self.previous.kind {
            TokenType::False => self.emit(Instruction::False),
            TokenType::Nil => self.emit(Instruction::Nil),
            TokenType::True => self.emit(Instruction::True),
            _ => unreachable!(),
        };
    }

    fn variable(&mut self, can_assign: bool) {
        self.named_variable(self.previous, can_assign);
    }

    fn super_(&mut self, _can_assign: bool) {
        if let Some(current_class) = self.class_compiler.as_ref() {
            if !current_class.has_superclass {
                self.error("Can't use 'super' in a class with no superclass.");
            }
        } else {
            self.error("Can't use 'super' outside of a class.");
        }
        self.consume(TokenType::Dot, "Expect '.' after 'super'.");
        self.consume(TokenType::Identifier, "Expect superclass method name.");
        let name = self.identifier_constant(self.previous);
        self.named_variable(Token::synthetic("this"), false);

        if self.matches(TokenType::LeftParen) {
            let arg_count = self.argument_list();
            self.named_variable(Token::synthetic("super"), false);
            self.emit(Instruction::SuperInvoke(name, arg_count));
        } else {
            self.named_variable(Token::synthetic("super"), false);
            self.emit(Instruction::GetSuper(name));
        }
    }

    fn this(&mut self, _can_assign: bool) {
        if self.class_compiler.is_none() {
            self.error("Can't use 'this' outside of a class.");
            return;
        }
        self.variable(false);
    }

    fn named_variable(&mut self, name: Token<'src>, can_assign: bool) {
        let get_op;
        let set_op;
        if let Some(local) = self.resolve_local(name) {
            get_op = Instruction::GetLocal(local);
            set_op = Instruction::SetLocal(local);
        } else if let Some(upvalue) = self.resolve_upvalue(name) {
            get_op = Instruction::GetUpvalue(upvalue);
            set_op = Instruction::SetUpvalue(upvalue);
        } else {
            let global = self.identifier_constant(name);
            get_op = Instruction::GetGlobal(global);
            set_op = Instruction::SetGlobal(global);
        }

        if can_assign && self.matches(TokenType::Equal) {
            self.expression();
            self.emit(set_op);
        } else {
            self.emit(get_op);
        }
    }

    fn resolve_local(&mut self, name: Token) -> Option<usize> {
        let result = self.compiler.resolve_local(name, &mut self.resolver_errors);
        while let Some(e) = self.resolver_errors.pop() {
            self.error(e)
        }
        result
    }

    fn resolve_upvalue(&mut self, name: Token<'src>) -> Option<usize> {
        let result = self.compiler.resolve_upvalue(name, &mut self.resolver_errors);
        while let Some(e) = self.resolver_errors.pop() {
            self.error(e)
        }
        result
    }

    fn call(&mut self, _can_assign: bool) {
        let arg_count = self.argument_list();
        self.emit(Instruction::Call(arg_count));
    }

    fn dot(&mut self, can_assign: bool) {
        self.consume(TokenType::Identifier, "Expect property name after '.'.");
        let name = self.identifier_constant(self.previous);

        if can_assign && self.matches(TokenType::Equal) {
            self.expression();
            self.emit(Instruction::SetProperty(name));
        } else if self.matches(TokenType::LeftParen) {
            let arg_count = self.argument_list();
            self.emit(Instruction::Invoke(name, arg_count));
        } else {
            self.emit(Instruction::GetProperty(name));
        }
    }

    fn index(&mut self, _can_assign: bool) {
        self.expression();
        self.consume(TokenType::RightBracket, "Expect ']' after index.");
        let get = self.identifier_constant(Token::synthetic("get"));
        let set = self.identifier_constant(Token::synthetic("set"));

        if self.matches(TokenType::Equal) {
            self.expression();
            self.emit(Instruction::Invoke(set, 2));
        } else {
            self.emit(Instruction::Invoke(get, 1));
        }
    }

    fn argument_list(&mut self) -> usize {
        let mut arg_count = 0;
        if !self.check(TokenType::RightParen) {
            loop {
                self.expression();
                arg_count += 1;
                if !self.matches(TokenType::Comma) {
                    break;
                }
            }
        }
        self.consume(TokenType::RightParen, "Expect ')' after arguments.");
        arg_count
    }

    fn grouping(&mut self, _can_assign: bool) {
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after expression.");
    }

    fn array(&mut self, _can_assign: bool) {
        let mut arg_count = 0;
        let array = self.identifier_constant(Token::synthetic("Array"));
        self.emit(Instruction::GetGlobal(array));
        let args = self.emit(Instruction::Nil);
        self.emit(Instruction::Call(1));
        self.add_local(Token::synthetic("$array"));
        let array = self.compiler.locals.len() - 1;
        self.mark_initialized();
        if !self.check(TokenType::RightBracket) {
            loop {
                self.emit(Instruction::GetLocal(array));
                self.emit_constant(Value::Number(arg_count as f64));
                self.expression();
                let set = self.identifier_constant(Token::synthetic("set"));
                self.emit(Instruction::Invoke(set, 2));
                self.emit(Instruction::Pop);
                arg_count += 1;
                if !self.matches(TokenType::Comma) {
                    break;
                }
            }
        }
        // Patch args
        let args_num = self.make_constant(Value::Number(arg_count as f64));
        self.compiler.function.chunk.code[args] = Instruction::Constant(args_num);
        self.consume(TokenType::RightBracket, "Expect ']' after array elements.");
    }

    fn unary(&mut self, _can_assign: bool) {
        let operator_type = self.previous.kind;
        self.parse_precedence(Precedence::Unary);
        match operator_type {
            TokenType::Bang => self.emit(Instruction::Not),
            TokenType::Minus => self.emit(Instruction::Negate),
            _ => unreachable!(),
        };
    }

    fn binary(&mut self, _can_assign: bool) {
        let operator_type = self.previous.kind;
        let rule = self.get_rule(operator_type);
        self.parse_precedence(rule.precedence.next());
        match operator_type {
            TokenType::Plus => self.emit(Instruction::Add),
            TokenType::Minus => self.emit(Instruction::Subtract),
            TokenType::Star => self.emit(Instruction::Multiply),
            TokenType::Slash => self.emit(Instruction::Divide),
            TokenType::Percent => self.emit(Instruction::Modulo),
            TokenType::BangEqual => self.emit_two(Instruction::Equal, Instruction::Not),
            TokenType::EqualEqual => self.emit(Instruction::Equal),
            TokenType::Greater => self.emit(Instruction::Greater),
            TokenType::GreaterEqual => self.emit_two(Instruction::Less, Instruction::Not),
            TokenType::Less => self.emit(Instruction::Less),
            TokenType::LessEqual => self.emit_two(Instruction::Greater, Instruction::Not),
            _ => unreachable!(),
        };
    }

    fn and_op(&mut self, _can_assign: bool) {
        let end_jump = self.emit(Instruction::JumpIfFalse(0));
        self.emit(Instruction::Pop);
        self.parse_precedence(Precedence::And);
        self.patch_jump(end_jump);
    }

    fn or_op(&mut self, _can_assign: bool) {
        let else_jump = self.emit(Instruction::JumpIfFalse(0));
        let end_jump = self.emit(Instruction::Jump(0));
        self.patch_jump(else_jump);
        self.emit(Instruction::Pop);
        self.parse_precedence(Precedence::Or);
        self.patch_jump(end_jump);
    }

    fn parse_precedence(&mut self, precedence: Precedence) {
        self.advance();
        let prefix_rule = self.get_rule(self.previous.kind).prefix;

        let prefix_rule = match prefix_rule {
            Some(rule) => rule,
            None => {
                self.error("Expect expression.");
                return;
            }
        };

        let can_assign = precedence <= Precedence::Assignment;
        prefix_rule(self, can_assign);

        while self.is_lower_precedence(precedence) {
            self.advance();
            let infix_rule = self.get_rule(self.previous.kind).infix.unwrap();
            infix_rule(self, can_assign);
        }

        if can_assign && self.matches(TokenType::Equal) {
            self.error("Invalid assignment target.");
        }
    }

    fn parse_variable(&mut self, error_message: &str) -> usize {
        self.consume(TokenType::Identifier, error_message);
        self.declare_variable();
        if self.compiler.scope_depth > 0 {
            return 0;
        }
        self.identifier_constant(self.previous)
    }

    fn identifier_constant(&mut self, name: Token) -> usize {
        self.make_constant(Value::String(name.lexeme.to_string()))
    }

    fn declare_variable(&mut self) {
        if self.compiler.scope_depth == 0 {
            return;
        }
        let name = self.previous;
        if self.compiler.is_local_declared(name) {
            self.error("Variable with this name already declared in this scope.");
        }
        self.add_local(name);
    }

    fn add_local(&mut self, name: Token<'src>) {
        let local = Local::new(name, -1);
        self.compiler.locals.push(local);
    }

    fn is_lower_precedence(&self, precedence: Precedence) -> bool {
        let current_precedence = self.get_rule(self.current.kind).precedence;
        precedence <= current_precedence
    }

    fn consume(&mut self, expected: TokenType, message: &str) {
        if self.current.kind == expected {
            self.advance();
            return;
        }
        self.error_at_current(message);
    }

    fn advance(&mut self) {
        self.previous = self.current;
        loop {
            self.current = self.scanner.scan_token();
            if self.current.kind != TokenType::Error {
                break;
            }
            self.error_at_current(self.current.lexeme);
        }
    }

    fn matches(&mut self, expected: TokenType) -> bool {
        if !self.check(expected) {
            return false;
        }
        self.advance();
        true
    }

    fn check(&self, expected: TokenType) -> bool {
        self.current.kind == expected
    }

    fn error_at_current(&mut self, message: &str) {
        self.error_at(self.current, message);
    }

    fn error(&mut self, message: &str) {
        self.error_at(self.previous, message);
    }

    fn error_at(&mut self, token: Token, message: &str) {
        if self.panic_mode {
            return;
        }
        self.panic_mode = true;
        eprint!("[line {}] Error", token.line);
        match token.kind {
            TokenType::Eof => eprint!(" at end"),
            TokenType::Error => {}
            _ => eprint!(" at '{}'", token.lexeme),
        }
        eprintln!(": {}", message);
        self.had_error = true;
    }

    fn synchronize(&mut self) {
        self.panic_mode = false;
        while self.previous.kind != TokenType::Eof {
            if self.previous.kind == TokenType::Semicolon {
                return;
            }
            match self.current.kind {
                TokenType::Class
                | TokenType::Fn
                | TokenType::Let
                | TokenType::For
                | TokenType::If
                | TokenType::While
                | TokenType::Print
                | TokenType::Return => return,
                _ => {}
            }
            self.advance();
        }
    }

    fn emit(&mut self, instruction: Instruction) -> usize {
        self.compiler.function.chunk.write(instruction, self.previous.line)
    }

    fn emit_two(&mut self, instruction1: Instruction, instruction2: Instruction) -> usize {
        self.emit(instruction1);
        self.emit(instruction2)
    }

    fn emit_return(&mut self) {
        match self.compiler.function_type {
            FunctionType::Initializer => self.emit(Instruction::GetLocal(0)),
            _ => self.emit(Instruction::Nil),
        };
        self.emit(Instruction::Return);
    }

    fn start_loop(&mut self) -> usize {
        self.compiler.function.chunk.code.len()
    }

    fn emit_loop(&mut self, start: usize) {
        let offset = self.start_loop() - start + 1;
        self.emit(Instruction::Loop(offset));
    }

    fn patch_jump(&mut self, pos: usize) {
        let offset = self.start_loop() - pos - 1;

        match self.compiler.function.chunk.code[pos] {
            Instruction::JumpIfFalse(ref mut o) => *o = offset,
            Instruction::Jump(ref mut o) => *o = offset,
            _ => panic!("Expected jump instruction."),
        }
    }

    fn make_constant(&mut self, value: Value) -> usize {
        self.compiler.function.chunk.add_constant(value)
    }

    fn emit_constant(&mut self, value: Value) {
        let constant = self.make_constant(value);
        self.emit(Instruction::Constant(constant));
    }

    fn get_rule(&self, kind: TokenType) -> ParseRule<'src> {
        self.rules.get(&kind).cloned().unwrap()
    }

}

pub fn compile(source: &str, module: GcRef<Module>) -> Result<Function, ()> {
    let parser = Parser::new(source, module);
    parser.compile()
}