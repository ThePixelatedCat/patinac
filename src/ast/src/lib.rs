//! The representation of source code produced by the parser, before any semantic analysis.
//! The primary type is [`Ast`].

use std::{
    fmt::{self, Display, Formatter},
    range::Range,
};

use package::ModuleId;
use slotmap::SecondaryMap;
use smallvec::SmallVec;

use ident::{Ident, SpanIdent};

/// The [`Asts`][Ast] for an entire package.
pub struct PackageAsts(SecondaryMap<ModuleId, Ast>);

impl PackageAsts {
    /// Returns a reference to the [`Ast`] for the provided module.
    pub fn get(&self, id: ModuleId) -> &Ast {
        &self.0[id]
    }

    /// Removes and returns the [`Ast`] for the provided module.
    pub fn take(&mut self, id: ModuleId) -> Ast {
        self.0.remove(id).expect("id is valid for this map")
    }
}

impl FromIterator<(ModuleId, Ast)> for PackageAsts {
    fn from_iter<T: IntoIterator<Item = (ModuleId, Ast)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

/// The top-level representation of a single module, containing all of the module's items.
///
/// For easier manipulation, these items are split into [ type definitions][TyItem] and "[executable items][ExecItem]" (items containing expressions).
#[derive(Default)]
pub struct Ast {
    /// The `import`s and `export`s of a module.
    pub vis_items: Vec<VisItem>,
    /// The type definitions of a module, containing both `enum` and `record` definitions.
    pub ty_items: Vec<TyItem>,
    /// The "executable items" of a module. These are the items that contain expressions.
    pub exec_items: Vec<ExecItem>,
}

/// A path made of one or more identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Path {
    head: SmallVec<[Ident; 4]>,
    tail: Ident,
}

impl Display for Path {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut string = String::new();
        for ident in &self.head {
            string.push_str(&ident.str());
            string.push_str("::");
        }
        string.push_str(&self.tail.str());
        string.fmt(f)
    }
}

impl Path {
    /// Create a path made of a single identifier.
    pub const fn single(ident: Ident) -> Self {
        Self {
            head: SmallVec::new_const(),
            tail: ident,
        }
    }

    /// Attempts to create a path, returning None if the provided `Vec` is empty.
    pub fn new(mut path: Vec<Ident>) -> Option<Self> {
        let tail = path.pop()?;
        Some(Self {
            head: path.into(),
            tail,
        })
    }

    /// Creates a path of a constant length.
    ///
    /// # Panics
    /// Will panic at compile-time if the length is 0.
    pub fn new_const<const N: usize>(path: [Ident; N]) -> Self {
        const { assert!(N > 0, "path must be non-empty") }
        Self {
            head: SmallVec::from_slice(&path[0..N - 1]),
            tail: path[N - 1],
        }
    }

    /// Add to the end of the path.
    pub fn push(&mut self, ident: Ident) {
        self.head.push(self.tail);
        self.tail = ident;
    }

    /// Returns the first identifier of the path.
    pub fn start(&self) -> Ident {
        if self.head.is_empty() {
            self.tail
        } else {
            self.head[0]
        }
    }

    /// Returns the last identifier of the path, which is the name of the referenced item.
    pub fn end(&self) -> Ident {
        self.tail
    }

    /// Returns true if the path is made up of a single identifier.
    pub fn is_single_ident(&self) -> bool {
        self.head.is_empty()
    }

    /// Returns the first identifier of the path, and the rest of the path if it had more than 1 segment.
    pub fn split(mut self) -> (Ident, Option<Self>) {
        if self.head.is_empty() {
            (self.tail, None)
        } else {
            let start = self.head.remove(0);
            (start, Some(self))
        }
    }
}

/// An `import` or `export`.
#[derive(Debug, PartialEq)]
pub enum VisItem {
    /// An `import` item, abbreviating the path required to refer to an item.
    Import(Path, Range<u32>),
    /// An `export` item, exposing the listed local items to the parent module.
    Export(Vec<SpanIdent>),
}

/// The definition of a type, either a `record` or a `enum`.
#[derive(Debug, PartialEq)]
pub struct TyItem {
    /// The name of the type.
    pub ident: SpanIdent,
    /// The declared generic parameters.
    pub generics: SmallVec<[SpanIdent; 4]>,
    /// The kind of type (`record` or `enum`).
    pub kind: TyItemKind,
}

