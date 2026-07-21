use std::io::Write;
use anyhow::bail;

use crate::token::*;

pub struct Lexer {
    input: Vec<u8>,
    pos: usize,
}

impl Lexer {
    pub fn new(input: Vec<u8>) -> Self {
        Self { input, pos: 0 }
    }

    fn read_char(&mut self) -> char {
        let ch = self.peek_char();
        self.pos += 1;
        ch
    }

    fn peek_char(&self) -> char {
        match self.input.get(self.pos) {
            Some(byte) => *byte as char,
            None => '\0',
        }
    }

    fn unread_byte(&mut self) {
        self.pos -= 1;
    }

    pub fn next_token(&mut self) -> anyhow::Result<Token> {
        self.skip_whitespace();
        use Token::*;

        Ok(match self.read_char() {
            '=' => if self.peek_char() == '=' {
                self.read_char();
                Equal
            } else {
                Assign
            }
            '<' => if self.peek_char() == '=' {
                self.read_char();
                Lte
            } else {
                Lt
            }
            '>' => if self.peek_char() == '=' {
                self.read_char();
                Gte
            } else {
                Gt
            }
            '+' => Plus,
            '-' => if self.peek_char() == '>' {
                self.read_char();
                Arrow
            } else {
                Minus
            }
            '*' => Asterisk,
            '/' => if self.peek_char() == '/' {
                self.read_char();
                self.skip_comment();
                self.next_token()?
            } else {
                Slash
            }
            '!' => if self.peek_char() == '=' {
                self.read_char();
                NotEqual
            } else {
                Bang
            }
            ',' => Comma,
            ':' => Colon,
            ';' => Semicolon,
            '(' => LParen,
            ')' => RParen,
            '{' => LBrace,
            '}' => RBrace,
            '"' => Token::String(self.read_string()?.to_string()),
            '\0' => Eof,
            ch => if ch.is_ascii_digit() {
                self.unread_byte();
                Int(self.read_int().to_string())
            } else if Self::is_ident_char(ch) {
                self.unread_byte();
                let ident = self.read_ident()?;
                Token::from_symbol(ident)
            } else {
                Illegal
            }
        })
    }

    fn skip_whitespace(&mut self) {
        while " \t\n\r".contains(self.peek_char()) {
            self.read_char();
        }
    }

    fn skip_comment(&mut self) {
        while !"\0\n".contains(self.peek_char()) {
            self.read_char();
        }
    }

    fn is_ident_char(ch: char) -> bool {
        ch.is_ascii_alphabetic() || ch.is_ascii_digit() || ch == '_'
    }

    fn read_ident(&mut self) -> anyhow::Result<&str> {
        let start = self.pos;

        while Self::is_ident_char(self.peek_char()) {
            self.read_char();
        }

        Ok(str::from_utf8(&self.input[start..self.pos])?)
    }

    fn read_int(&mut self) -> &str {
        let start = self.pos;

        while self.peek_char().is_ascii_digit() {
            self.read_char();
        }

        str::from_utf8(&self.input[start..self.pos]).unwrap()
    }

    fn read_string(&mut self) -> anyhow::Result<&str> {
        let start = self.pos;

        while !['"', '\0'].contains(&self.peek_char()) {
            self.read_char();
        }

        if self.read_char() != '"' {
            bail!("Expected \" at the end of the string literal");
        }

        Ok(str::from_utf8(&self.input[start..self.pos - 1])?)
    }
}

#[allow(unused)]
pub fn repl() {
    println!("Welcome to lexer REPL");

    loop {
        print!("> ");
        std::io::stdout().flush().unwrap();

        let mut buffer = String::new();
        std::io::stdin().read_line(&mut buffer).unwrap();

        let mut lexer = Lexer::new(buffer.into_bytes());

        loop {
            let tok = lexer.next_token().unwrap();
            if tok == Token::Eof { break; }
            println!("{tok}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer() {
        let input = b"let a: int32 = b + c * (12 - _radu); // this is a comment
            return a + b;";

        use Token::*;
        let ok = vec![
            Let,
            Ident("a".to_string()),
            Colon,
            Ident("int32".to_string()),
            Assign,
            Ident("b".to_string()),
            Plus,
            Ident("c".to_string()),
            Asterisk,
            LParen,
            Int("12".to_string()),
            Minus,
            Ident("_radu".to_string()),
            RParen,
            Semicolon,
            Return,
            Ident("a".to_string()),
            Plus,
            Ident("b".to_string()),
            Semicolon,
        ];

        let mut lexer = Lexer::new(input.to_vec());
        let mut out = vec![];

        let mut tok = lexer.next_token().unwrap();
        while tok != Eof {
            out.push(tok);
            tok = lexer.next_token().unwrap();
        }

        assert_eq!(ok, out);
    }
}
