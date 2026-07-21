#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Illegal,
    Eof,

    Ident(String),
    Int(String),
    String(String),

    Assign,
    Plus,
    Minus,
    Asterisk,
    Slash,
    Bang,
    Equal,
    NotEqual,
    Lt,
    Gt,
    Lte,
    Gte,

    Comma,
    Colon,
    Semicolon,
    Arrow,

    LParen,
    RParen,
    LBrace,
    RBrace,

    Let,
    Fn,
    If,
    Else,
    Return,
    True,
    False,
}

impl Token {
    pub fn from_symbol(symbol: &str) -> Self {
        use Token::*;
        match symbol {
            "let" => Let,
            "fn" => Fn,
            "if" => If,
            "else" => Else,
            "return" => Return,
            "true" => True,
            "false" => False,
            _ => Ident(symbol.to_string()),
        }
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Token::*;

        let res = match self {
            Illegal => "<illegal>",
            Eof => "<eof>",

            Ident(name) => name,
            Int(lit) => lit,
            String(lit) => &format!("\"{lit}\""),

            Assign => "=",
            Plus => "+",
            Minus => "-",
            Asterisk => "*",
            Slash => "/",
            Bang => "!",
            Equal => "==",
            NotEqual => "!=",
            Lt => "<",
            Gt => ">",
            Lte => "<=",
            Gte => ">=",

            Comma => ",",
            Colon => ":",
            Semicolon => ";",
            Arrow => "->",

            LParen => "(",
            RParen => ")",
            LBrace => "{",
            RBrace => "}",

            Let => "let",
            Fn => "fn",
            If => "if",
            Else => "else",
            Return => "return",
            True => "true",
            False => "false",
        };

        write!(f, "{}", res)
    }
}
