use hir::{
    exprs::{BlockExpr, Expr, ExprId, InfixOp, LitExpr, PrefixOp},
    types::Ty,
};
use inkwell::{
    FloatPredicate, basic_block,
    values::{AnyValue, AnyValueEnum, BasicValue, BasicValueEnum},
};

use crate::Codegen;

impl<'ctx> Codegen<'ctx, '_> {
    pub fn codegen_expr(&self, expr: ExprId) -> BasicValueEnum<'ctx> {
        match self.hir.expr_info(expr) {
            Expr::Ident(var_id) => todo!(),
            Expr::Lit(lit) => self.codegen_lit(expr, lit),
            Expr::Array(expr_ids) => {
                //self.ctx.
                todo!()
            }
            Expr::Tuple(exprs) => {
                //self.ctx.struct_type(field_types, packed).
                todo!()
            }
            Expr::Infix { op, lhs, rhs } => self.codegen_infix(*op, *lhs, *rhs),
            Expr::Prefix { op, expr } => self.codegen_prefix(*op, *expr),
            Expr::Field { base, field } => todo!(),
            Expr::Index { arr, idx } => todo!(),
            Expr::Call { func, args } => todo!(),
            Expr::Lambda { params, body } => todo!(),
            Expr::If { cond, th, el } => self.codegen_if(expr, *cond, th, el.as_ref()),
            Expr::For { id, iter, body } => todo!(),
            Expr::Loop(body) => self.codegen_loop(body),
            Expr::Break => todo!(),
            Expr::Continue => todo!(),
            Expr::Return(expr_id) => todo!(),
            Expr::Block(stmts) => todo!(),
        }
    }

    fn unit(&self) -> BasicValueEnum<'ctx> {
        self.ctx.const_struct(&[], false).as_basic_value_enum()
    }

    fn codegen_lit(&self, expr: ExprId, lit: &LitExpr) -> BasicValueEnum<'ctx> {
        match lit {
            LitExpr::Int(val) => match self.ty_map.get(expr) {
                Ty::Int => {
                    let max = i64::MAX as u64;
                    let clamped_val = if *val > max {
                        self.report_warning(format!(
                            "Int literal {val} overflowed and was clamped to {max}"
                        ));
                        max
                    } else {
                        *val
                    };
                    self.ctx.i64_type().const_int(clamped_val, false)
                }
                Ty::UInt => self.ctx.i64_type().const_int(*val, false),
                Ty::Byte => {
                    let max = u8::MAX as u64;
                    let clamped_val = if *val > max {
                        self.report_warning(format!(
                            "Byte literal {val} overflowed and was clamped to {max}"
                        ));
                        max
                    } else {
                        *val
                    };
                    self.ctx.i8_type().const_int(clamped_val, false)
                }
                _ => unreachable!("ICE: int literal inferred as non-int type"),
            }
            .as_basic_value_enum(),
            LitExpr::Float(val) => self.ctx.f64_type().const_float(*val).as_basic_value_enum(),
            LitExpr::Char(_) => todo!(),
            LitExpr::String(_) => todo!(),
            LitExpr::Bool(val) => {
                let bool_ty = self.ctx.bool_type();
                if *val {
                    bool_ty.const_all_ones()
                } else {
                    bool_ty.const_zero()
                }
                .as_basic_value_enum()
            }
        }
    }

    fn codegen_infix(&self, op: InfixOp, lhs: ExprId, rhs: ExprId) -> BasicValueEnum<'ctx> {
        let lhs = self.codegen_expr(lhs);
        let rhs = self.codegen_expr(rhs);

        match op {
            InfixOp::Assign => todo!(),
            InfixOp::Add => self
                .builder
                .build_int_add(lhs.into_int_value(), rhs.into_int_value(), "iaddtmp")
                .unwrap()
                .as_basic_value_enum(),
            InfixOp::AddF => self
                .builder
                .build_float_add(lhs.into_float_value(), rhs.into_float_value(), "faddtmp")
                .unwrap()
                .as_basic_value_enum(),
            InfixOp::Sub => self
                .builder
                .build_int_sub(lhs.into_int_value(), rhs.into_int_value(), "isubtmp")
                .unwrap()
                .as_basic_value_enum(),
            InfixOp::SubF => self
                .builder
                .build_float_sub(lhs.into_float_value(), rhs.into_float_value(), "fsubtmp")
                .unwrap()
                .as_basic_value_enum(),
            InfixOp::Mul => self
                .builder
                .build_int_mul(lhs.into_int_value(), rhs.into_int_value(), "imultmp")
                .unwrap()
                .as_basic_value_enum(),
            InfixOp::MulF => self
                .builder
                .build_float_mul(lhs.into_float_value(), rhs.into_float_value(), "fmultmp")
                .unwrap()
                .as_basic_value_enum(),
            InfixOp::Div => todo!(),
            InfixOp::DivF => todo!(),
            InfixOp::Exp => todo!(),
            InfixOp::Rem => todo!(),
            InfixOp::RemF => todo!(),
            InfixOp::And => todo!(),
            InfixOp::Or => todo!(),
            InfixOp::Xor => todo!(),
            InfixOp::Eqq => todo!(),
            InfixOp::Neq => todo!(),
            InfixOp::Gt => self
                .builder
                .build_float_compare(
                    FloatPredicate::UGT,
                    lhs.into_float_value(),
                    rhs.into_float_value(),
                    "gttmp",
                )
                .unwrap()
                .as_basic_value_enum(),
            InfixOp::Lt => self
                .builder
                .build_float_compare(
                    FloatPredicate::ULT,
                    lhs.into_float_value(),
                    rhs.into_float_value(),
                    "lttmp",
                )
                .unwrap()
                .as_basic_value_enum(),
            InfixOp::Geq => self
                .builder
                .build_float_compare(
                    FloatPredicate::UGE,
                    lhs.into_float_value(),
                    rhs.into_float_value(),
                    "geqtmp",
                )
                .unwrap()
                .as_basic_value_enum(),
            InfixOp::Leq => self
                .builder
                .build_float_compare(
                    FloatPredicate::ULE,
                    lhs.into_float_value(),
                    rhs.into_float_value(),
                    "leqtmp",
                )
                .unwrap()
                .as_basic_value_enum(),
        }
    }

    fn codegen_prefix(&self, op: PrefixOp, expr: ExprId) -> BasicValueEnum<'ctx> {
        let expr = self.codegen_expr(expr);

        match op {
            PrefixOp::Not => self
                .builder
                .build_not(expr.into_int_value(), "nottmp")
                .unwrap()
                .as_basic_value_enum(),
            PrefixOp::Neg => self
                .builder
                .build_float_neg(expr.into_float_value(), "fnegtmp")
                .unwrap()
                .as_basic_value_enum(),
        }
    }

    fn codegen_if(
        &self,
        expr: ExprId,
        cond: ExprId,
        th: &BlockExpr,
        el: Option<&BlockExpr>,
    ) -> BasicValueEnum<'ctx> {
        let cond = self.codegen_expr(cond);

        let function = self
            .builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap();

        let mut th_block = self.ctx.append_basic_block(function, "th");
        let mut el_block = self.ctx.append_basic_block(function, "el");
        let merge_block = self.ctx.append_basic_block(function, "merge");
        self.builder
            .build_conditional_branch(cond.into_int_value(), th_block, el_block)
            .unwrap();

        self.builder.position_at_end(th_block);
        let th = self.codegen_block_expr(th);
        self.builder
            .build_unconditional_branch(merge_block)
            .unwrap();
        th_block = self.builder.get_insert_block().unwrap();

        el_block.move_after(th_block).unwrap();
        self.builder.position_at_end(el_block);
        let el = self.codegen_block_expr(el.expect("TODO: no else"));
        self.builder
            .build_unconditional_branch(merge_block)
            .unwrap();
        el_block = self.builder.get_insert_block().unwrap();

        merge_block.move_after(el_block).unwrap();
        self.builder.position_at_end(merge_block);
        let phi = self
            .builder
            .build_phi(self.convert_ty(self.ty_map.get(expr)), "iftmp")
            .unwrap();
        phi.add_incoming(&[(&th, th_block), (&el, el_block)]);
        phi.as_basic_value()
    }

    fn codegen_loop(&self, body: &BlockExpr) -> BasicValueEnum<'ctx> {
        let pre_block = self.builder.get_insert_block().unwrap();
        let function = pre_block.get_parent().unwrap();
        let body_block = self.ctx.append_basic_block(function, "body");

        self.builder.build_unconditional_branch(body_block).unwrap();

        self.builder.position_at_end(body_block);
        let _ = self.codegen_block_expr(body);
        self.builder.build_unconditional_branch(pre_block).unwrap();

        self.unit()
    }

    fn codegen_block_expr(&self, block: &BlockExpr) -> BasicValueEnum<'ctx> {
        todo!()
    }
}
