use std::iter;

use cranelift::{
    codegen::ir::{BlockArg, InstBuilder, StackSlotData, StackSlotKind, Value, condcodes::FloatCC, types},
    frontend::FunctionBuilder,
};
use hir::{
    VarId,
    exprs::{Arg, BlockExpr, Expr, ExprId, InfixOp, LitExpr, PrefixOp, Stmt},
    types::Ty,
};
use ident::SpanIdent;

use crate::{Codegen, VirtualValue};

impl Codegen<'_, '_,> {
    pub(crate) fn emit_expr(&self, builder: &mut FunctionBuilder, expr: ExprId) -> VirtualValue {
        match self.hir.expr_info(expr) {
            Expr::Ident(id) => self.emit_ident(builder, *id),
            Expr::Lit(lit) => self.emit_lit(builder, expr, lit),
            Expr::Array(exprs) => self.emit_array(builder, self.ty_map.ty(expr), exprs),
            Expr::Tuple(exprs) => self.emit_tuple(builder, self.ty_map.ty(expr), exprs),
            Expr::Infix { op, lhs, rhs } => self.emit_infix(builder, *op, *lhs, *rhs),
            Expr::Prefix { op, expr } => self.emit_prefix(builder, *op, *expr),
            Expr::Field { base, field } => self.emit_field(builder, *base, *field),
            Expr::Index { arr, idx } => self.emit_index(builder, *arr, *idx),
            Expr::Call { func, args } => self.emit_call(builder, *func, args, self.ty_map.ty(expr)),
            Expr::Lambda {
                params,
                body,
                captures,
            } => self.emit_lambda(builder, self.ty_map.ty(expr), params, *body, captures),
            Expr::If { cond, th, el } => self.emit_if(builder, self.ty_map.ty(expr), *cond, th, el.as_ref()),
            Expr::For { .. } => todo!(),
            Expr::Loop(body) => self.emit_loop(builder, body),
            Expr::Break => todo!("Unconditional Control Flow"),
            Expr::Continue => todo!("Unconditional Control Flow"),
            Expr::Return(_) => todo!("Unconditional Control Flow"),
            Expr::Block(stmts) => self.emit_block_expr(builder, stmts),

            Expr::Print(expr) => self.emit_print(builder, *expr),
        }
    }

    fn emit_print(&self, builder: &mut FunctionBuilder, expr: ExprId) -> VirtualValue {
        let format_string = match self.ty_map.ty(expr) {
            Ty::Int => "%lld\n",
            Ty::UInt => "%llu\n",
            Ty::Byte => "%hhu\n",
            Ty::Float => "%f\n",
            Ty::Bool => "%hhd\n",
            Ty::Char => todo!("Strings"),
            Ty::Named(_) => todo!(),
            Ty::Tuple(_) => todo!(),
            Ty::Array(_) => todo!(),
            Ty::Fn(_, _) => todo!(),
        };

        let format_ptr = self
            .builder
            .build_global_string_ptr(format_string, "format_string")
            .unwrap()
            .as_pointer_value();

        let expr = self.emit_expr(builder, expr);
        let printf = builder.import_function(data)
        builder
            .ins()
            .call(self.printf(), &[format_ptr.into(), expr.into()]);

        self.unit()
    }

