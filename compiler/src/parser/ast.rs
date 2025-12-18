pub type Ast = Vec<Item>;

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Const {
        ident: String,
        ty: Type,
        value: Expr,
    },
    Function {
        name: String,
        params: Vec<Binding>,
        return_type: Option<Type>,
        body: Expr,
    },
    Struct {
        name: Type,
        fields: Vec<Field>,
    },
    Enum {
        name: Type,
        variants: Vec<Variant>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Variant {
    Unit(String),
    Tuple(String, Vec<Type>),
    Struct(String, Vec<Field>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    pub mutable: bool,
    pub name: String,
    pub type_annotation: Option<Type>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type {
    pub name: String,
    pub generics: Vec<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Lit),
    Ident(String),
    FnCall {
        fun: Box<Expr>,
        args: Vec<Expr>,
    },
    BinaryOp {
        op: Bop,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    UnaryOp {
        op: Unop,
        expr: Box<Expr>,
    },
    If {
        cond: Box<Expr>,
        th: Box<Expr>,
        el: Option<Box<Expr>>,
    },
    Let {
        binding: Binding,
        value: Box<Expr>,
    },
    Lambda {
        params: Vec<Binding>,
        return_type: Option<Type>,
        body: Box<Expr>,
    },
    Block {
        exprs: Vec<Expr>,
        trailing: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Lit {
    Int(i64),
    Float(f64),
    Str(String),
    Char(char),
    Bool(bool),
    Array(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),
}

impl From<Lit> for Expr {
    fn from(value: Lit) -> Self {
        Expr::Literal(value)
    }
}

impl From<Lit> for Box<Expr> {
    fn from(value: Lit) -> Self {
        Box::new(Expr::Literal(value))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
    Assign,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Unop {
    Not,
    Neg,
}
