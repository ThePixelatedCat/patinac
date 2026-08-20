//! The mid-level intermediate representation of Patina. Lowered from the HIR, and used for code generation.

use slotmap::{SlotMap, new_key_type};

use ident::Ident;

/// The mid-level intermediate representation of Patina.
///
/// - Lambdas have been lowered to closures
/// - Records have been erased and are equivalent to tuples
/// - Location information has been stripped
/// - Type information has been trimmed down to what is necessary
#[derive(Debug, Default)]
pub struct Mir {
    main: Option<Item>,
    items: Vec<Item>,
    exprs: SlotMap<ExprId, Expr>,
    vars: SlotMap<VarId, VarInfo>,
}

impl Mir {
    /// Returns all of the items.
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// Adds an item.
    pub fn add_item(&mut self, item: Item) {
        self.items.push(item);
    }

    /// Returns the main function, if it exists.
    pub const fn main(&self) -> Option<&Item> {
        self.main.as_ref()
    }

    /// Sets the main function.
    pub fn set_main(&mut self, main: Item) {
        self.main = Some(main);
    }
}

// Expr-related functions
impl Mir {
    /// Adds an expression, returning a unique ID for it.
    pub fn add_expr(&mut self, expr: Expr) -> ExprId {
        self.exprs.insert(expr)
    }

    /// Returns the actual expression associated with the given ID.
    pub fn expr(&self, id: ExprId) -> &Expr {
        &self.exprs[id]
    }
}

// Var-related functions
impl Mir {
    /// Adds a variable with the provided information, returning a unique ID for it.
    pub fn add_var(&mut self, info: VarInfo) -> VarId {
        self.vars.insert(info)
    }

    /// Returns the information for the given variable.
    pub fn var(&self, id: VarId) -> &VarInfo {
        &self.vars[id]
    }
}

/// An item.
///
/// The only items directly tracked by the MIR are functions and constants,
/// known as "executable items" in earlier IRs.
///
/// All information for user-defined types has been lowered into normal types,
/// and methods have been fully desugared into regular functions.
#[derive(Debug, PartialEq, Eq)]
pub struct Item {
    /// The variable the item is associated with.
    pub var: VarId,
    /// The kind of the item (constant or function).
    pub kind: ItemKind,
}

/// The information of an [`Item`] specific to whether it's a constant or function.
#[derive(Debug, PartialEq, Eq)]
pub enum ItemKind {
    /// A constant item.
    Const(ExprId),
    /// A function item.
    Func {
        /// The value parameters.
        params: Vec<VarId>,
        /// The body.
        body: ExprId,
    },
}

new_key_type! {
    /// An ID representing a variable. Information about the variable can be acquired through [`Mir::var`].
    pub struct VarId;
}
/// Information for a variable.
#[derive(Debug, Clone)]
pub struct VarInfo {
    /// The name of the variable.
    pub ident: Ident,
    /// Whether the variable was declared mutable.
    pub mutable: bool,
    /// The type of the variable.
    pub ty: Ty,
}

new_key_type! {
    /// An ID corresponding to an [`Expr`].
    pub struct ExprId;
}
/// An expression.
#[derive(Debug, Clone)]
pub enum Expr {
    /// A reference to a named value.
    Var(VarId),
    /// A scalar literal value. The specific kinds of literals are represented by [`LitExpr`].
    Lit(LitExpr),
    /// An array literal, with the type of it's elements.
    Array(Ty, Vec<ExprId>),
    /// A literal for a heterogenous aggreggate value (tuple or record).
    ///
    /// The two vecs should be the same lengths, forming field type - field value pairs.
    Construct(Vec<Ty>, Vec<ExprId>),
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
    /// Field access.
    Field {
        /// The base expression from which the field is being accessed.
        base: ExprId,
        /// The index of the field being accessed.
        field: u32,
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
        /// The return type of the function being called.
        ret_ty: Ty,
    },
    /// A closure.
    Closure {
        /// The lifted function representing the closure body.
        func: VarId,
        /// The captured variables.
        captures: Vec<VarId>,
    },
    /// A variable assignment.
    Assign {
        /// The place being assigned to, on the left of the `=`. Semantically, this must be a "place expression".
        place: ExprId,
        /// The value being assigned, on the right of the `=`.
        value: ExprId,
    },
    /// An if-then, with an optional else branch.
    If {
        /// The type that the if-then evaluates to.
        ty: Ty,
        /// The condition of the if.
        cond: ExprId,
        /// The "then" block.
        th: BlockExpr,
        /// The "else" block, if there is one.
        el: Option<BlockExpr>,
    },
    /// An infinite loop.
    Loop(BlockExpr),
    /// A block, which executes each contained statement sequentially.
    ///
    /// Blocks evaluate to the value of the last statement, or unit if the last statement is not an expression.
    Block(BlockExpr),

