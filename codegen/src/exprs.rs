use hir::{
    VarId,
    exprs::{Arg, BlockExpr, Expr, ExprId, InfixOp, LitExpr, PrefixOp, Stmt},
    types::Ty,
};
use ident::SpanIdent;
use inkwell::{
    FloatPredicate,
    types::BasicTypeEnum,
    values::{BasicValue, BasicValueEnum, PointerValue},
};

use crate::Codegen;

impl<'ctx> Codegen<'ctx, '_> {
    #[allow(unused)]
    pub fn codegen_expr(&mut self, expr: ExprId) -> BasicValueEnum<'ctx> {
        match self.hir.expr_info(expr) {
            Expr::Ident(id) => self.codegen_ident(*id),
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
            Expr::Field { base, field } => self.codegen_field(*base, *field),
            Expr::Index { arr, idx } => todo!(),
            Expr::Call { func, args } => self.codegen_call(*func, args),
            Expr::Lambda { params, body } => todo!(),
            Expr::If { cond, th, el } => self.codegen_if(*cond, th, el.as_ref()),
            Expr::For { id, iter, body } => todo!(),
            Expr::Loop(body) => self.codegen_loop(body),
            Expr::Break => todo!(),
            Expr::Continue => todo!(),
            Expr::Return(expr) => todo!(),
            Expr::Block(stmts) => self.codegen_block_expr(stmts),
        }
    }

    #[allow(unused)]
    fn codegen_place_expr(&mut self, expr: ExprId) -> BasicValueEnum<'ctx> {
        let ptr: PointerValue = match self.hir.expr_info(expr) {
            Expr::Ident(id) => self.vars[*id].ptr,
            Expr::Field { base, field } => {
                let Ty::Adt(id) = self.ty_map.expr_ty(*base) else {
                    unreachable!("ICE")
                };
                let ty = self.structs[*id];
                let idx = self.hir.adt_info(*id).fields.get_idx(field.ident);

                let base = self.codegen_expr(*base);
                self.builder
                    .build_struct_gep(ty, base.into_pointer_value(), idx, "geptmp")
                    .unwrap()
            }
            Expr::Index { arr, idx } => todo!(),
            Expr::Call { func, args } => todo!("Projections"),
            _ => unreachable!("ICE: Tried to codegen non-place expr as place expr"),
        };
        ptr.as_basic_value_enum()
    }

    fn unit(&self) -> BasicValueEnum<'ctx> {
        self.ctx.const_struct(&[], false).as_basic_value_enum()
    }

    fn codegen_ident(&self, id: VarId) -> BasicValueEnum<'ctx> {
        let alloc = self.vars[id];
        self.builder
            .build_load(
                BasicTypeEnum::try_from(alloc.ty).unwrap(),
                alloc.ptr,
                &self.hir.var_ident(id).ident.to_string(),
            )
            .unwrap()
    }

    fn codegen_lit(&self, expr: ExprId, lit: &LitExpr) -> BasicValueEnum<'ctx> {
        match lit {
            LitExpr::Int(val) => match self.ty_map.expr_ty(expr) {
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
                    let max = u64::from(u8::MAX);
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

    fn codegen_infix(&mut self, op: InfixOp, lhs: ExprId, rhs: ExprId) -> BasicValueEnum<'ctx> {
        let lhs = match op {
            InfixOp::Assign => self.codegen_place_expr(lhs),
            _ => self.codegen_expr(lhs),
        };
        let rhs = self.codegen_expr(rhs);

        match op {
            InfixOp::Assign => {
                self.builder
                    .build_store(lhs.into_pointer_value(), rhs)
                    .unwrap();
                self.unit()
            }
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
            InfixOp::DivF => self
                .builder
                .build_float_div(lhs.into_float_value(), rhs.into_float_value(), "fdivtmp")
                .unwrap()
                .as_basic_value_enum(),
            InfixOp::Exp => todo!(),
            InfixOp::And => self
                .builder
                .build_and(lhs.into_int_value(), rhs.into_int_value(), "andtmp")
                .unwrap()
                .as_basic_value_enum(),
            InfixOp::Or => self
                .builder
                .build_or(lhs.into_int_value(), rhs.into_int_value(), "ortmp")
                .unwrap()
                .as_basic_value_enum(),
            InfixOp::Xor => self
                .builder
                .build_xor(lhs.into_int_value(), rhs.into_int_value(), "xortmp")
                .unwrap()
                .as_basic_value_enum(),
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

    fn codegen_prefix(&mut self, op: PrefixOp, expr: ExprId) -> BasicValueEnum<'ctx> {
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

    fn codegen_field(&mut self, base: ExprId, field: SpanIdent) -> BasicValueEnum<'ctx> {
        let Ty::Adt(id) = self.ty_map.expr_ty(base) else {
            unreachable!("ICE")
        };
        let ty = self.structs[*id];
        let idx = self.hir.adt_info(*id).fields.get_idx(field.ident);

        let base = self.codegen_expr(base);
        let field_ptr = self
            .builder
            .build_struct_gep(ty, base.into_pointer_value(), idx, "geptmp")
            .unwrap();
        self.builder
            .build_load(
                ty.get_field_type_at_index(idx).unwrap(),
                field_ptr,
                "fieldtmp",
            )
            .unwrap()
    }

    fn codegen_call(&mut self, func: ExprId, args: &[Arg]) -> BasicValueEnum<'ctx> {
        let func = if let Expr::Ident(id) = self.hir.expr_info(func)
            && let Some(func) = self.funcs.get(*id)
        {
            *func
        } else {
            todo!("Closures")
        };

        let args: Vec<_> = args
            .iter()
            .map(|a| {
                if a.mutable {
                    self.codegen_place_expr(a.val)
                } else {
                    self.codegen_expr(a.val)
                }
                .into()
            })
            .collect();

        self.builder
            .build_call(func, &args, "calltmp")
            .unwrap()
            .try_as_basic_value()
            .unwrap_basic()
    }

    fn codegen_if(
        &mut self,
        cond: ExprId,
        th: &BlockExpr,
        el: Option<&BlockExpr>,
    ) -> BasicValueEnum<'ctx> {
        match el {
            Some(el) => self.codegen_if_else(cond, th, el),
            None => self.codegen_if_no_else(cond, th),
        }
    }

    fn codegen_if_else(
        &mut self,
        cond: ExprId,
        th: &BlockExpr,
        el: &BlockExpr,
    ) -> BasicValueEnum<'ctx> {
        let cond = self.codegen_expr(cond);

        let function = self.curr_function();

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

        el_block
            .move_after(function.get_last_basic_block().unwrap())
            .unwrap();
        self.builder.position_at_end(el_block);
        let el = self.codegen_block_expr(el);
        self.builder
            .build_unconditional_branch(merge_block)
            .unwrap();
        el_block = self.builder.get_insert_block().unwrap();

        merge_block
            .move_after(function.get_last_basic_block().unwrap())
            .unwrap();
        self.builder.position_at_end(merge_block);
        let phi = self.builder.build_phi(th.get_type(), "iftmp").unwrap();
        phi.add_incoming(&[(&th, th_block), (&el, el_block)]);
        phi.as_basic_value()
    }

    fn codegen_if_no_else(&mut self, cond: ExprId, th: &BlockExpr) -> BasicValueEnum<'ctx> {
        let cond = self.codegen_expr(cond);

        let function = self.curr_function();

        let th_block = self.ctx.append_basic_block(function, "th");
        let merge_block = self.ctx.append_basic_block(function, "merge");
        self.builder
            .build_conditional_branch(cond.into_int_value(), th_block, merge_block)
            .unwrap();

        self.builder.position_at_end(th_block);
        let _ = self.codegen_block_expr(th);
        self.builder
            .build_unconditional_branch(merge_block)
            .unwrap();

        merge_block
            .move_after(function.get_last_basic_block().unwrap())
            .unwrap();
        self.builder.position_at_end(merge_block);
        self.unit()
    }

    fn codegen_loop(&mut self, body: &BlockExpr) -> BasicValueEnum<'ctx> {
        let function = self
            .builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap();

        let body_block = self.ctx.append_basic_block(function, "body");
        self.builder.build_unconditional_branch(body_block).unwrap();

        self.builder.position_at_end(body_block);
        let _ = self.codegen_block_expr(body);
        self.builder.build_unconditional_branch(body_block).unwrap();

        let post_block = self.ctx.append_basic_block(function, "post");
        self.builder.position_at_end(post_block);

        self.unit()
    }

    fn codegen_block_expr(&mut self, block: &BlockExpr) -> BasicValueEnum<'ctx> {
        block
            .stmts
            .iter()
            .map(|s| self.codegen_stmt(s))
            .last()
            .unwrap_or_else(|| self.unit())
    }

    fn codegen_stmt(&mut self, stmt: &Stmt) -> BasicValueEnum<'ctx> {
        match stmt {
            Stmt::Decl { id, val, .. } => {
                let ty = self.convert_ty(self.ty_map.var_ty(*id));
                let name = self.hir.var_ident(*id).ident.to_string();
                let alloc = self.alloca(ty, &name);
                self.vars.insert(*id, alloc);

                let val = self.codegen_expr(*val);
                self.builder.build_store(alloc.ptr, val).unwrap();

                self.unit()
            }
            Stmt::Expr(expr) => self.codegen_expr(*expr),
        }
    }
}
