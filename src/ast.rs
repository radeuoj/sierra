use crate::token::Token;

pub type NodeId = usize;

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

pub struct FuncParam {
    name: String,
    ty: String,
}

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
    }
}