/// The information of a [`TyItem`] specific to whether it's a `record` or an `enum`.
#[derive(Debug, PartialEq)]
pub enum TyItemKind {
    /// A `record` type.
    Record(Vec<Field>),
    /// An `enum` type.
    Enum(Vec<Variant>),
}

/// A variant of an `enum`.
#[derive(Debug, PartialEq)]
pub struct Variant {
    /// The name of the variant.
    pub ident: SpanIdent,
    /// The fields contained within the variant.
    pub fields: Vec<Field>,
}

#[derive(Debug, PartialEq)]
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
#[derive(Debug, PartialEq)]
pub struct Param {
    /// Whether the parameter is mutable (a second-class in-out reference).
    pub mutable: bool,
    /// The pattern to bind the parameter's variables.
    pub pat: Pat,
    /// The type of the parameter.
    pub ty: Ty,
    /// The span of the parameter, from the `mut` keyword if present to the [`ty`][Param::ty].
    pub span: Range<u32>,
}

/// A statement. Always contained within a [`BlockExpr`].
#[derive(Debug, PartialEq)]
pub enum Stmt {
    /// A variable declaration.
    Decl {
        /// The binding information for the variable.
        binding: Binding,
        /// The initial value for the variable.
        val: Expr,
        /// The span of the declaration, starting from the `let` and ending after the [`val`][`Stmt::Decl::val`].
        span: Range<u32>,
    },
    /// An expression used as a statement. Evaluated purely for side-effects.
    Expr(Expr),
}

/// A spanned [expression][ExprKind].
#[derive(Debug, PartialEq)]
pub struct Expr {
    /// The kind of the expression.
    pub kind: ExprKind,
    /// The span of the expression.
    pub span: Range<u32>,
}

impl Expr {
    /// Creates a single-statement block containing this expression.
    ///
    /// The provided span is to account for the curly braces.
    ///
    /// This function primarily exists as a helper for tests.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::range::Range;
    /// # use ast::{ExprKind, Stmt, BlockExpr};
    /// let expr = ExprKind::Tuple(vec![]).span(2..3);
    /// assert_eq!(
    ///     expr.clone().as_block(0..5),
    ///     BlockExpr {
    ///         stmts: vec![Stmt::Expr(expr.clone())],
    ///         span: Range::from(0..5)
    ///     }
    /// )
    /// ```
    pub fn as_block(self, span: impl Into<Range<u32>>) -> BlockExpr {
        BlockExpr {
            stmts: vec![Stmt::Expr(self)],
            span: span.into(),
        }
    }
}

#[derive(Debug, PartialEq)]
/// The kinds of expressions.
pub enum ExprKind {
    /// A reference to a named value..
    Var(Path),
    /// A scalar literal value. The specific kinds of literals are represented by [`LitExpr`].
    Lit(LitExpr),
    /// An array literal.
    Array(Vec<Expr>),
    /// A tuple literal.
    Tuple(Vec<Expr>),
    /// An infix operation. This includes assignment.
    Infix {
        /// The infix operator used.
        op: InfixOp,
        /// The left-hand side of the operation.
        lhs: Box<Expr>,
        /// The right-hand side of the operation.
        rhs: Box<Expr>,
    },
    /// A prefix operation.
    Prefix {
        /// The prefix operator used.
        op: PrefixOp,
        /// The base expression the operator is attached to.
        expr: Box<Expr>,
    },
    /// Record field access.
    Field {
        /// The base expression from which the field is being accessed.
        base: Box<Expr>,
        /// The name of the field.
        field: SpanIdent,
    },
    /// Array indexing.
    Index {
        /// The base expression being indexed into.
        arr: Box<Expr>,
        /// The index to access.
        idx: Box<Expr>,
    },
    /// A function call.
    Call {
        /// The function being called.
        func: Box<Expr>,
        /// The list of arguments being applied.
        args: Vec<Arg>,
    },
    /// A capturing lambda.
    Lambda {
        /// The parameters of the function.
        params: Vec<Binding>,
        /// The body of the function.
        body: Box<Expr>,
    },
    /// An if-then, with an optional else branch.
    If {
        /// The condition of the if.
        cond: Box<Expr>,
        /// The "then" block.
        th: BlockExpr,
        /// The "else" block, if there is one.
        el: Option<BlockExpr>,
    },
    /// A match.
    Match {
        /// The value being matched against.
        scrutinee: Box<Expr>,
        /// The match arms.
        arms: Vec<MatchArm>,
    },
    /// A loop over the elements of an iterator.
    For {
        /// The pattern to bind each element against.
        pat: Pat,
        /// The iterator to be iterated over.
        iter: Box<Expr>,
        /// The body to execute for each element.
        body: BlockExpr,
    },
    /// An infinite loop.
    Loop(BlockExpr),
    /// Break, which terminates the enclosing [`loop`][Self::Loop] or [`for loop`][Self::For] early.
    Break,
    /// Continue, which skips to the next iteration of the enclosing [`loop`][Self::Loop] or [`for loop`][Self::For].
    Continue,
    /// Return, which returns early from the enclosing function.
    Return(Box<Expr>),
    /// A block, which executes each contained statement sequentially.
    /// Blocks evaluates the value of the last statement, or unit if the last statement is not an expression.
    Block(BlockExpr),