    /// TEMPORARY, until we have stdlib + FFI.
    Print(Ty, ExprId),
}

/// The kinds of [literal expressions][Expr::Lit].
#[derive(Debug, Clone, PartialEq)]
pub enum LitExpr {
    /// A signed 64-bit integer. Sign is not part of the literal.
    Int(i64),
    /// An unsigned 64-bit interger.
    UInt(u64),
    /// An unsigned 8-bit integer.
    Byte(u8),
    /// A float. Can include exponent.
    Float(f64),
    /// A boolean.
    Bool(bool),
}

/// An argument in a [ function call][Expr::Call], consisting of an expression that may have a mutability annotation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arg {
    /// The type of the function argument.
    pub ty: Ty,
    /// The value of the function argument.
    pub value: ExprId,
    /// Whether this argument is mutable. If it is, the value must be a place expression.
    pub mutable: bool,
}

/// A block of statements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockExpr(pub Vec<Stmt>);

/// A statement. Always contained within a [`BlockExpr`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stmt {
    /// A variable declaration.
    Decl {
        /// The the variable being bound.
        var: VarId,
        /// The initial value for the variable.
        value: ExprId,
    },
    /// An expression used as a statement, evaluated purely for side-effects.
    Expr(ExprId),
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

/// A prefix operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixOp {
    /// `!`. Logical negation.
    Not,
    /// `-`. Numeric negation.
    Neg,
}

/// The kinds of types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    /// 64-bit signed integer.
    Int,
    /// 64-bit unsigned integer.
    UInt,
    /// 8-bit unsigned integer.
    Byte,
    /// Double-precision floating point number (binary64).
    Float,
    /// Truth value (`true`/`false`).
    Bool,
    /// A dynamic homogenous array.
    Array(Box<Self>),
    /// A collection of heterogenous fields. Tuples and records both get lowered to this.
    Fields(Vec<Self>),
    /// A first-class function value, implemented as a closure.
    Func(Vec<Param>, Box<Self>),
}

impl Ty {
    /// Size of a pointer in bytes. FIXME: Support for non-64 bit architectures?
    const PTR_SIZE: u64 = 8;

    /// Returns the inline size of this type, in bytes.
    pub fn size(&self) -> u64 {
        match self {
            Self::Int | Self::UInt | Self::Float => 8,
            Self::Byte | Self::Bool => 1,
            Self::Fields(field_tys) => {
                let align = self.alignment();
                if align == 0 {
                    return 0;
                }

                let base_size = field_tys.iter().fold(0, |sum, ty| {
                    let align = ty.alignment();
                    let padding = if align == 0 {
                        0
                    } else {
                        (align - (sum % align)) % align
                    };
                    sum + padding + ty.size()
                });

                let end_padding = (align - (base_size % align)) % align;
                base_size + end_padding
            }
            Self::Array(_) => Self::PTR_SIZE,
            Self::Func(_, _) => Self::PTR_SIZE * 5, // Function, environment, drop, copy, equality.
        }
    }

    /// Returns the alignment of this type, in bytes.
    pub fn alignment(&self) -> u64 {
        match self {
            Self::Int | Self::UInt | Self::Float | Self::Byte | Self::Bool | Self::Array(_) => {
                self.size()
            }
            Self::Fields(field_tys) => field_tys.iter().map(Self::alignment).max().unwrap_or(1),
            Self::Func(_, _) => Self::PTR_SIZE,
        }
    }
}

/// A parameter of a function type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    /// The type of the paremeter.
    pub ty: Ty,
    /// Whether the parameter is mutable.
    pub mutable: bool,
}
