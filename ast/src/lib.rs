//! The representation of source code produced by the parser, before any semantic analysis.
//! The primary type is [`Ast`].

use std::{
    fmt::{self, Display, Formatter},
    range::Range,
};

use derive_more::Display;
use itertools::Itertools as _;
use smallvec::SmallVec;

use ident::{Ident, SpanIdent};

/// The top-level representation of a program, containing all of the program's items.
///
/// For easier manipulation, these items are split into [ type definitions][TyItem] and "[executable items][ExecItem]" (items containing expressions).
#[derive(Default)]
pub struct Ast {
    /// The type definitions of a program, containing both `enum` and `record` definitions.
    pub tys: Vec<TyItem>,
    /// The "executable items" of a program. These are the items that contain expressions.
    pub execs: Vec<ExecItem>,
}

/// The definition of a type, either a `record` or a `enum`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TyItem {
    /// The name of the type.
    pub ident: SpanIdent,
    /// The declared generic parameters.
    pub generics: SmallVec<[SpanIdent; 4]>,
    /// The kind of type (`record` or `enum`).
    pub kind: TyItemKind,
}

/// The information of a [`TyItem`] specific to whether it's a `record` or an `enum`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TyItemKind {
    /// A `record` type.
    Record(Vec<Field>),
    /// An `enum` type.
    Enum(Vec<Variant>),
}

/// A variant of an `enum`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    /// The name of the variant.
    pub ident: SpanIdent,
    /// The fields contained within the variant.
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A field of a `record` or of an `enum` variant.
pub struct Field {
    /// The name of the field.
    pub ident: SpanIdent,
    /// The type of the field.
    pub ty: Ty,
}

#[derive(Debug, PartialEq)]
/// An "executable item". These are the items that contain expressions, namely constants and functions.
pub struct ExecItem {
    /// The name of the item.
    pub ident: SpanIdent,
    /// The kind of the item (`const` or `fn`).
    pub kind: ExecKind,
}

#[derive(Debug, PartialEq)]
/// The information of an [`ExecItem`] specific to whether it's a `const` or a `fn`.
pub enum ExecKind {
    /// A constant item.
    Const {
        /// The constant's type.
        ty: Ty,
        /// The initialiser expression.
        val: Expr,
    },
    /// A function item.
    Fn {
        /// The generic parameters.
        generics: SmallVec<[SpanIdent; 4]>,
        /// The value parameters.
        params: Vec<Param>,
        /// Whether the return type is mutable (i.e. a projection).
        ret_mut: bool,
        /// The return type.
        ret_ty: Ty,
        /// The body.
        body: Expr,
    },
}

/// A parameter of a [function item][ExecKind::Fn].
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    /// Whether the parameter is mutable (a second-class in-out reference).
    pub mutable: bool,
    /// The pattern to bind the parameter's variables.
    pub pat: Pat,
    /// The type of the parameter.
    pub ty: Ty,
    /// The span of the parameter, from the `mut` keyword if present to the [`ty`][Param::ty].
    pub span: Range<usize>,
}

/// A statement. Always contained within a [`BlockExpr`].
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// A variable declaration.
    Decl {
        /// The binding information for the variable.
        binding: Binding,
        /// The initial value for the variable.
        val: Expr,
        /// The span of the declaration, starting from the `let` and ending after the [`val`][`Stmt::Decl::val`].
        span: Range<usize>,
    },
    /// An expression used as a statement. Evaluated purely for side-effects.
    Expr(Expr),
}

/// A spanned [expression][ExprKind].
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    /// The kind of the expression.
    pub kind: ExprKind,
    /// The span of the expression.
    pub span: Range<usize>,
}