    /// TEMPORARY, until we have stdlib + FFI
    Print(Box<Expr>),
}

impl ExprKind {
    /// Constructs an [`Expr`] wrapping `self` with the provided span.
    pub fn span(self, span: impl Into<Range<u32>>) -> Expr {
        Expr {
            kind: self,
            span: span.into(),
        }
    }

    /// Constructs a single-segment [`var`][Self::Var] expression from the given string.
    pub fn ident(string: &str) -> Self {
        Self::Var(Path::single(Ident::new(string)))
    }

    /// Constructs an integer literal.
    pub const fn int(i: u64) -> Self {
        Self::Lit(LitExpr::Int(i))
    }

    /// Constructs a float literal.
    pub const fn float(f: f64) -> Self {
        Self::Lit(LitExpr::Float(f))
    }

    /// Constructs a character literal.
    pub const fn char(c: char) -> Self {
        Self::Lit(LitExpr::Char(c))
    }

    /// Constructs a string literal.
    pub fn string(s: &str) -> Self {
        Self::Lit(LitExpr::String(String::from(s)))
    }

    /// Constructs a boolean literal.
    pub const fn bool(b: bool) -> Self {
        Self::Lit(LitExpr::Bool(b))
    }
}

/// The kinds of [literal expressions][ExprKind::Lit].
#[derive(Debug, PartialEq)]
pub enum LitExpr {
    /// An integer, of any of the [three][TyKind::Int] [integer][TyKind::UInt] [types][TyKind::Byte]. Sign is not part of the literal.
    Int(u64),
    /// A float. Can include sign and exponent.
    Float(f64),
    /// A character. May be removed.
    Char(char),
    /// A string. Common escape sequences and raw strings are supported.
    String(String),
    /// A boolean.
    Bool(bool),
}

/// A variable binding, combining mutability, pattern, and optional type annotation.
#[derive(Debug, PartialEq)]
pub struct Binding {
    /// Whether this variable is mutable.
    pub mutable: bool,
    /// The pattern to bind the variable against.
    pub pat: Pat,
    /// An optional type annotation for the variable.
    pub ty: Option<Ty>,
}

/// An argument in a [ function call][ExprKind::Call], consisting of an expression that may have a mutability annotation.
#[derive(Debug, PartialEq)]
pub struct Arg {
    /// The value of the function argument.
    pub val: Expr,
    /// Whether this argument is mutable. If it is, the value must be a place expression.
    pub mutable: bool,
    /// The total span of the argument.
    /// If `mutable` is false, this should be identical to the span of `val`.
    /// Otherwise, it should include the span of the `mut` keyword.
    pub span: Range<u32>,
}

/// A single arm of a [match][ExprKind::Match] expression
#[derive(Debug, PartialEq)]
pub struct MatchArm {
    /// The pattern to attempt to match the scrutinee against.
    pub pat: Pat,
    /// The expression to run if the match succeeds.
    pub body: Expr,
}

