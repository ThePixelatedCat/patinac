use crate::helpers::{Spannable, Spnd};

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Const {
        name: String,
        ty: Option<TypeS>,
        value: ExprS,
    },
    Func {
        name: String,
        params: Vec<PatternS>,
        return_ty: Option<TypeS>,
        body: ExprS,
    },
    Record {
        def: TypeDef,
        fields: Vec<FieldS>,
    },
    Enum {
        def: TypeDef,
        variants: Vec<Variant>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDef {
    pub name: String,
    pub generic_params: Vec<Spnd<String>>
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Variant {
    Unit(String),
    Tuple(String, Vec<TypeS>),
    Struct(String, Vec<FieldS>),
}

pub type FieldS = Spnd<Field>;
impl Spannable for Field {}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub ty: TypeS,
}

pub type PatternS = Spnd<Pattern>;
impl Spannable for Pattern {}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
    Var {
        mutable: bool,
        ident: String,
        annotated_ty: Option<TypeS>,
    },
}

pub type TypeS = Spnd<Type>;
impl Spannable for Type {}
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

pub type ExprS = Spnd<Expr>;
impl Spannable for Expr {}
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
        field: Spnd<String>,
    },
    If {
        cond: Box<ExprS>,
        th: Box<ExprS>,
        el: Option<Box<ExprS>>,
    },
    For {
        pattern: PatternS,
        iter: Box<ExprS>,
        body: Box<ExprS>,
    },
    While {
        cond: Box<ExprS>,
        body: Box<ExprS>,
    },
    Match {
        scrutinee: Box<ExprS>,
        arms: Vec<MatchArmS>,
    },
    Let {
        binding: PatternS,
        value: Box<ExprS>,
    },
    Assign {
        ident: Spnd<String>,
        value: Box<ExprS>,
    },
    Lambda {
        params: Vec<PatternS>,
        return_type: Option<TypeS>,
        body: Box<ExprS>,
    },
    Block {
        exprs: Vec<ExprS>,
        trailing: bool,
    },
}

pub type MatchArmS = Spnd<MatchArm>;
impl Spannable for MatchArm {}
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: PatternS,
    pub guard: Option<Box<ExprS>>,
    pub body: Box<ExprS>,
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
