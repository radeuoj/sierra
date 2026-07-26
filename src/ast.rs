use std::{fmt::Display, hash::{DefaultHasher, Hash, Hasher}};

use crate::token::Token;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Expression {
    Ident {
        value: String,
    },
    Int {
        value: String,
    },
    String {
        value: String,
    },
    Unary {
        op: Token,
        right: Box<Expression>,
    },
    Binary {
        op: Token,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Call {
        func: Box<Expression>,
        args: Vec<Expression>,
    },
}

impl Expression {
    pub fn get_hash(&self) -> ExprHash {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        ExprHash(hasher.finish())
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct ExprHash(u64);

pub type BlockStmt = Vec<Statement>;

#[derive(Debug, PartialEq)]
pub struct FuncParam {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, PartialEq)]
pub enum Statement {
    Let {
        name: String,
        ty: Option<Type>,
        value: Option<Expression>,
    },
    Return {
        value: Expression,
    },
    If {
        cond: Expression,
        then: BlockStmt,
        else_then: BlockStmt,
    },
    Func {
        name: String,
        return_type: Type,
        params: Vec<FuncParam>,
        body: Option<BlockStmt>,
    },
    Expr {
        value: Expression,
    }
}

#[derive(Debug)]
pub struct File {
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Void,
    Primitive(Primitive),
    Func(Box<FuncType>),
    Ptr(Box<Type>)
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Void => write!(f, "<void>"),
            Type::Primitive(ty) => write!(f, "{ty}"),
            Type::Func(ty) => write!(f, "{ty}"),
            Type::Ptr(ty) => write!(f, "*{ty}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuncType {
    pub return_type: Type,
    pub params: Vec<Type>,
}

impl Display for FuncType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fn({}) -> {}",
            self.params.iter()
                .map(|ty| ty.to_string())
                .reduce(|acc, ty| format!("{acc}, {ty}"))
                .unwrap_or_default(),
            self.return_type,
        )
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Primitive {
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

impl Display for Primitive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Primitive::I8 => "i8",
            Primitive::U8 => "u8",
            Primitive::I16 => "i16",
            Primitive::U16 => "u16",
            Primitive::I32 => "i32",
            Primitive::U32 => "u32",
            Primitive::I64 => "i64",
            Primitive::U64 => "u64",
            Primitive::F32 => "f32",
            Primitive::F64 => "f64",
        })
    }
}
