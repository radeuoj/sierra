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

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct ExprHash(u64);

pub type BlockStmt = Vec<Statement>;

#[derive(Debug, PartialEq)]
pub struct FuncParam {
    pub name: String,
    pub ty: String,
}

#[derive(Debug, PartialEq)]
pub enum Statement {
    Let {
        name: String,
        ty: String,
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
        return_type: String,
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

impl Expression {
    pub fn get_hash(&self) -> ExprHash {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        ExprHash(hasher.finish())
    }
}
