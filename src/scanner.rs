use crate::token::Token;

pub struct Scanner {
    source: String,
    pub tokens: Vec<Token>,
    start: usize,
    current: usize,
}

impl Scanner {
    pub fn new<S: ToString>(source: S) -> Scanner {
        Scanner {
            source: source.to_string(),
            tokens: Vec::new(),
            start: 0,
            current: 0,
        }
    }

    pub fn scan_tokens(&mut self) {
        while !self.is_at_end() {
            self.start = self.current;
            self.scan_token();
        }

        self.tokens.push(Token::Eof);
    }

    fn scan_token(&mut self) {
        let c = self.advance();

        match c {
            ' ' | '\r' | '\t' | '\n' => {},
            '(' => self.add_token(Token::LeftParen),
            ')' => self.add_token(Token::RightParen),
            '{' => self.add_token(Token::LeftBrace),
            '}' => self.add_token(Token::RightBrace),
            ',' => self.add_token(Token::Comma),
            '-' => self.add_token(Token::Minus),
            '+' => self.add_token(Token::Plus),
            ';' => self.add_token(Token::Semicolon),
            '*' => self.add_token(Token::Star),
            '.' => self.add_token(Token::Dot),
            '!' => self.match_token('=', Token::BangEqual, Token::Bang),
            '=' => self.match_token('=', Token::EqualEqual, Token::Equal),
            '>' => self.match_token('=', Token::GreaterEqual, Token::Greater),
            '<' => self.match_token('=', Token::LessEqual, Token::Less),
            '/' => {
                if self.match_char('/') {
                    self.skip_comment()
                } else {
                    self.add_token(Token::Slash);
                }
            },
            '"' => self.string('"'),
            '\'' => self.string('\''),
            '0'..='9' => self.number(),
            'a'..='z' | 'A'..='Z' | '_' => self.identifier(),
            _ => panic!("Unexpected character {}", c),
        }
    }

    fn skip_comment(&mut self) {
        while self.peek() != '\n' && !self.is_at_end() {
            self.advance();
        }
    }

    fn string(&mut self, terminator: char) {
        let mut string = String::new();
        let mut escape = false;

        while self.peek() != terminator || escape {

            assert!(!self.is_at_end(), "Unterminated string.");

            let c = self.advance();

            if escape {
                escape = false;
                match c {
                    'n' => string.push('\n'),
                    't' => string.push('\t'),
                    'r' => string.push('\r'),
                    _ => string.push(c),
                }
            } else if c == '\\' {
                escape = true;
            } else {
                string.push(c);
            }
        }

        self.advance();

        self.add_token(Token::String(string));
    }

    fn number(&mut self) {
        while self.peek().is_digit(10) {
            self.advance();
        }

        if self.peek() == '.' && self.peek_next().is_digit(10) {
            self.advance();

            while self.peek().is_digit(10) {
                self.advance();
            }
        }

        let value = self.source[self.start..self.current].parse::<f64>().unwrap();
        self.add_token(Token::Number(value));
    }

    fn identifier(&mut self) {
        while self.peek().is_alphanumeric() {
            self.advance();
        }

        let value = &self.source[self.start..self.current];
        let token = match value {
            "and" => Token::And,
            "class" => Token::Class,
            "else" => Token::Else,
            "false" => Token::False,
            "fn" => Token::Fn,
            "if" => Token::If,
            "let" => Token::Let,
            "nil" => Token::Nil,
            "or" => Token::Or,
            "print" => Token::Print,
            "return" => Token::Return,
            "super" => Token::Super,
            "this" => Token::This,
            "true" => Token::True,
            "while" => Token::While,
            _ => Token::Identifier(value.to_string()),
        };

        self.add_token(token);
    }