    fn emit_place(&self, builder: &mut FunctionBuilder, expr: ExprId) -> PointerValue<'ctx> {
        match self.hir.expr_info(expr) {
            Expr::Ident(id) => self.vars[*id],
            Expr::Field { base, field } => {
                let Ty::Named(id) = self.ty_map.ty(*base) else {
                    unreachable!("ICE")
                };
                let base = self.emit_place(*base);
                let (idx, _) = self.hir.ty_info(*id).fields.get_ty_idx(field.ident);
                builder
                    .ins()
                    .build_struct_gep(self.structs[*id], base, idx, "fieldptr")
                    .unwrap()
            }
            Expr::Index { arr, idx } => {
                let Ty::Array(elem_ty) = self.ty_map.ty(*arr) else {
                    unreachable!("ICE")
                };
                let arr = self.emit_place(*arr);
                let idx = self.emit_expr(*idx);
                builder
                    .ins()
                    .build_call(self.array_bounds_check(), &[arr.into(), idx.into()], "")
                    .unwrap();
                unsafe {
                    builder
                        .ins()
                        .build_in_bounds_gep(
                            self.lower_ty(elem_ty),
                            self.get_array_payload(arr),
                            &[idx.into_int_value()],
                            "elemptr",
                        )
                        .unwrap()
                }
            }
            Expr::Call { .. } => todo!("Projections"),
            _ => unreachable!("ICE: Tried to use non-place expr as place"),
        }
    }

    fn emit_unique_place(&self, builder: &mut FunctionBuilder, expr: ExprId) -> PointerValue<'ctx> {
        match self.hir.expr_info(expr) {
            Expr::Ident(id) => self.vars[*id],
            Expr::Field { base, field } => {
                let Ty::Named(id) = self.ty_map.ty(*base) else {
                    unreachable!("ICE")
                };
                let base = self.emit_unique_place(*base);
                let (idx, _) = self.hir.ty_info(*id).fields.get_ty_idx(field.ident);
                builder
                    .ins()
                    .build_struct_gep(self.structs[*id], base, idx, "fieldptr")
                    .unwrap()
            }
            Expr::Index { arr, idx } => {
                let ty = self.ty_map.ty(*arr);
                let elem_ty = self.ty_map.ty(expr);
                let arr = self.emit_unique_place(*arr);
                let idx = self.emit_expr(*idx);
                builder
                    .ins()
                    .build_call(self.array_bounds_check(), &[arr.into(), idx.into()], "")
                    .unwrap();
                builder
                    .ins()
                    .build_call(self.array_unique(ty, elem_ty), &[arr.into()], "")
                    .unwrap();
                unsafe {
                    builder
                        .ins()
                        .build_in_bounds_gep(
                            self.lower_ty(elem_ty),
                            self.get_array_payload(arr),
                            &[idx.into_int_value()],
                            "elemptr",
                        )
                        .unwrap()
                }
            }
            Expr::Call { .. } => todo!("Projections"),
            _ => unreachable!("ICE: Tried to use non-place expr as place"),
        }
    }

    fn unit(&self) -> VirtualValue {
        self.ctx.const_struct(&[], false).as_basic_value_enum()
    }

    fn emit_ident(&self, builder: &mut FunctionBuilder, id: VarId) -> VirtualValue {
        // If it's the name of a top-level function, convert it into a closure
        if let Some(func) = self.funcs.get(id) {
            return self
                .emit_closure(
                    &self.hir.var_info(id).ident.str(),
                    *func,
                    &[],
                    self.null_ptr(),
                    None,
                )
                .as_basic_value_enum();
        }

        let var = self.vars[id];
        let ty = self.hir.var_ty(id);

        if Self::is_indirect(ty) {
            let new_alloc =
                self.emit_alloca_entry(self.lower_ty(ty), &self.hir.var_info(id).ident.str());
            self.emit_copy(ty, alloc.as_basic_value_enum(), new_alloc);
            new_alloc.as_basic_value_enum()
        } else {
            match var {
                VirtualValue::Direct(value) => value,
                VirtualValue::Indirect(variable) => builder.use_var(variable),
            }
        }
    }

    fn emit_lit(&self, builder: &mut FunctionBuilder, expr: ExprId, lit: &LitExpr) -> VirtualValue {
        let val = match lit {
            LitExpr::Int(val) => match self.ty_map.ty(expr) {
                Ty::Int => {
                    let val = match i64::try_from(*val) {
                        Ok(val) => val,
                        Err(_) => {
                            self.handler.warn(
                                &format!("int literal {val} overflowed and was clamped"),
                                self.hir.expr_span(expr),
                            );
                            i64::MAX
                        },
                    };
                    builder.ins().iconst(types::I64, val)
                }
                Ty::UInt => builder.ins().iconst(types::I64, *val),
                Ty::Byte => {
                    let val = match u8::try_from(*val) {
                        Ok(val) => val,
                        Err(_) => {
                            self.handler.warn(
                                &format!("int literal {val} overflowed and was clamped"),
                                self.hir.expr_span(expr),
                            );
                            u8::MAX
                        },
                    };
                    builder.ins().iconst(types::I8, i64::from(val))
                }
                _ => unreachable!("ICE: int literal inferred as non-int type"),
            },
            LitExpr::Float(val) => builder.ins().f64const(*val),
            LitExpr::Char(_) => todo!("Strings"),
            LitExpr::String(_) => todo!("Strings"),
            LitExpr::Bool(val) => {
                let val = if *val { 1 } else { 0 };
                builder.ins().iconst(types::I8, val)
            }
        };
        VirtualValue::Direct(val)
    }

    fn emit_array(&self, builder: &mut FunctionBuilder, ty: &Ty, exprs: &[ExprId]) -> VirtualValue {
        let Ty::Array(elem_ty) = ty else {
            unreachable!("ICE")
        };

        // Allocate the array.
        let alloc = self.emit_alloca_entry(self.array_ty(), "array");
        let count = builder.ins().iconst(types::I64, i64::try_from(exprs.len()).unwrap());
        let ret = builder
            .ins()
            .call(
                self.array_init(ty, elem_ty),
                &[
                    alloc.into(),
                    count
                ],
            );
        let array = builder.inst_results(ret)[0];

        // Initialize each element.
        let payload = self.get_array_payload(alloc);
        let lowered_elem_ty = self.lower_ty(elem_ty);
        for (idx, expr) in exprs.iter().enumerate() {
            let idx = self.ctx.i64_type().const_int(
                u64::try_from(idx).expect("I doubt we'll see 128bit CPUs any time soon"),
                false,
            );
            let ptr = unsafe {
                builder
                    .ins()
                    .build_in_bounds_gep(lowered_elem_ty, payload, &[idx], "ptr")
                    .unwrap()
            };
            let elem = self.emit_expr(*expr);
            self.emit_move(elem_ty, elem, ptr);
        }

        alloc.as_basic_value_enum()
    }

    fn emit_tuple(&self, builder: &mut FunctionBuilder, ty: &Ty, exprs: &[ExprId]) -> VirtualValue {
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

    fn emit_infix(
        &self,
        builder: &mut FunctionBuilder,
        op: InfixOp,
        lhs: ExprId,
        rhs: ExprId,
    ) -> VirtualValue {
        let ty = self.ty_map.ty(lhs);
        match op {
            InfixOp::Assign => {
                let dst = self.emit_unique_place(lhs);
                let tmp = self.emit_expr(rhs);
                // Drop the current value in the assigned-to variable
                self.emit_drop(ty, dst.as_basic_value_enum());
                // Move the temporary value into the variable
                self.emit_move(ty, tmp, dst);
                self.unit()
            }
            _ => {
                let lhs = self.emit_expr(builder, lhs).get_val(builder);
                let rhs = self.emit_expr(builder, rhs).get_val(builder);
                self.emit_math_infix(builder, ty, op, lhs, rhs)
            }
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "Any given arm is readable on it's own"
    )]
    fn emit_math_infix(&self, builder: &mut FunctionBuilder, ty: &Ty, op: InfixOp, lhs: Value, rhs: Value) -> VirtualValue {
        let val = match op {
            InfixOp::Assign => unreachable!("ICE: Should not be called when the op is assignment"),
            InfixOp::Add => builder.ins().iadd(lhs, rhs),
            InfixOp::AddF => builder.ins().fadd(lhs, rhs),
            InfixOp::Sub => builder.ins().isub(lhs, rhs),
            InfixOp::SubF => builder.ins().fsub(lhs, rhs),
            InfixOp::Mul => builder.ins().imul(lhs, rhs),
            InfixOp::MulF => builder.ins().fmul(lhs, rhs),
            InfixOp::Div => {
                match ty {
                    Ty::Int => builder.ins().sdiv(lhs, rhs),
                    Ty::UInt => builder.ins().udiv(lhs, rhs),
                    _ => unreachable!()
                }
            },
            InfixOp::DivF => builder.ins().fdiv(lhs, rhs),
            InfixOp::Exp => todo!(),
            InfixOp::And => todo!("Short-circuiting operators"),
            InfixOp::Or => todo!("Short-circuiting operators"),
            InfixOp::Xor => builder.ins().bxor(lhs, rhs),
            InfixOp::Eqq | InfixOp::Neq => {
                let equals = self.emit_equals(ty, lhs, rhs);
                if op == InfixOp::Neq {
                    builder
                        .ins()
                        .bnot(equals)
                } else {
                    equals
                }
            }
            InfixOp::Gt => builder.ins().fcmp(FloatCC::GreaterThan, lhs, rhs),
            InfixOp::Lt => builder.ins().fcmp(FloatCC::LessThan, lhs, rhs),
            InfixOp::Geq => builder.ins().fcmp(FloatCC::GreaterThanOrEqual, lhs, rhs),
            InfixOp::Leq => builder.ins().fcmp(FloatCC::LessThanOrEqual, lhs, rhs),
        };
        VirtualValue::Direct(val)
    }

    fn emit_prefix(&self, builder: &mut FunctionBuilder, op: PrefixOp, expr: ExprId) -> VirtualValue {
        let expr = self.emit_expr(builder, expr).get_val(builder);
        let val = match op {
            PrefixOp::Not => builder.ins().bnot(expr),
            PrefixOp::Neg => builder.ins().ineg(expr),
        };
        VirtualValue::Direct(val)
    }

    fn emit_field(&self, builder: &mut FunctionBuilder, base: ExprId, field: SpanIdent) -> VirtualValue {
        let Ty::Named(id) = self.ty_map.ty(base) else {
            unreachable!("ICE")
        };

        let base = self.emit_expr(base);
        let (idx, field_ty) = self.hir.ty_info(*id).fields.get_ty_idx(field.ident);
        let field_ptr = self
            .builder
            .build_struct_gep(
                self.structs[*id],
                base.into_pointer_value(),
                idx,
                "fieldptr",
            )
            .unwrap();

        let result = if Self::is_indirect(field_ty) {
            let new_alloc = self.emit_alloca_entry(self.lower_ty(field_ty), &field.ident.str());
            self.emit_copy(field_ty, field_ptr.as_basic_value_enum(), new_alloc);
            new_alloc.as_basic_value_enum()
        } else {
            builder
                .ins()
                .build_load(self.lower_ty(field_ty), field_ptr, &field.ident.str())
                .unwrap()
        };
        self.emit_drop(&Ty::Named(*id), base);
        result
    }

    fn emit_index(&self, builder: &mut FunctionBuilder, arr: ExprId, idx: ExprId) -> VirtualValue {
        let ty = self.ty_map.ty(arr);
        let Ty::Array(elem_ty) = ty else {
            unreachable!("ICE")
        };

        let arr = self.emit_expr(arr);
        let idx = self.emit_expr(idx);
        builder
            .ins()
            .build_call(self.array_bounds_check(), &[arr.into(), idx.into()], "")
            .unwrap();
        let elem_ptr = unsafe {
            builder
                .ins()
                .build_in_bounds_gep(
                    self.lower_ty(elem_ty),
                    self.get_array_payload(arr.into_pointer_value()),
                    &[idx.into_int_value()],
                    "elemptr",
                )
                .unwrap()
        };

        let result = if Self::is_indirect(elem_ty) {
            let new_alloc = self.emit_alloca_entry(self.lower_ty(elem_ty), "elem");
            self.emit_copy(elem_ty, elem_ptr.as_basic_value_enum(), new_alloc);
            new_alloc.as_basic_value_enum()
        } else {
            builder
                .ins()
                .build_load(self.lower_ty(elem_ty), elem_ptr, "elem")
                .unwrap()
        };
        self.emit_drop(ty, arr);
        result
    }

    fn emit_call(
        &self,
        builder: &mut FunctionBuilder,
        func: ExprId,
        args: &[Arg],
        ret_ty: &Ty,
    ) -> VirtualValue {
        let mut tmps = Vec::new();
        let mut args: Vec<_> = args
            .iter()
            .map(|a| {
                let arg_ty = self.ty_map.ty(a.val);
                if let Expr::Ident(id) = self.hir.expr_info(a.val)
                    && !a.mutable
                    && Self::is_indirect(arg_ty)
                    && self.funcs.get(*id).is_none()
                {
                    self.vars[*id].as_basic_value_enum().into()
                } else {
                    let tmp = if a.mutable {
                        self.emit_unique_place(a.val).as_basic_value_enum()
                    } else {
                        self.emit_expr(a.val)
                    };
                    tmps.push((arg_ty, tmp));
                    tmp.into()
                }
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
        &self,
        builder: &mut FunctionBuilder,
        func: ExprId,
        mut args: Vec<BasicMetadataValueEnum<'ctx>>,
    ) -> CallSiteValue<'ctx> {
        if let Expr::Ident(id) = self.hir.expr_info(func)
            && let Some(func) = self.funcs.get(*id)
        {
            // Can use null environment if we're calling a top-level function
            args.push(self.null_ptr().as_basic_value_enum().into());
            builder.ins().build_call(*func, &args, "call").unwrap()
        } else {
            let closure = self.emit_expr(func).into_pointer_value();
            let ty = self.closure_ty();

            let env = self
                .builder
                .build_struct_gep(ty, closure, 1, "env")
                .unwrap();
            let env = builder.ins().build_load(self.ptr_ty(), env, "env").unwrap();
            args.push(env.as_basic_value_enum().into());

            let Ty::Fn(params, ret_ty) = self.ty_map.ty(func) else {
                unreachable!()
            };
            let func_ty = self.func_ty(params, ret_ty);
            let func = self
                .builder
                .build_struct_gep(ty, closure, 0, "func")
                .unwrap();
            let func = self
                .builder
                .build_load(self.ptr_ty(), func, "func")
                .unwrap();
            builder
                .ins()
                .build_indirect_call(func_ty, func.into_pointer_value(), &args, "call")
                .unwrap()
        }
    }

    fn emit_lambda(
        &self,
        builder: &mut FunctionBuilder,
        ty: &Ty,
        params: &[VarId],
        body: ExprId,
        captures: &[VarId],
    ) -> VirtualValue {
        // Create a unique name for this lambda, used for it's witnesses and it's defunctionalised body
        let func_name = format!("_lambda{}", self.lambda_counter);
        self.lambda_counter += 1;

        // Create the environment, if one is needed
        let (env, env_ty) = if captures.is_empty() {
            (self.null_ptr(), None)
        } else {
            // Allocate the environment.
            let env_ty = self.ctx.opaque_struct_type(&format!("{func_name}.Env"));
            let capture_tys: Vec<_> = captures
                .iter()
                .map(|id| self.lower_ty(self.hir.var_ty(*id)))
                .collect();
            env_ty.set_body(&capture_tys, false);
            let env = self
                .builder
                .build_call(
                    self.malloc(),
                    &[env_ty.size_of().unwrap().as_basic_value_enum().into()],
                    "malloc",
                )
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic()
                .into_pointer_value();

            // Initialize the environment
            for (idx, capture) in captures.iter().enumerate() {
                let dst = self
                    .builder
                    .build_struct_gep(env_ty, env, u32::try_from(idx).unwrap(), "captureptr")
                    .unwrap();
                let ty = self.hir.var_ty(*capture);
                let val = if Self::is_indirect(ty) {
                    self.vars[*capture].as_basic_value_enum()
                } else {
                    builder
                        .ins()
                        .build_load(self.lower_ty(ty), self.vars[*capture], "captureval")
                        .unwrap()
                };
                self.emit_copy(ty, val, dst);
            }

            (env, Some(env_ty))
        };

        // Create the defunctionalised function
        let func = {
            let Ty::Fn(param_tys, ret_ty) = ty else {
                unreachable!("ICE")
            };
            let func = self.module.add_function(
                &func_name,
                self.func_ty(param_tys, ret_ty),
                Some(Linkage::Private),
            );
            self.emit_defunc_body(func, body, params, ret_ty, captures, env_ty);
            func
        };

        // Create the final closure
        self.emit_closure(&func_name, func, captures, env, env_ty)
            .as_basic_value_enum()
    }

    fn emit_closure(
        &self,
        name: &str,
        func: FunctionValue<'ctx>,
        captures: &[VarId],
        env: PointerValue<'ctx>,
        env_ty: Option<StructType<'ctx>>,
    ) -> PointerValue<'ctx> {
        let closure_ty = self.closure_ty();
        let closure = self.emit_alloca_entry(closure_ty, "closure");

        let store_closure = |idx, val: PointerValue<'ctx>| {
            let ptr = self
                .builder
                .build_struct_gep(closure_ty, closure, idx, "fieldptr")
                .unwrap();
            builder.ins().build_store(ptr, val).unwrap();
        };

        store_closure(0, func.as_global_value().as_pointer_value());
        store_closure(1, env);
        store_closure(
            2,
            self.emit_closure_drop(name, captures, env_ty)
                .as_global_value()
                .as_pointer_value(),
        );
        store_closure(
            3,
            self.emit_closure_copy(name, captures, env_ty)
                .as_global_value()
                .as_pointer_value(),
        );
        store_closure(
            4,
            self.emit_closure_equals(name, captures, env_ty)
                .as_global_value()
                .as_pointer_value(),
        );
        closure
    }

    fn emit_defunc_body(
        &self,
        builder: &mut FunctionBuilder,
        func: FunctionValue<'ctx>,
        body: ExprId,
        params: &[VarId],
        ret_ty: &Ty,
        captures: &[VarId],
        env_ty: Option<StructType<'ctx>>,
    ) {
        // Save the builder's current insertion block to restore at the end
        let old_insert_block = builder.ins().get_insert_block().unwrap();

        let entry_block = self.ctx.append_basic_block(func, "entry");
        builder.ins().position_at_end(entry_block);

        // Skip the first argument if it's an out-pointer
        let offset = if Self::is_indirect(ret_ty) { 1 } else { 0 };
        for (arg, param) in iter::zip(func.get_param_iter().skip(offset), params) {
            let ty = self.hir.var_ty(*param);
            if self.hir.var_info(*param).mutable || Self::is_indirect(ty) {
                self.vars.insert(*param, arg.into_pointer_value());
            } else {
                let ptr = self.emit_alloca(arg.get_type(), &self.hir.var_info(*param).ident.str());
                builder.ins().build_store(ptr, arg).unwrap();
                self.vars.insert(*param, ptr);
            }
        }

        // Bind the captures, saving the original values to restore later
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

        // Emit the body and return
        let body = self.emit_expr(body);
        if Self::is_indirect(ret_ty) {
            let out_ptr = func.get_first_param().unwrap().into_pointer_value();
            self.emit_move(ret_ty, body, out_ptr);
            builder.ins().build_return(None).unwrap();
        } else {
            builder.ins().build_return(Some(&body)).unwrap();
        }

        assert!(func.verify(true));

        // Restore the insert block and the vars overwritten by captures
        for (id, ptr) in overwritten_vars {
            self.vars.insert(id, ptr);
        }
        builder.ins().position_at_end(old_insert_block);
    }

    fn emit_if(
        &self,
        builder: &mut FunctionBuilder,
        ty: &Ty,
        cond: ExprId,
        th: &BlockExpr,
        el: Option<&BlockExpr>,
    ) -> VirtualValue {
        match el {
            Some(el) => self.emit_if_else(builder, ty, cond, th, el),
            None => self.emit_if_no_else(builder, cond, th),
        }
    }

    fn emit_if_else(
        &self,
        builder: &mut FunctionBuilder,
        ty: &Ty, 
        cond: ExprId,
        th: &BlockExpr,
        el: &BlockExpr,
    ) -> VirtualValue {
        // Emit the condition in whatever block we're currently in.
        let cond = self.emit_expr(builder, cond);
        
        // Set up the blocks needed for the if-then-else.
        let mut th_block = builder.create_block();
        let mut el_block = builder.create_block();
        let merge_block = builder.create_block();
        builder.append_block_param(merge_block, self.lower_ty(ty));

        // Conditionally branch to either the then or else blocks.
        builder
            .ins()
            .brif(cond, th_block, [], el_block, []);
        builder.seal_block(th_block);
        builder.seal_block(el_block);

        // Emit the then block, then jump to the merge block with the result value.
        builder.switch_to_block(th_block);
        let th = self.emit_block_expr(builder, th);
        builder.ins().jump(merge_block, [&BlockArg::Value(th)]);
        // Account for any sub-blocks within the then block
        th_block = builder.current_block().unwrap();

        // Emit the else block, then jump to the merge block with the result value.
        builder.insert_block_after(el_block, th_block);
        builder.switch_to_block(el_block);
        let el = self.emit_block_expr(builder, el);
        builder.ins().jump(merge_block, [&BlockArg::Value(el)]);
        // Account for any sub-blocks within the else block
        el_block = builder.current_block().unwrap();

        builder.seal_block(merge_block);

        // Merge the result values.
        builder.insert_block_after(merge_block, el_block);
        builder.switch_to_block(merge_block);
        builder.block_params(merge_block)[0]
    }

    fn emit_if_no_else(
        &self,
        builder: &mut FunctionBuilder,
        cond: ExprId,
        th: &BlockExpr,
    ) -> VirtualValue {
        let cond = self.emit_expr(builder, cond);

        // Set up the blocks needed for the if-then.
        let mut th_block = builder.create_block();
        let merge_block = builder.create_block();

        // Conditionally branch to the then block or straight to the merge block
        builder
            .ins()
            .brif(cond, th_block, [], merge_block, []);
        builder.seal_block(th_block);

        // Emit the then block, then jump to the merge block.
        builder.switch_to_block(th_block);
        self.emit_block_expr(builder, th);
        builder.ins().jump(merge_block, []);
        // Account for any sub-blocks within the then block
        th_block = builder.current_block().unwrap();

        builder.seal_block(merge_block);

        builder.insert_block_after(merge_block, th_block);
        builder.switch_to_block(merge_block);
        self.unit()
    }

    fn emit_loop(&self, builder: &mut FunctionBuilder, body: &BlockExpr) -> VirtualValue {

        // Set up the blocks needed for the loop.
        let mut body_block = builder.create_block();
        let post_block = builder.create_block();

        // Jump into the body block.
        builder.ins().jump(body_block, []);
        builder.seal_block(body_block);

        // Emit the body block, then jump back to itself.
        builder.switch_to_block(body_block);
        self.emit_block_expr(builder, body);
        builder.ins().jump(body_block, []);
        // Account for any sub-blocks within the body block
        body_block = builder.current_block().unwrap();

        builder.seal_block(post_block);

        builder.insert_block_after(post_block, body_block);
        builder.switch_to_block(post_block);
        self.unit()
    }

    fn emit_block_expr(&self, builder: &mut FunctionBuilder, block: &BlockExpr) -> VirtualValue {
        let mut tmps: Vec<_> = block
            .stmts
            .iter()
            .map(|stmt| match stmt {
                Stmt::Decl { id, val, .. } => {
                    let ty = self.hir.var_ty(*id);
                    let val = if Self::is_indirect(ty) {
                        let slot = builder.create_sized_stack_slot(StackSlotData::new(
                            StackSlotKind::ExplicitSlot,
                            self.size_of(ty),
                            0,
                        ));
                        let ptr = builder.ins().stack_addr(self.ptr_ty(), slot, 0);
                        let val = self.emit_expr(builder, *val);
                        self.emit_move(builder, ty, val, ptr);

                        VirtualValue::Indirect(ptr)
                    } else if self.hir.var_info(*id).mutable {
                        let var = builder.declare_var(self.lower_ty(ty));
                        let val = self.emit_expr(builder, *val);
                        builder.def_var(var, val);
                        self.emit_drop(builder, ty, val);

                        VirtualValue::Variable(var)
                    } else {
                        VirtualValue::Direct(self.emit_expr(builder, *val))
                    };
                    self.vars.insert(*id, val);
                    (ty, val)
                }
                Stmt::Expr(expr) => (self.ty_map.ty(*expr), self.emit_expr(builder, *expr)),
            })
            .collect();
        let result = tmps.pop().map_or_else(|| self.unit(), |v| v.1);
        for (ty, val) in tmps {
            self.emit_drop(builder, ty, val);
        }
        result
    }
}
