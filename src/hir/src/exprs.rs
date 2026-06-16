use std::range::Range;

use slotmap::new_key_type;

use ident::SpanIdent;

use crate::VarId;

new_key_type! { pub struct ExprId; }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stmt {
    Decl {
        id: VarId,
        val: ExprId,
        span: Range<u32>,
    },
    Expr(ExprId),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Var(VarId),
    Lit(LitExpr),
    Array(Vec<ExprId>),
    Tuple(Vec<ExprId>),
    Infix {
        op: InfixOp,
        lhs: ExprId,
        rhs: ExprId,
    },
    Prefix {
        op: PrefixOp,
        expr: ExprId,
    },
    Field {
        base: ExprId,
        field: SpanIdent,
    },
    Index {
        array: ExprId,
        index: ExprId,
    },
    Call {
        func: ExprId,
        args: Vec<Arg>,
    },
    Lambda {
        params: Vec<VarId>,
        body: ExprId,
        captures: Vec<VarId>,
    },
    Assign {
        place: ExprId,
        value: ExprId,
    },
    If {
        cond: ExprId,
        th: BlockExpr,
        el: Option<BlockExpr>,
    },
    For {
        id: VarId,
        iter: ExprId,
        body: BlockExpr,
    },
    Loop(BlockExpr),
    Break,
    Continue,
    Return(ExprId),
    Block(BlockExpr),

    Print(ExprId),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LitExpr {
    Int(u64),
    Float(f64),
    Char(char),
    String(String),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arg {
    pub value: ExprId,
    pub mutable: bool,
    pub span: Range<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockExpr {
    pub stmts: Vec<Stmt>,
    pub span: Range<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfixOp {
    Add,
    AddF,
    Sub,
    SubF,
    Mul,
    MulF,
    Div,
    DivF,
    Exp,
    And,
    Or,
    Xor,
    Eqq,
    Neq,
    Gt,
    Lt,
    Geq,
    Leq,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixOp {
    Not,
    Neg,
}
