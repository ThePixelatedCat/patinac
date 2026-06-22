use crate::LowerInfo;

impl LowerInfo<'_, '_> {
    #[allow(
        clippy::too_many_lines,
        reason = "Any given arm is readable on it's own"
    )]
    pub fn lower_expr(&mut self, expr: hir::ExprId) -> mir::ExprId {
        let new_expr = match self.hir.expr(expr) {
            hir::Expr::Var(var) => mir::Expr::Var(self.lower_var(*var)),
            hir::Expr::Lit(lit) => mir::Expr::Lit(self.lower_lit(expr, lit)),
            hir::Expr::Array(elems) => {
                mir::Expr::Array(elems.iter().map(|&elem| self.lower_expr(elem)).collect())
            }
            hir::Expr::Tuple(elems) => {
                mir::Expr::Construct(elems.iter().map(|&elem| self.lower_expr(elem)).collect())
            }
            &hir::Expr::Infix { op, lhs, rhs } => mir::Expr::Infix {
                op: convert_infix_op(op),
                lhs: self.lower_expr(lhs),
                rhs: self.lower_expr(rhs),
            },
            &hir::Expr::Prefix { op, expr } => mir::Expr::Prefix {
                op: convert_prefix_op(op),
                expr: self.lower_expr(expr),
            },
            &hir::Expr::Field { base, field } => {
                let hir::Ty::Named(ty) = self.expr_ty(base) else {
                    unreachable!("field access of non-record type")
                };
                let base = self.lower_expr(base);
                let field = self.field_index(*ty, field.ident);
                mir::Expr::Field { base, field }
            }
            &hir::Expr::Index { array, index } => mir::Expr::Index {
                array: self.lower_expr(array),
                index: self.lower_expr(index),
            },
            hir::Expr::Call { func, args } => {
                let func = self.lower_expr(*func);
                let args = args
                    .iter()
                    .map(|arg| mir::Arg {
                        value: self.lower_expr(arg.value),
                        mutable: arg.mutable,
                    })
                    .collect();
                mir::Expr::Call { func, args }
            }
            hir::Expr::Lambda {
                params,
                body,
                captures,
            } => todo!(),
            &hir::Expr::Assign { place, value } => mir::Expr::Assign {
                place: self.lower_expr(place),
                value: self.lower_expr(value),
            },
            hir::Expr::If { cond, th, el } => mir::Expr::If {
                cond: self.lower_expr(*cond),
                th: self.lower_block_expr(th),
                el: el.as_ref().map(|el| self.lower_block_expr(el)),
            },
            hir::Expr::For { .. } => todo!("Traits"),
            hir::Expr::Loop(body) => mir::Expr::Loop(self.lower_block_expr(body)),
            hir::Expr::Break => todo!("Unconditional Control Flow"),
            hir::Expr::Continue => todo!("Unconditional Control Flow"),
            hir::Expr::Return(_) => todo!("Unconditional Control Flow"),
            hir::Expr::Block(block) => mir::Expr::Block(self.lower_block_expr(block)),

            hir::Expr::Print(expr) => mir::Expr::Print(self.lower_expr(*expr)),
        };
        let ty = self.lower_ty(self.expr_ty(expr));
        self.mir.add_expr(new_expr, ty)
    }

    fn lower_lit(&self, expr: hir::ExprId, lit: &hir::LitExpr) -> mir::LitExpr {
        match lit {
            &hir::LitExpr::Int(value) => match self.expr_ty(expr) {
                hir::Ty::Int => {
                    let i = i64::try_from(value).unwrap_or_else(|_| {
                        self.handler.warn(
                            &format!(
                                "int literal {value} overflowed and was clamped to {}",
                                i64::MAX
                            ),
                            self.hir.expr_span(expr),
                            self.module,
                        );
                        i64::MAX
                    });
                    mir::LitExpr::Int(i)
                }
                hir::Ty::UInt => mir::LitExpr::UInt(value),
                hir::Ty::Byte => {
                    let b = u8::try_from(value).unwrap_or_else(|_| {
                        self.handler.warn(
                            &format!(
                                "byte literal {value} overflowed and was clamped to {}",
                                u8::MAX
                            ),
                            self.hir.expr_span(expr),
                            self.module,
                        );
                        u8::MAX
                    });
                    mir::LitExpr::Byte(b)
                }
                _ => unreachable!("not an int type"),
            },
            &hir::LitExpr::Float(value) => mir::LitExpr::Float(value),
            hir::LitExpr::String(_) => todo!("Strings"),
            &hir::LitExpr::Bool(value) => mir::LitExpr::Bool(value),
        }
    }

    fn lower_block_expr(&mut self, block: &hir::BlockExpr) -> mir::BlockExpr {
        let stmts = block
            .stmts
            .iter()
            .map(|stmt| match stmt {
                hir::Stmt::Decl { id, val, .. } => mir::Stmt::Decl {
                    id: self.lower_var(*id),
                    val: self.lower_expr(*val),
                },
                hir::Stmt::Expr(expr) => mir::Stmt::Expr(self.lower_expr(*expr)),
            })
            .collect();
        mir::BlockExpr(stmts)
    }
}

macro_rules! convert_op {
    ($op:ident, $enum_name:ident, $($variant:ident),*) => {
        match $op {
            $(hir::$enum_name::$variant => mir::$enum_name::$variant),*
        }
    };
}

const fn convert_prefix_op(op: hir::PrefixOp) -> mir::PrefixOp {
    convert_op!(op, PrefixOp, Not, Neg)
}

const fn convert_infix_op(op: hir::InfixOp) -> mir::InfixOp {
    convert_op!(
        op, InfixOp, Add, AddF, Sub, SubF, Mul, MulF, Div, DivF, Exp, And, Or, Xor, Eqq, Neq, Gt,
        Lt, Geq, Leq
    )
}
