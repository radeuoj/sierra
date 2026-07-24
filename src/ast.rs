use std::hash::{DefaultHasher, Hash, Hasher};

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
        ty: Type,
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuncType {
    pub return_type: Type,
    pub params: Vec<Type>,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
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
