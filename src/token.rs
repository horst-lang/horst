#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    // One character tokens
    LeftParen,              // "("
    RightParen,             // ")"
    LeftBrace,              // "{"
    RightBrace,             // "}"
    Comma,                  // ","
    Minus,                  // "-"
    Plus,                   // "+"
    Semicolon,              // ";"
    Slash,                  // "/"
    Star,                   // "*"
    Dot,                    // "."

    // One or two character tokens
    Bang,                   // "!"
    BangEqual,              // "!="
    Equal,                  // "="
    EqualEqual,             // "=="
    Greater,                // ">"
    GreaterEqual,           // ">="
    Less,                   // "<"
    LessEqual,              // "<="

    // Literals
    Identifier(String),
    String(String),
    Number(f64),

    // Keywords
    And,                    // "and"
    Class,                  // "class"
    Else,                   // "else"
    False,                  // "false"
    Fn,                     // "fn"
    If,                     // "if"
    Let,                    // "let"
    Nil,                    // "nil"
    Or,                     // "or"
    Print,                  // "print"
    Return,                 // "return"
    Super,                  // "super"
    This,                   // "this"
    True,                   // "true"
    While,                  // "while"

    // End of file
    Eof,

    // Error
    Error,
}