/// A block of statements. Used by [`ExprKind::Block`], [`ExprKind::If`], [`ExprKind::For`], and [`ExprKind::Loop`].
#[derive(Debug, PartialEq)]
pub struct BlockExpr {
    /// The statements within the block.
    pub stmts: Vec<Stmt>,
    /// The total span of the block, from opening to closing brace.
    pub span: Range<u32>,
}

/// An infix operator. Includes assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfixOp {
    /// `=`.
    Assign,
    /// `+`.
    Add,
    /// TEMPORARY, until we have traits. `+.`.
    AddF,
    /// `-`.
    Sub,
    /// TEMPORARY, until we have traits. `-.`.
    SubF,
    /// `*`.
    Mul,
    /// TEMPORARY, until we have traits. `*.`.
    MulF,
    /// `/`.
    Div,
    /// TEMPORARY, until we have traits. `/.`.
    DivF,
    /// `**`.
    Exp,
    /// `&&`
    And,
    /// `||`
    Or,
    /// `^`
    Xor,
    /// `==`
    Eqq,
    /// `!=`
    Neq,
    /// `>`
    Gt,
    /// `<`
    Lt,
    /// `>=`
    Geq,
    /// `<=`
    Leq,
}

impl InfixOp {
    /// Returns the left and right binding powers of this operator, for Pratt Parsing.
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

/// A prefix operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixOp {
    /// `!`. Logical negation.
    Not,
    /// `-`. Numeric negation.
    Neg,
}

impl PrefixOp {
    /// Returns the right binding power of this operator, for Pratt Parsing.
    pub const fn binding_power(self) -> u8 {
        match self {
            Self::Neg | Self::Not => 50,
        }
    }
}

/// A spanned [type][TyKind].
#[derive(Debug, PartialEq, Eq)]
pub struct Ty {
    /// The kind of the type.
    pub kind: TyKind,
    /// The span of the type.
    pub span: Range<u32>,
}

/// The kinds of types.
#[derive(Debug, PartialEq, Eq)]
pub enum TyKind {
    /// 64-bit signed integer.
    Int,
    /// 64-bit unsigned integer.
    UInt,
    /// 8-bit unsigned integer.
    Byte,
    /// Double-precision floating point number (binary64).
    Float,
    /// TODO
    Char,
    /// Truth value (`true`/`false``).
    Bool,
    /// A dynamic homogenous array.
    Array(Box<Ty>),
    /// A heterogenous tuple (compile-time length).
    Tuple(Vec<Ty>),
    /// A first-class function value, implemented as a closure.
    Func(Vec<FuncTy>, Box<FuncTy>),
    /// A user-defined type, such as a `record` or `enum`.
    Named(Path, Vec<Ty>),
}

impl TyKind {
    /// Constructs an [`Ty`] wrapping `self` with the provided span.
    pub fn span(self, span: impl Into<Range<u32>>) -> Ty {
        Ty {
            kind: self,
            span: span.into(),
        }
    }

    /// Constructs an empty [`TyKind::Tuple`] for representing the Unit type.
    pub const fn unit() -> Self {
        Self::Tuple(vec![])
    }

    /// Constructs a single-segment [`named`][Self::Named] type from the given string.
    pub fn named(name: &str) -> Self {
        Self::Named(Path::single(Ident::new(name)), vec![])
    }
}

/// A parameter or return type of a function type.
#[derive(Debug, PartialEq, Eq)]
pub struct FuncTy {
    /// The type of the paremeter.
    pub ty: Ty,
    /// Whether the parameter is mutable.
    pub mutable: bool,
    /// The total span of the parameter.
    /// If `mutable` is false, this should be identical to the span of `ty`.
    /// Otherwise, it should include the span of the `mut` keyword.
    pub span: Range<u32>,
}

#[derive(Debug, PartialEq)]
pub struct Pat {
    pub kind: PatKind,
    pub span: Range<u32>,
}

#[derive(Debug, PartialEq)]
pub enum PatKind {
    Literal { negate: bool, lit: LitExpr },
    Wildcard,
    Ident(Ident),
    Constructor(Ident, Vec<Pat>),
    Tuple(Vec<Pat>),
}

impl PatKind {
    pub fn span(self, span: impl Into<Range<u32>>) -> Pat {
        Pat {
            kind: self,
            span: span.into(),
        }
    }

    pub fn ident(name: &str) -> Self {
        Self::Ident(Ident::new(name))
    }
}