    fn match_token(&mut self, expected: char, token_type: Token, token_type_if_not_equal: Token) {
        if self.match_char(expected) {
            self.add_token(token_type);
        } else {
            self.add_token(token_type_if_not_equal);
        }
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() || self.source_at(self.current) != expected {
            return false;
        }

        self.current += 1;
        true
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.source_at(self.current)
        }
    }

    fn source_at(&self, index: usize) -> char {
        self.source.chars().nth(index).unwrap()
    }

    fn peek_next(&self) -> char {
        if self.current + 1 >= self.source.len() {
            '\0'
        } else {
            self.source_at(self.current + 1)
        }
    }

    fn is_at_end(&self) -> bool {
        self.source.chars().nth(self.current).is_none()
    }

    fn add_token(&mut self, token: Token) {
        self.tokens.push(token);
    }

    fn advance(&mut self) -> char {
        self.current += 1;
        self.source_at(self.current - 1)
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_identifier() {
        let mut scanner = Scanner::new("foo");
        scanner.scan_tokens();
        assert_eq!(scanner.tokens, vec![Token::Identifier("foo".to_string()), Token::Eof,]);
    }

    #[test]
    fn test_scan_number() {
        let mut scanner = Scanner::new("123");
        scanner.scan_tokens();
        assert_eq!(scanner.tokens, vec![Token::Number(123.0), Token::Eof,]);
    }

    #[test]
    fn test_scan_string() {
        let mut scanner = Scanner::new("\"foo\";");
        scanner.scan_tokens();
        assert_eq!(scanner.tokens, vec![Token::String("foo".to_string()), Token::Semicolon, Token::Eof,]);
    }

    #[test]
    fn test_scan_string_with_escaped_characters() {
        let mut scanner = Scanner::new("\"\\\"\\n\\r\\t\"");
        scanner.scan_tokens();
        assert_eq!(scanner.tokens, vec![Token::String("\"\n\r\t".to_string()), Token::Eof,]);
    }

    #[test]
    fn scan_expression() {
        let mut scanner = Scanner::new("1 + 2");
        scanner.scan_tokens();
        assert_eq!(scanner.tokens, vec![
            Token::Number(1.0),
            Token::Plus,
            Token::Number(2.0),
            Token::Eof,
        ]);
    }

    #[test]
    fn scan_expression_with_multiple_operators() {
        let mut scanner = Scanner::new("1 + 2 * 3 - 4 / 5");
        scanner.scan_tokens();
        assert_eq!(scanner.tokens, vec![
            Token::Number(1.0),
            Token::Plus,
            Token::Number(2.0),
            Token::Star,
            Token::Number(3.0),
            Token::Minus,
            Token::Number(4.0),
            Token::Slash,
            Token::Number(5.0),
            Token::Eof,
        ]);
    }

    #[test]
    fn test_scan_keywords() {
        let mut scanner = Scanner::new("and else false fn if let nil or print return true while");
        scanner.scan_tokens();

        assert_eq!(scanner.tokens, vec![
            Token::And,
            Token::Else,
            Token::False,
            Token::Fn,
            Token::If,
            Token::Let,
            Token::Nil,
            Token::Or,
            Token::Print,
            Token::Return,
            Token::True,
            Token::While,
            Token::Eof,
        ]);
    }

    #[test]
    fn test_scan_multiple_tokens() {
        let mut scanner = Scanner::new("let five = 5;");
        scanner.scan_tokens();

        assert_eq!(scanner.tokens, vec![
            Token::Let,
            Token::Identifier("five".to_string()),
            Token::Equal,
            Token::Number(5.0),
            Token::Semicolon,
            Token::Eof,
        ]);
    }

    #[test]
    fn test_scan_multiple_tokens_with_comments() {
        let mut scanner = Scanner::new("let five = 5; // comment");
        scanner.scan_tokens();

        assert_eq!(scanner.tokens, vec![
            Token::Let,
            Token::Identifier("five".to_string()),
            Token::Equal,
            Token::Number(5.0),
            Token::Semicolon,
            Token::Eof,
        ]);
    }

    #[test]
    fn test_scan_multiple_tokens_with_comments_and_multiple_lines() {
        let mut scanner = Scanner::new("let five = 5; // comment\nlet ten = 10;");
        scanner.scan_tokens();

        assert_eq!(scanner.tokens, vec![
            Token::Let,
            Token::Identifier("five".to_string()),
            Token::Equal,
            Token::Number(5.0),
            Token::Semicolon,
            Token::Let,
            Token::Identifier("ten".to_string()),
            Token::Equal,
            Token::Number(10.0),
            Token::Semicolon,
            Token::Eof,
        ]);
    }

    #[test]
    fn test_scan_example_program() {
        let mut scanner = Scanner::new("
            let five = 5;
            let ten = 10;

            let add = fn(x, y) {
                x + y;
            };

            let result = add(five, ten);
            !-/*5;
            5 < 10 > 5;

            if (5 < 10) {
                return true;
            } else {
                return false;
            }

            10 == 10;
            10 != 9;
        ");
        scanner.scan_tokens();

        assert_eq!(scanner.tokens, vec![
            Token::Let,
            Token::Identifier("five".to_string()),
            Token::Equal,
            Token::Number(5.0),
            Token::Semicolon,
            Token::Let,
            Token::Identifier("ten".to_string()),
            Token::Equal,
            Token::Number(10.0),
            Token::Semicolon,
            Token::Let,
            Token::Identifier("add".to_string()),
            Token::Equal,
            Token::Fn,
            Token::LeftParen,
            Token::Identifier("x".to_string()),
            Token::Comma,
            Token::Identifier("y".to_string()),
            Token::RightParen,
            Token::LeftBrace,
            Token::Identifier("x".to_string()),
            Token::Plus,
            Token::Identifier("y".to_string()),
            Token::Semicolon,
            Token::RightBrace,
            Token::Semicolon,
            Token::Let,
            Token::Identifier("result".to_string()),
            Token::Equal,
            Token::Identifier("add".to_string()),
            Token::LeftParen,
            Token::Identifier("five".to_string()),
            Token::Comma,
            Token::Identifier("ten".to_string()),
            Token::RightParen,
            Token::Semicolon,
            Token::Bang,
            Token::Minus,
            Token::Slash,
            Token::Star,
            Token::Number(5.0),
            Token::Semicolon,
            Token::Number(5.0),
            Token::Less,
            Token::Number(10.0),
            Token::Greater,
            Token::Number(5.0),
            Token::Semicolon,
            Token::If,
            Token::LeftParen,
            Token::Number(5.0),
            Token::Less,
            Token::Number(10.0),
            Token::RightParen,
            Token::LeftBrace,
            Token::Return,
            Token::True,
            Token::Semicolon,
            Token::RightBrace,
            Token::Else,
            Token::LeftBrace,
            Token::Return,
            Token::False,
            Token::Semicolon,
            Token::RightBrace,
            Token::Number(10.0),
            Token::EqualEqual,
            Token::Number(10.0),
            Token::Semicolon,
            Token::Number(10.0),
            Token::BangEqual,
            Token::Number(9.0),
            Token::Semicolon,
            Token::Eof,
        ]);
    }
}