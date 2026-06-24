use std::range::Range;

use slotmap::new_key_type;

use ident::SpanIdent;

use crate::VarId;

new_key_type! {
    /// An ID for an [`Expr`] in a slotmap.
    pub struct ExprId;
}

/// A statement. Always contained within a [`BlockExpr`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stmt {
    /// A variable declaration.
    Decl {
        /// The the variable being bound.
        var: VarId,
        /// The initial value for the variable.
        value: ExprId,
        /// The span of the declaration, starting from the `let` and ending after the [`value`][`Stmt::Decl::value`].
        span: Range<u32>,
    },
    /// An expression used as a statement. Evaluated purely for side-effects.
    Expr(ExprId),
}
/// An expression.
#[derive(Debug, PartialEq)]
pub enum Expr {
    /// A reference to a named value.
    Var(VarId),
    /// A scalar literal value. The specific kinds of literals are represented by [`LitExpr`].
    Lit(LitExpr),
    /// An array literal.
    Array(Vec<ExprId>),
    /// A tuple literal.
    Tuple(Vec<ExprId>),
    /// An infix operation.
    Infix {
        /// The infix operator used.
        op: InfixOp,
        /// The left-hand side of the operation.
        lhs: ExprId,
        /// The right-hand side of the operation.
        rhs: ExprId,
    },
    /// A prefix operation.
    Prefix {
        /// The prefix operator used.
        op: PrefixOp,
        /// The base expression the operator is applied to.
        expr: ExprId,
    },
    /// Record field access.
    Field {
        /// The base expression from which the field is being accessed.
        base: ExprId,
        /// The name of the field being accessed.
        field: SpanIdent,
    },
    /// Array indexing.
    Index {
        /// The base expression being indexed into.
        array: ExprId,
        /// The index to access.
        index: ExprId,
    },
    /// A function call.
    Call {
        /// The function being called.
        func: ExprId,
        /// The list of arguments being applied.
        args: Vec<Arg>,
    },
    /// A capturing lambda.
    Lambda {
        /// The parameters of the function.
        params: Vec<VarId>,
        /// List of external capture - local rebinding pairs.
        captures: Vec<(VarId, VarId)>,
        /// The body of the function.
        body: ExprId,
    },
    /// A variable assignment.
    Assign {
        /// The place being assigned to, on the lhs of the `=`. This semantically must be a "place expression".
        place: ExprId,
        /// The value being assigned, on the rhs of the `=`.
        value: ExprId,
    },
    /// An if-then, with an optional else branch.
    If {
        /// The condition of the if.
        cond: ExprId,
        /// The "then" block.
        th: BlockExpr,
        /// The "else" block, if there is one.
        el: Option<BlockExpr>,
    },
    /// A loop over the elements of an iterator.
    For {
        /// The variable to bind each element to.
        id: VarId,
        /// The iterator to be iterated over.
        iter: ExprId,
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
    Return(ExprId),
    /// A block, which executes each contained statement sequentially.
    /// Blocks evaluates the value of the last statement, or unit if the last statement is not an expression.
    Block(BlockExpr),

    /// TEMPORARY, until we have stdlib + FFI.
    Print(ExprId),
}

/// The kinds of [literal expressions][Expr::Lit].
#[derive(Debug, PartialEq)]
pub enum LitExpr {
    /// An integer, of any of the [three][crate::Ty::Int] [integer][crate::Ty::UInt] [types][crate::Ty::Byte]. Sign is not part of the literal.
    Int(u64),
    /// A float. Can include sign and exponent.
    Float(f64),
    /// A string. Common escape sequences and raw strings are supported.
    String(String),
    /// A boolean.
    Bool(bool),
}
/// An argument in a [ function call][ExprKind::Call], consisting of an expression that may have a mutability annotation.
#[derive(Debug, PartialEq, Eq)]
pub struct Arg {
    /// The value of the function argument.
    pub value: ExprId,
    /// Whether this argument is mutable. If it is, the value must be a place expression.
    pub mutable: bool,
    /// The total span of the argument.
    /// If `mutable` is false, this should be identical to the span of `val`.
    /// Otherwise, it should include the span of the `mut` keyword.
    pub span: Range<u32>,
}

/// A block of statements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockExpr {
    /// The statements within the block.
    pub stmts: Vec<Stmt>,
    /// The total span of the block, from opening to closing brace.
    pub span: Range<u32>,
}

/// An infix operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfixOp {
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
    /// `^`.
    Exp,
    /// `&&`.
    And,
    /// `||`.
    Or,
    /// `==`.
    Eqq,
    /// `!=`.
    Neq,
    /// `>`.
    Gt,
    /// `<`.
    Lt,
    /// `>=`.
    Geq,
    /// `<=`.
    Leq,
}

/// A prefix operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixOp {
    /// `!`. Logical negation.
    Not,
    /// `-`. Numeric negation.
    Neg,
}
