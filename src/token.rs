#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Token {
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
    Ampersand,

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

    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
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
            "i8" => I8,
            "u8" => U8,
            "i16" => I16,
            "u16" => U16,
            "i32" => I32,
            "u32" => U32,
            "i64" => I64,
            "u64" => U64,
            "f32" => F32,
            "f64" => F64,
            _ => Ident(symbol.to_string()),
        }
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Token::*;

        let res = match self {
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
            Ampersand => "&",

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

            I8 => "i8",
            U8 => "u8",
            I16 => "i16",
            U16 => "u16",
            I32 => "i32",
            U32 => "u32",
            I64 => "i64",
            U64 => "u64",
            F32 => "f32",
            F64 => "f64",
        };

        write!(f, "{}", res)
    }
}
