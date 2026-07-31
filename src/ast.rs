use std::{fmt::Display, hash::{DefaultHasher, Hash, Hasher}};

use crate::token::Token;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Expression {
    Ident {
        value: String,
        id: u64,
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
    At {
        left: Box<Expression>,
        right: Box<Expression>,
    }
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
    While {
        cond: Expression,
        block: BlockStmt,
    },
    Func {
        name: String,
        return_type: Type,
        params: Vec<FuncParam>,
        body: Option<BlockStmt>,
    },
    Expr {
        value: Expression,
    },
}

#[derive(Debug)]
pub struct File {
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Void,
    Named(String),
    Ptr(Box<Type>),
    Array(Box<Type>, u64),
    Slice(Box<Type>),
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Void => write!(f, "<void>"),
            Type::Named(name) => write!(f, "{name}"),
            Type::Ptr(ty) => write!(f, "*{ty}"),
            Type::Array(ty, size) => write!(f, "[{size}]{ty}"),
            Type::Slice(ty) => write!(f, "[]{ty}"),
        }
    }
}
