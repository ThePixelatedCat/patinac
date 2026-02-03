use crate::{helpers::Spanned, span};

pub type Ast = Vec<ItemS>;

span! {Item as ItemS}
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Const {
        name: String,
        ty: Option<TypeS>,
        value: ExprS,
    },
    Func {
        name: String,
        params: Vec<BindingS>,
        return_ty: Option<TypeS>,
        body: ExprS,
    },
    Struct {
        name: String,
        generic_params: Vec<String>,
        fields: Vec<FieldS>,
    },
    Enum {
        name: String,
        generic_params: Vec<String>,
        variants: Vec<VariantS>,
    },
}

span! {Variant as VariantS}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Variant {
    Unit(String),
    Tuple(String, Vec<TypeS>),
    Struct(String, Vec<FieldS>),
}

// pub type FieldS = Spanned<Field>;
// impl Spannable for Field {}
span! {Field as FieldS}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub ty: TypeS,
}

span! {Binding as BindingS}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Binding {
    Var {
        mutable: bool,
        ident: String,
        annotated_ty: Option<TypeS>,
    },
}

span! {Type as TypeS}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    UInt,
    Byte,
    Float,
    Bool,
    Char,
    Array(Box<TypeS>),
    Tuple(Vec<TypeS>),
    Fn(Vec<TypeS>, Box<TypeS>),
    Named { name: String, args: Vec<TypeS> },
}

span! {Expr as ExprS}
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Ident(String),
    Int(u64),
    Float(f64),
    String(String),
    Char(char),
    Bool(bool),
    Array(Vec<ExprS>),
    Tuple(Vec<ExprS>),
    FnCall {
        fun: Box<ExprS>,
        args: Vec<ExprS>,
    },
    BinaryOp {
        op: Bop,
        lhs: Box<ExprS>,
        rhs: Box<ExprS>,
    },
    UnaryOp {
        op: Unop,
        expr: Box<ExprS>,
    },
    Index {
        arr: Box<ExprS>,
        index: Box<ExprS>,
    },
    FieldAccess {
        base: Box<ExprS>,
        field: Spanned<String>,
    },
    If {
        cond: Box<ExprS>,
        th: Box<ExprS>,
        el: Option<Box<ExprS>>,
    },
    Let {
        binding: BindingS,
        value: Box<ExprS>,
    },
    Assign {
        ident: Spanned<String>,
        value: Box<ExprS>,
    },
    Lambda {
        params: Vec<BindingS>,
        return_type: Option<TypeS>,
        body: Box<ExprS>,
    },
    Block {
        exprs: Vec<ExprS>,
        trailing: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bop {
    Add,
    Sub,
    Mul,
    Div,
    Exp,
    And,
    Or,
    Xor,
    BOr,
    BAnd,
    Gt,
    Lt,
    Eqq,
    Neq,
    Geq,
    Leq,
}

impl Bop {
    pub const fn binding_power(self) -> (u8, u8) {
        match self {
            Self::Or => (3, 4),
            Self::And => (5, 6),
            Self::Eqq | Self::Neq => (7, 8),
            Self::Gt | Self::Lt | Self::Leq | Self::Geq => (9, 10),
            Self::BOr => (11, 12),
            Self::Xor => (13, 14),
            Self::BAnd => (15, 16),
            Self::Add | Self::Sub => (17, 18),
            Self::Mul | Self::Div => (19, 20),
            Self::Exp => (22, 21),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unop {
    Not,
    Neg,
}

impl Unop {
    pub const fn binding_power(self) -> u8 {
        match self {
            Self::Neg | Self::Not => 51,
        }
    }
}
