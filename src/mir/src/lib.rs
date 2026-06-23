//! The high-level intermediate representation of Patina. Produced after name resolution, and used for typechecking.

use slotmap::{SecondaryMap, SlotMap, new_key_type};

use ident::Ident;

#[derive(Debug, Default)]
pub struct Mir {
    main: Option<Item>,
    items: Vec<Item>,
    exprs: SlotMap<ExprId, Expr>,
    // expr_tys: SecondaryMap<ExprId, Ty>,
    vars: SlotMap<VarId, VarInfo>,
}

impl Mir {
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    pub fn add_item(&mut self, item: Item) {
        self.items.push(item);
    }

    pub const fn main(&self) -> Option<&Item> {
        self.main.as_ref()
    }

    pub fn set_main(&mut self, main: Item) {
        self.main = Some(main);
    }
}

// Expr-related functions
impl Mir {
    pub fn add_expr(&mut self, expr: Expr) -> ExprId {
        let id = self.exprs.insert(expr);
        //self.expr_tys.insert(id, ty);
        id
    }

    pub fn expr(&self, id: ExprId) -> &Expr {
        &self.exprs[id]
    }
}

// Var-related functions
impl Mir {
    pub fn add_var(&mut self, info: VarInfo) -> VarId {
        self.vars.insert(info)
    }

    pub fn var(&self, id: VarId) -> &VarInfo {
        &self.vars[id]
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Item {
    pub var: VarId,
    pub kind: ItemKind,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ItemKind {
    Const(ExprId),
    Func { params: Vec<VarId>, body: ExprId },
}

new_key_type! { pub struct VarId; }
#[derive(Debug, Clone)]
pub struct VarInfo {
    pub ident: Ident,
    pub ty: Ty,
    pub mutable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stmt {
    Decl { var: VarId, val: ExprId },
    Expr(ExprId),
}

new_key_type! { pub struct ExprId; }
#[derive(Debug, Clone)]
pub enum Expr {
    Var(VarId),
    Lit(LitExpr),
    Array(Ty, Vec<ExprId>),
    Construct(Vec<Ty>, Vec<ExprId>),
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
        field: u32,
    },
    Index {
        array: ExprId,
        index: ExprId,
    },
    Call {
        func: ExprId,
        args: Vec<Arg>,
        ret_ty: Ty,
    },
    Closure {
        func: VarId,
        captures: Vec<VarId>,
    },
    Assign {
        place: ExprId,
        value: ExprId,
    },
    If {
        ty: Ty,
        cond: ExprId,
        th: BlockExpr,
        el: Option<BlockExpr>,
    },
    Loop(BlockExpr),
    Block(BlockExpr),

    Print(Ty, ExprId),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LitExpr {
    Int(i64),
    UInt(u64),
    Byte(u8),
    Float(f64),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arg {
    pub ty: Ty,
    pub value: ExprId,
    pub mutable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockExpr(pub Vec<Stmt>);

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

/// The kinds of types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Int,
    UInt,
    Byte,
    Float,
    Bool,
    Fields(Vec<Self>),
    Array(Box<Self>),
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
            Self::Fields(field_tys) => field_tys.iter().map(Ty::alignment).max().unwrap_or(0),
            Self::Func(_, _) => Self::PTR_SIZE,
        }
    }
}

/// A parameter of a function type
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub ty: Ty,
    pub mutable: bool,
}