impl Expr {
    /// Creates a single-statement block containing this expression.
    /// The provided span is to account for the curly braces.
    /// Primarily exists as a helper for tests.
    ///
    /// # Example
    pub fn as_block(self, span: impl Into<Range<usize>>) -> BlockExpr {
        BlockExpr {
            stmts: vec![Stmt::Expr(self)],
            span: span.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Ident(Ident),
    Lit(LitExpr),
    Array(Vec<Expr>),
    Tuple(Vec<Expr>),
    Infix {
        op: InfixOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Prefix {
        op: PrefixOp,
        expr: Box<Expr>,
    },
    Field {
        base: Box<Expr>,
        field: SpanIdent,
    },
    Index {
        arr: Box<Expr>,
        idx: Box<Expr>,
    },
    Call {
        func: Box<Expr>,
        args: Vec<Arg>,
    },
    Lambda {
        params: Vec<Binding>,
        body: Box<Expr>,
    },
    If {
        cond: Box<Expr>,
        th: BlockExpr,
        el: Option<BlockExpr>,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    For {
        pat: Pat,
        iter: Box<Expr>,
        body: BlockExpr,
    },
    Loop(BlockExpr),
    Break,
    Continue,
    Return(Box<Expr>),
    Block(BlockExpr),

    Print(Box<Expr>),
}

impl ExprKind {
    pub fn span(self, span: impl Into<Range<usize>>) -> Expr {
        Expr {
            kind: self,
            span: span.into(),
        }
    }

    pub fn ident(string: &str) -> Self {
        Self::Ident(Ident::new(string))
    }

    pub const fn int(i: u64) -> Self {
        Self::Lit(LitExpr::Int(i))
    }

    pub const fn float(f: f64) -> Self {
        Self::Lit(LitExpr::Float(f))
    }

    pub const fn char(c: char) -> Self {
        Self::Lit(LitExpr::Char(c))
    }

    pub fn string(s: &str) -> Self {
        Self::Lit(LitExpr::String(String::from(s)))
    }

    pub const fn bool(b: bool) -> Self {
        Self::Lit(LitExpr::Bool(b))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LitExpr {
    Int(u64),
    Float(f64),
    Char(char),
    String(String),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    pub mutable: bool,
    pub pat: Pat,
    pub ty: Option<Ty>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Arg {
    pub val: Expr,
    pub mutable: bool,
    pub span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pat: Pat,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockExpr {
    pub stmts: Vec<Stmt>,
    pub span: Range<usize>,
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

impl InfixOp {
    pub const fn binding_power(self) -> (u8, u8) {
        match self {
            Self::Assign => (1, 0),
            Self::Or => (3, 4),
            Self::And => (5, 6),
            Self::Eqq | Self::Neq => (7, 8),
            Self::Gt | Self::Lt | Self::Leq | Self::Geq => (9, 10),
            Self::Xor => (13, 14),
            Self::Add | Self::AddF | Self::Sub | Self::SubF => (17, 18),
            Self::Mul | Self::MulF | Self::Div | Self::DivF => (19, 20),
            Self::Exp => (22, 21),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixOp {
    Not,
    Neg,
}

impl PrefixOp {
    pub const fn binding_power(self) -> u8 {
        match self {
            Self::Neg | Self::Not => 51,
        }
    }
}

#[derive(Debug, Display, Clone, PartialEq, Eq, Hash)]
#[display("{kind}")]
pub struct Ty {
    pub kind: TyKind,
    pub span: Range<usize>,
}

/// The kinds of types.
#[derive(Debug, Display, Clone, PartialEq, Eq, Hash)]
pub enum TyKind {
    Int,
    UInt,
    Byte,
    Float,
    Char,
    Bool,
    #[display("[{_0}]")]
    Array(Box<Ty>),
    #[display("{{{}}}", _0.iter().join(", "))]
    Tuple(Vec<Ty>),
    #[display("fn({}) -> {_1}", _0.iter().join(", "))]
    Fn(Vec<ParamTy>, Return),
    #[display("{_0}[{}]", _1.iter().join(", "))]
    Named(Ident, Vec<Ty>),
}

impl TyKind {
    pub fn span(self, span: impl Into<Range<usize>>) -> Ty {
        Ty {
            kind: self,
            span: span.into(),
        }
    }

    /// Helper to create a new empty [`TyKind::Tuple`] for representing the Unit type.
    pub const fn unit() -> Self {
        Self::Tuple(vec![])
    }

    pub fn named(name: &str) -> Self {
        Self::Named(Ident::new(name), vec![])
    }

    /// Helper to create a new [`TyKind::Named`] for a `String`.
    pub fn string() -> Self {
        Self::named("String")
    }
}

/// A parameter of a function type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParamTy {
    pub ty: Ty,
    pub mutable: bool,
    pub span: Range<usize>,
}

impl Display for ParamTy {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.mutable {
            "mut ".fmt(f)?;
        }
        self.ty.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Return {
    pub mutable: bool,
    pub ty: Box<Ty>,
}

impl Display for Return {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.mutable {
            "mut ".fmt(f)?;
        }
        self.ty.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pat {
    pub kind: PatKind,
    pub span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatKind {
    Literal { negate: bool, lit: LitExpr },
    Wildcard,
    Ident(Ident),
    Constructor(Ident, Vec<Pat>),
    Tuple(Vec<Pat>),
}

impl PatKind {
    pub fn span(self, span: impl Into<Range<usize>>) -> Pat {
        Pat {
            kind: self,
            span: span.into(),
        }
    }

    pub fn ident(name: &str) -> Self {
        Self::Ident(Ident::new(name))
    }
}
