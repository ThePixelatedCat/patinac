use std::iter;

use hir::{
    VarId,
    exprs::{Arg, BlockExpr, Expr, ExprId, InfixOp, LitExpr, PrefixOp, Stmt},
    types::Ty,
};
use ident::SpanIdent;
use inkwell::{
    AddressSpace, FloatPredicate,
    module::Linkage,
    types::BasicTypeEnum,
    values::{
        BasicMetadataValueEnum, BasicValue, BasicValueEnum, CallSiteValue, FunctionValue,
        PointerValue,
    },
};

use crate::Codegen;

impl<'ctx> Codegen<'ctx, '_> {
    pub fn emit_expr(&mut self, expr: ExprId) -> BasicValueEnum<'ctx> {
        match self.hir.expr_info(expr) {
            Expr::Ident(id) => self.emit_ident(*id),
            Expr::Lit(lit) => self.emit_lit(expr, lit),
            Expr::Array(expr_ids) => todo!("Arrays"),
            Expr::Tuple(exprs) => self.emit_tuple(self.ty_map.ty(expr), exprs),
            Expr::Infix { op, lhs, rhs } => self.emit_infix(*op, *lhs, *rhs),
            Expr::Prefix { op, expr } => self.emit_prefix(*op, *expr),
            Expr::Field { base, field } => self.emit_field(expr, *base, *field),
            Expr::Index { arr, idx } => todo!("Arrays"),
            Expr::Call { func, args } => self.emit_call(*func, args, self.ty_map.ty(expr)),
            Expr::Lambda {
                params,
                body,
                captures,
            } => self.emit_lambda(self.ty_map.ty(expr), params, *body, captures),
            Expr::If { cond, th, el } => self.emit_if(*cond, th, el.as_ref()),
            Expr::For { id, iter, body } => todo!(),
            Expr::Loop(body) => self.emit_loop(body),
            Expr::Break => todo!("Unconditional Control Flow"),
            Expr::Continue => todo!("Unconditional Control Flow"),
            Expr::Return(expr) => todo!("Unconditional Control Flow"),
            Expr::Block(stmts) => self.emit_block_expr(stmts),

            Expr::Print(expr) => self.emit_print(*expr),
        }
    }

    fn emit_print(&mut self, expr: ExprId) -> BasicValueEnum<'ctx> {
        let format_string = match self.ty_map.ty(expr) {
            Ty::Int => "%lld\n",
            Ty::UInt => "%llu\n",
            Ty::Byte => "%hhu\n",
            Ty::Float => "%f\n",
            Ty::Bool => "%hhd\n",
            Ty::Char => todo!("Strings"),
            Ty::Adt(_) => todo!(),
            Ty::Tuple(_) => todo!(),
            Ty::Array(_) => todo!("Arrays"),
            Ty::Fn(_, _) => todo!(),
        };

        let format_ptr = self
            .builder
            .build_global_string_ptr(format_string, "format_string")
            .unwrap()
            .as_pointer_value();

        let expr = self.emit_expr(expr);
        self.builder
            .build_call(self.printf(), &[format_ptr.into(), expr.into()], "print")
            .unwrap();

        self.unit()
    }

    fn emit_place(&self, expr: ExprId) -> PointerValue<'ctx> {
        match self.hir.expr_info(expr) {
            Expr::Ident(id) => self.vars[*id],
            Expr::Field { base, field } => {
                let Ty::Adt(id) = self.ty_map.ty(*base) else {
                    unreachable!("ICE")
                };
                let idx = self.hir.adt_info(*id).fields.get_idx(field.ident);
                self.builder
                    .build_struct_gep(self.structs[*id], self.emit_place(*base), idx, "fieldptr")
                    .unwrap()
            }
            Expr::Index { arr, idx } => todo!("Arrays"),
            Expr::Call { func, args } => todo!("Projections"),
            _ => unreachable!("ICE: Tried to use non-place expr as place"),
        }
    }

    fn unit(&self) -> BasicValueEnum<'ctx> {
        self.ctx.const_struct(&[], false).as_basic_value_enum()
    }

    fn emit_ident(&self, id: VarId) -> BasicValueEnum<'ctx> {
        let alloc = self.vars[id];
        let ty = self.hir.var_ty(id);

        if Self::is_indirect(ty) {
            let new_alloc =
                self.emit_alloca_entry(self.lower_ty(ty), &self.hir.var_info(id).ident.str());
            self.emit_copy(ty, alloc.as_basic_value_enum(), new_alloc);
            new_alloc.as_basic_value_enum()
        } else {
            self.builder
                .build_load(self.lower_ty(ty), alloc, &self.hir.var_info(id).ident.str())
                .unwrap()
        }
    }

    fn emit_lit(&self, expr: ExprId, lit: &LitExpr) -> BasicValueEnum<'ctx> {
        match lit {
            LitExpr::Int(val) => match self.ty_map.ty(expr) {
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
            LitExpr::Char(_) => todo!("Strings"),
            LitExpr::String(_) => todo!("Strings"),
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

    fn emit_tuple(&mut self, ty: &Ty, exprs: &[ExprId]) -> BasicValueEnum<'ctx> {
        // Fast-path explicit units
        if exprs.is_empty() {
            return self.unit();
        }

        let ty = self.lower_ty(ty);
        let out = self.emit_alloca_entry(ty, "tuple");
        for (idx, expr) in exprs.iter().enumerate() {
            let tmp = self.emit_expr(*expr);
            let ptr = self
                .builder
                .build_struct_gep(ty, out, u32::try_from(idx).unwrap(), &format!("field{idx}"))
                .unwrap();
            self.emit_move(self.ty_map.ty(*expr), tmp, ptr);
        }
        out.as_basic_value_enum()
    }

    fn emit_infix(&mut self, op: InfixOp, lhs: ExprId, rhs: ExprId) -> BasicValueEnum<'ctx> {
        let ty = self.ty_map.ty(lhs);
        match op {
            InfixOp::Assign => {
                let dst = self.emit_place(lhs);
                let tmp = self.emit_expr(rhs);
                // Drop the current value in the assigned-to variable
                self.emit_drop(ty, dst.as_basic_value_enum());
                // Move the temporary value into the variable
                self.emit_move(ty, tmp, dst);
                self.unit()
            }
            _ => {
                let lhs = self.emit_expr(lhs);
                let rhs = self.emit_expr(rhs);
                self.emit_math_infix(ty, op, lhs, rhs)
            }
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "Any given arm is readable on it's own"
    )]
    fn emit_math_infix(
        &self,
        ty: &Ty,
        op: InfixOp,
        lhs: BasicValueEnum<'ctx>,
        rhs: BasicValueEnum<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        match op {
            InfixOp::Assign => unreachable!("ICE: Should not be called when the op is assignment"),
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
            op @ (InfixOp::Eqq | InfixOp::Neq) => {
                let equals = self.emit_equals(ty, lhs, rhs);
                if op == InfixOp::Neq {
                    self.builder
                        .build_not(equals.into_int_value(), "not_equals")
                        .unwrap()
                        .as_basic_value_enum()
                } else {
                    equals
                }
            }
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

    fn emit_prefix(&mut self, op: PrefixOp, expr: ExprId) -> BasicValueEnum<'ctx> {
        let expr = self.emit_expr(expr);

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

    fn emit_field(&mut self, expr: ExprId, base: ExprId, field: SpanIdent) -> BasicValueEnum<'ctx> {
        let Ty::Adt(id) = self.ty_map.ty(base) else {
            unreachable!("ICE")
        };

        let base = self.emit_expr(base);

        let idx = self.hir.adt_info(*id).fields.get_idx(field.ident);
        let alloc = self
            .builder
            .build_struct_gep(
                self.structs[*id],
                base.into_pointer_value(),
                idx,
                "fieldptr",
            )
            .unwrap();

        let ty = self.ty_map.ty(expr);
        if Self::is_indirect(ty) {
            let new_alloc = self.emit_alloca_entry(self.lower_ty(ty), &field.ident.str());
            self.emit_copy(ty, alloc.as_basic_value_enum(), new_alloc);
            new_alloc.as_basic_value_enum()
        } else {
            self.builder
                .build_load(self.lower_ty(ty), alloc, &field.ident.str())
                .unwrap()
        }
    }

    fn emit_call(&mut self, func: ExprId, args: &[Arg], ret_ty: &Ty) -> BasicValueEnum<'ctx> {
        let (mut args, tmps): (Vec<_>, Vec<_>) = args
            .iter()
            .map(|a| {
                let tmp = if a.mutable {
                    self.emit_place(a.val).as_basic_value_enum()
                } else {
                    self.emit_expr(a.val)
                };
                (
                    BasicMetadataValueEnum::from(tmp),
                    (self.ty_map.ty(a.val), tmp),
                )
            })
            .collect();

        let result = if Self::is_indirect(ret_ty) {
            let ret_ptr = self
                .builder
                .build_alloca(self.lower_ty(ret_ty), "out")
                .unwrap()
                .as_basic_value_enum();
            args.insert(0, ret_ptr.into());
            self.emit_call_inner(func, args);
            ret_ptr
        } else {
            self.emit_call_inner(func, args)
                .try_as_basic_value()
                .unwrap_basic()
        };

        for (ty, val) in tmps {
            self.emit_drop(ty, val);
        }

        result
    }

    fn emit_call_inner(
        &mut self,
        func: ExprId,
        mut args: Vec<BasicMetadataValueEnum<'ctx>>,
    ) -> CallSiteValue<'ctx> {
        if let Expr::Ident(id) = self.hir.expr_info(func)
            && let Some(func) = self.funcs.get(*id)
        {
            self.builder.build_call(*func, &args, "call").unwrap()
        } else {
            let closure = self.emit_expr(func).into_pointer_value();
            let ty = self.closure_ty();

            let env_ptr = self
                .builder
                .build_struct_gep(ty, closure, 1, "envptr")
                .unwrap();
            let env = self
                .builder
                .build_load(self.ptr_ty(), env_ptr, "env")
                .unwrap();
            args.push(env.as_basic_value_enum().into());

            let Ty::Fn(params, ret_ty) = self.ty_map.ty(func) else {
                unreachable!()
            };
            let func_ty = self.build_func_ty(params, ret_ty, true);
            let func_ptr = self
                .builder
                .build_struct_gep(ty, closure, 0, "funcptr")
                .unwrap();
            let func = self
                .builder
                .build_load(self.ptr_ty(), func_ptr, "func")
                .unwrap();

            self.builder
                .build_indirect_call(func_ty, func.into_pointer_value(), &args, "call")
                .unwrap()
        }
    }

    fn emit_lambda(
        &mut self,
        ty: &Ty,
        params: &[VarId],
        body: ExprId,
        captures: &[VarId],
    ) -> BasicValueEnum<'ctx> {
        let func_name = format!("_lambda{}", self.lambda_counter);
        self.lambda_counter += 1;
        let Ty::Fn(param_tys, ret_ty) = ty else {
            unreachable!("ICE")
        };
        let func = self.module.add_function(
            &func_name,
            self.build_func_ty(param_tys, ret_ty, true),
            Some(Linkage::Private),
        );

        let (env, env_ty) = if captures.is_empty() {
            (
                self.ctx.ptr_type(AddressSpace::default()).const_null(),
                None,
            )
        } else {
            todo!("Build env")
        };

        self.emit_defunc_body(func, body, params, ret_ty, captures, env_ty);

        self.emit_closure(&func_name, func, captures, env, env_ty)
            .as_basic_value_enum()
    }

    fn emit_closure(
        &self,
        name: &str,
        func: FunctionValue<'ctx>,
        captures: &[VarId],
        env: PointerValue<'ctx>,
        env_ty: Option<BasicTypeEnum<'ctx>>,
    ) -> PointerValue<'ctx> {
        let closure_ty = self.closure_ty();
        let closure = self.emit_alloca_entry(closure_ty, "closure");

        let store_closure = |idx, val: PointerValue<'ctx>| {
            let ptr = self
                .builder
                .build_struct_gep(closure_ty, closure, idx, "fieldptr")
                .unwrap();
            self.builder.build_store(ptr, val).unwrap();
        };

        store_closure(0, func.as_global_value().as_pointer_value());
        store_closure(1, env);
        store_closure(
            2,
            self.closure_drop(name, captures, env_ty)
                .as_global_value()
                .as_pointer_value(),
        );
        store_closure(
            3,
            self.closure_copy(name, captures, env_ty)
                .as_global_value()
                .as_pointer_value(),
        );
        store_closure(
            4,
            self.closure_equals(name, captures, env_ty)
                .as_global_value()
                .as_pointer_value(),
        );
        closure
    }

    fn emit_defunc_body(
        &mut self,
        func: FunctionValue<'ctx>,
        body: ExprId,
        params: &[VarId],
        ret_ty: &Ty,
        captures: &[VarId],
        env_ty: Option<BasicTypeEnum<'ctx>>,
    ) {
        // Save the builder's current insertion block to restore at the end
        let old_insert_block = self.builder.get_insert_block().unwrap();

        let entry_block = self.ctx.append_basic_block(func, "entry");
        self.builder.position_at_end(entry_block);

        // Skip the first argument if it's an out-pointer
        let offset = if Self::is_indirect(ret_ty) { 1 } else { 0 };
        for (arg, param) in iter::zip(func.get_param_iter().skip(offset), params) {
            let ty = self.hir.var_ty(*param);
            if self.hir.var_info(*param).mutable || Self::is_indirect(ty) {
                self.vars.insert(*param, arg.into_pointer_value());
            } else {
                let ptr = self.emit_alloca(arg.get_type(), &self.hir.var_info(*param).ident.str());
                self.builder.build_store(ptr, arg).unwrap();
                self.vars.insert(*param, ptr);
            }
        }

        // Set up the captures
        let mut overwritten_vars = Vec::new();
        if let Some(env_ty) = env_ty {
            let env = func.get_last_param().unwrap().into_pointer_value();
            for (idx, capture) in captures.iter().enumerate() {
                let capture_ptr = self
                    .builder
                    .build_struct_gep(env_ty, env, u32::try_from(idx).unwrap(), "captureptr")
                    .unwrap();
                if let Some(old_ptr) = self.vars.insert(*capture, capture_ptr) {
                    overwritten_vars.push((*capture, old_ptr));
                }
            }
        }

        let body = self.emit_expr(body);
        if Self::is_indirect(ret_ty) {
            let out_ptr = func.get_first_param().unwrap().into_pointer_value();
            self.emit_move(ret_ty, body, out_ptr);
            self.builder.build_return(None).unwrap();
        } else {
            self.builder.build_return(Some(&body)).unwrap();
        }

        assert!(func.verify(true));

        for (id, ptr) in overwritten_vars {
            self.vars.insert(id, ptr);
        }
        self.builder.position_at_end(old_insert_block);
    }

    fn emit_if(
        &mut self,
        cond: ExprId,
        th: &BlockExpr,
        el: Option<&BlockExpr>,
    ) -> BasicValueEnum<'ctx> {
        match el {
            Some(el) => self.emit_if_else(cond, th, el),
            None => self.emit_if_no_else(cond, th),
        }
    }

    fn emit_if_else(
        &mut self,
        cond: ExprId,
        th: &BlockExpr,
        el: &BlockExpr,
    ) -> BasicValueEnum<'ctx> {
        let cond = self.emit_expr(cond);

        let function = self.curr_function();

        let mut th_block = self.ctx.append_basic_block(function, "th");
        let mut el_block = self.ctx.append_basic_block(function, "el");
        let merge_block = self.ctx.append_basic_block(function, "merge");
        self.builder
            .build_conditional_branch(cond.into_int_value(), th_block, el_block)
            .unwrap();

        self.builder.position_at_end(th_block);
        let th = self.emit_block_expr(th);
        self.builder
            .build_unconditional_branch(merge_block)
            .unwrap();
        th_block = self.builder.get_insert_block().unwrap();

        el_block
            .move_after(function.get_last_basic_block().unwrap())
            .unwrap();
        self.builder.position_at_end(el_block);
        let el = self.emit_block_expr(el);
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

    fn emit_if_no_else(&mut self, cond: ExprId, th: &BlockExpr) -> BasicValueEnum<'ctx> {
        let cond = self.emit_expr(cond);

        let function = self.curr_function();

        let th_block = self.ctx.append_basic_block(function, "th");
        let merge_block = self.ctx.append_basic_block(function, "merge");
        self.builder
            .build_conditional_branch(cond.into_int_value(), th_block, merge_block)
            .unwrap();

        self.builder.position_at_end(th_block);
        let _ = self.emit_block_expr(th);
        self.builder
            .build_unconditional_branch(merge_block)
            .unwrap();

        merge_block
            .move_after(function.get_last_basic_block().unwrap())
            .unwrap();
        self.builder.position_at_end(merge_block);
        self.unit()
    }

    fn emit_loop(&mut self, body: &BlockExpr) -> BasicValueEnum<'ctx> {
        let function = self.curr_function();

        let body_block = self.ctx.append_basic_block(function, "body");
        self.builder.build_unconditional_branch(body_block).unwrap();

        self.builder.position_at_end(body_block);
        let _ = self.emit_block_expr(body);
        self.builder.build_unconditional_branch(body_block).unwrap();

        let post_block = self.ctx.append_basic_block(function, "post");
        self.builder.position_at_end(post_block);

        self.unit()
    }

    fn emit_block_expr(&mut self, block: &BlockExpr) -> BasicValueEnum<'ctx> {
        let mut tmps: Vec<_> = block
            .stmts
            .iter()
            .map(|stmt| match stmt {
                Stmt::Decl { id, val, .. } => {
                    let ty = self.hir.var_ty(*id);
                    let ptr = self
                        .emit_alloca_entry(self.lower_ty(ty), &self.hir.var_info(*id).ident.str());
                    self.vars.insert(*id, ptr);

                    let val_tmp = self.emit_expr(*val);
                    self.emit_move(ty, val_tmp, ptr);

                    (ty, ptr.as_basic_value_enum())
                }
                Stmt::Expr(expr) => (self.ty_map.ty(*expr), self.emit_expr(*expr)),
            })
            .collect();
        let result = tmps.pop().map_or_else(|| self.unit(), |v| v.1);
        for (ty, val) in tmps {
            self.emit_drop(ty, val);
        }
        result
    }
}
