use crate::token::Token;

pub type NodeId = usize;

#[derive(Debug, PartialEq)]
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
        right: NodeId,
    },
    Binary {
        op: Token,
        left: NodeId,
        right: NodeId,
    },
    Call {
        func: NodeId,
        args: Vec<NodeId>,
    }
}

pub type BlockStmt = Vec<NodeId>;

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
        value: Option<NodeId>,
    },
    Return {
        value: NodeId,
    },
    If {
        cond: NodeId,
        then: BlockStmt,
        else_then: BlockStmt,
    },
    Func {
        name: String,
        return_ty: String,
        params: Vec<FuncParam>,
        body: Option<BlockStmt>,
    },
    Expr {
        value: NodeId,
    }
}

#[derive(Debug)]
pub struct Program {
    pub body: Vec<NodeId>,
    pub expressions: Vec<Expression>,
    pub statements: Vec<Statement>,
}
