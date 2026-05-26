use slotmap::new_key_type;

use ident::SpanIdent;
use smallvec::SmallVec;
use span::Span;

use crate::VarId;

new_key_type! { pub struct ExprId; }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    Decl { id: VarId, val: ExprId, span: Span },
    Expr(ExprId),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Ident(VarId),
    Lit(LitExpr),
    Array(SmallVec<[ExprId; 3]>),
    Tuple(SmallVec<[ExprId; 3]>),
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
        arr: ExprId,
        idx: ExprId,
    },
    Call {
        func: ExprId,
        args: Vec<Arg>,
    },
    Lambda {
        params: SmallVec<[VarId; 3]>,
        body: ExprId,
        captures: SmallVec<[VarId; 3]>,
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
    pub val: ExprId,
    pub mutable: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockExpr {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfixOp {
    Assign,
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
