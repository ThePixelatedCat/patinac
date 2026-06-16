//! The high-level intermediate representation of Patina. Produced after name resolution, and used for typechecking.

use slotmap::{SecondaryMap, SlotMap, new_key_type};

use ident::Ident;

#[derive(Debug, Default)]
pub struct Mir {
    main: Option<Item>,
    items: Vec<Item>,
    exprs: SlotMap<ExprId, Expr>,
    expr_tys: SecondaryMap<ExprId, Ty>,
    vars: SlotMap<VarId, VarInfo>,
}

impl Mir {
    pub fn execs(&self) -> &[Item] {
        &self.items
    }

    pub fn add_exec(&mut self, item: Item) {
        self.items.push(item);
    }

    pub fn add_execs(&mut self, items: impl IntoIterator<Item = Item>) {
        self.items.extend(items);
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
    pub fn add_expr(&mut self, expr: Expr, ty: Ty) -> ExprId {
        let id = self.exprs.insert(expr);
        self.expr_tys.insert(id, ty);
        id
    }

    pub fn expr(&self, id: ExprId) -> &Expr {
        &self.exprs[id]
    }

    pub fn expr_ty(&self, id: ExprId) -> &Ty {
        &self.expr_tys[id]
    }
}

// Var-related functions
impl Mir {
    pub fn add_var(&mut self, ident: Ident, ty: Ty, mutable: bool) -> VarId {
        self.vars.insert(VarInfo { ident, ty, mutable })
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarInfo {
    pub ident: Ident,
    pub ty: Ty,
    pub mutable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stmt {
    Decl { id: VarId, val: ExprId },
    Expr(ExprId),
}

new_key_type! { pub struct ExprId; }
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Var(VarId),
    Lit(LitExpr),
    Array(Vec<ExprId>),
    Construct {
        field_tys: Vec<Ty>,
        field_values: Vec<ExprId>,
    },
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
    Loop(BlockExpr),
    Block(BlockExpr),

    Print(ExprId),
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

/// The kinds of types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ty {
    Int,
    UInt,
    Byte,
    Float,
    Bool,
    Fields(Vec<Self>),
    Array(Box<Self>),
    FuncPtr(Vec<Param>, Box<Self>),
    Closure(Vec<Param>, Box<Self>),
}

/// A parameter of a function type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Param {
    pub ty: Ty,
    pub mutable: bool,
}
