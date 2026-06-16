use package::ModuleId;

use crate::LowerInfo;

impl LowerInfo<'_> {
    #[allow(
        clippy::too_many_lines,
        reason = "Any given arm is readable on it's own"
    )]
    pub fn lower_expr(&mut self, module: ModuleId, expr: hir::ExprId) -> mir::ExprId {
        let new_expr = match self.hir.take_expr(expr) {
            hir::Expr::Var(var) => mir::Expr::Var(self.var_map[var]),
            hir::Expr::Lit(lit) => mir::Expr::Lit(self.lower_lit(module, expr, lit)),
            hir::Expr::Array(elems) => mir::Expr::Array(self.lower_exprs(module, &elems)),
            hir::Expr::Tuple(elems) => {
                // FIXME: Reorder optimally.
                let (field_tys, field_values) = elems
                    .into_iter()
                    .map(|elem| {
                        (
                            self.lower_ty(&self.expr_tys[elem]),
                            self.lower_expr(module, elem),
                        )
                    })
                    .unzip();
                mir::Expr::Construct {
                    field_tys,
                    field_values,
                }
            }
            hir::Expr::Infix { op, lhs, rhs } => mir::Expr::Infix {
                op: convert_infix_op(op),
                lhs: self.lower_expr(module, lhs),
                rhs: self.lower_expr(module, rhs),
            },
            hir::Expr::Prefix { op, expr } => mir::Expr::Prefix {
                op: convert_prefix_op(op),
                expr: self.lower_expr(module, expr),
            },
            hir::Expr::Field { base, field } => todo!(),
            hir::Expr::Index { array, index } => mir::Expr::Index {
                array: self.lower_expr(module, array),
                index: self.lower_expr(module, index),
            },
            hir::Expr::Call { func, args } => {
                let func = self.lower_expr(module, func);
                let args = args
                    .into_iter()
                    .map(|arg| mir::Arg {
                        value: self.lower_expr(module, arg.value),
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
            hir::Expr::Assign { place, value } => mir::Expr::Assign {
                place: self.lower_expr(module, place),
                value: self.lower_expr(module, value),
            },
            hir::Expr::If { cond, th, el } => todo!(),
            hir::Expr::For { .. } => todo!("Traits"),
            hir::Expr::Loop(body) => todo!(),
            hir::Expr::Break => todo!("Unconditional Control Flow"),
            hir::Expr::Continue => todo!("Unconditional Control Flow"),
            hir::Expr::Return(_) => todo!("Unconditional Control Flow"),
            hir::Expr::Block(block) => todo!(),

            hir::Expr::Print(expr) => mir::Expr::Print(self.lower_expr(module, expr)),
        };
        self.mir
            .add_expr(new_expr, self.lower_ty(&self.expr_tys[expr]))
    }

    fn lower_exprs<V: FromIterator<mir::ExprId>>(
        &mut self,
        module: ModuleId,
        exprs: &[hir::ExprId],
    ) -> V {
        exprs
            .iter()
            .map(|expr| self.lower_expr(module, *expr))
            .collect()
    }

    fn lower_lit(&self, module: ModuleId, expr: hir::ExprId, lit: hir::LitExpr) -> mir::LitExpr {
        match lit {
            hir::LitExpr::Int(value) => match self.expr_tys[expr] {
                hir::Ty::Int => {
                    let i = i64::try_from(value).unwrap_or_else(|_| {
                        self.handler.warn(
                            &format!(
                                "int literal {value} overflowed and was clamped to {}",
                                i64::MAX
                            ),
                            self.hir.expr_span(expr),
                            module,
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
                            module,
                        );
                        u8::MAX
                    });
                    mir::LitExpr::Byte(b)
                }
                _ => unreachable!("not an int type"),
            },
            hir::LitExpr::Float(value) => mir::LitExpr::Float(value),
            hir::LitExpr::Char(_) => todo!("Strings"),
            hir::LitExpr::String(_) => todo!("Strings"),
            hir::LitExpr::Bool(value) => mir::LitExpr::Bool(value),
        }
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
