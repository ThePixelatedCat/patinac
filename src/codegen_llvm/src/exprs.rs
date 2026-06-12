use hir::{Arg, BlockExpr, Expr, ExprId, InfixOp, LitExpr, PrefixOp, Stmt, Ty, VarId};
use ident::SpanIdent;
use inkwell::{
    FloatPredicate,
    module::Linkage,
    types::StructType,
    values::{BasicMetadataValueEnum, BasicValue, CallSiteValue, FunctionValue, PointerValue},
};

use crate::{
    Codegen,
    layout::{IntSize, LayoutValue, ScalarKind, ScalarLayout, StorageClass},
};

impl<'hir, 'ctx> Codegen<'hir, '_, 'ctx> {
    pub(crate) fn emit_expr(&mut self, expr: ExprId) -> LayoutValue<'hir, 'ctx> {
        match self.hir.expr_info(expr) {
            Expr::Ident(id) => self.emit_ident(*id),
            Expr::Lit(lit) => self.emit_lit(expr, lit),
            Expr::Array(exprs) => self.emit_array(&self.ty_map[expr], exprs),
            Expr::Tuple(exprs) => self.emit_tuple(&self.ty_map[expr], exprs),
            Expr::Infix { op, lhs, rhs } => self.emit_infix(*op, *lhs, *rhs),
            Expr::Prefix { op, expr } => self.emit_prefix(*op, *expr),
            Expr::Field { base, field } => self.emit_field(*base, *field),
            Expr::Index {
                array: arr,
                index: idx,
            } => self.emit_index(*arr, *idx),
            Expr::Call { func, args } => self.emit_call(*func, args, &self.ty_map[expr]),
            Expr::Lambda {
                params,
                body,
                captures,
            } => self.emit_lambda(&self.ty_map[expr], params, *body, captures),
            Expr::If { cond, th, el } => self.emit_if(&self.ty_map[expr], *cond, th, el.as_ref()),
            Expr::For { .. } => todo!(),
            Expr::Loop(body) => self.emit_loop(body),
            Expr::Break => todo!("Unconditional Control Flow"),
            Expr::Continue => todo!("Unconditional Control Flow"),
            Expr::Return(_) => todo!("Unconditional Control Flow"),
            Expr::Block(stmts) => self.emit_block_expr(stmts),

            Expr::Print(expr) => self.emit_print(*expr),
        }
    }

    fn emit_print(&mut self, expr: ExprId) -> LayoutValue<'hir, 'ctx> {
        let format = match &self.ty_map[expr] {
            Ty::Int => "%lld\n",
            Ty::UInt => "%llu\n",
            Ty::Byte => "%hhu\n",
            Ty::Float => "%f\n",
            Ty::Bool => "%hhd\n",
            Ty::Char => todo!("Strings"),
            Ty::Named(_) => todo!(),
            Ty::Tuple(_) => todo!(),
            Ty::Array(_) => todo!(),
            Ty::Func(_, _) => todo!(),
        };
        let format = self
            .builder
            .build_global_string_ptr(format, "format_string")
            .unwrap()
            .as_pointer_value();

        let expr = self.emit_expr(expr);
        self.builder
            .build_call(self.printf(), &[format.into(), expr.as_scalar().into()], "")
            .unwrap();

        LayoutValue::Zst
    }

    fn emit_place(&mut self, expr: ExprId) -> LayoutValue<'hir, 'ctx> {
        match self.hir.expr_info(expr) {
            Expr::Ident(id) => self.vars[*id],
            Expr::Field { base, field } => {
                let Ty::Named(id) = &self.ty_map[*base] else {
                    unreachable!("ICE")
                };
                let base = self.emit_place(*base).as_record();
                let (idx, field_ty) = self.hir.ty_info(*id).fields.get_ty_idx(field.ident);
                let field_ptr = self
                    .builder
                    .build_struct_gep(self.structs[*id], base, idx, "")
                    .unwrap();
                self.layout_indirect(field_ty, field_ptr)
            }
            Expr::Index { array, index } => {
                let array = self.emit_place(*array);
                let index = self.emit_expr(*index);
                self.emit_array_indexing(array, index)
            }
            Expr::Call { .. } => todo!("Projections"),
            _ => unreachable!("ICE: Tried to use non-place expr as place"),
        }
    }

    fn emit_unique_place(&mut self, expr: ExprId) -> LayoutValue<'hir, 'ctx> {
        match self.hir.expr_info(expr) {
            Expr::Ident(id) => self.vars[*id],
            Expr::Field { base, field } => {
                let Ty::Named(id) = &self.ty_map[*base] else {
                    unreachable!("ICE")
                };
                let base = self.emit_unique_place(*base).as_record();
                let (idx, field_ty) = self.hir.ty_info(*id).fields.get_ty_idx(field.ident);
                let field_ptr = self
                    .builder
                    .build_struct_gep(self.structs[*id], base, idx, "")
                    .unwrap();
                self.layout_indirect(field_ty, field_ptr)
            }
            Expr::Index { array, index } => {
                let array = self.emit_unique_place(*array);
                let index = self.emit_expr(*index);
                let (elem_ty, array_ptr) = array.as_array();
                self.builder
                    .build_call(self.array_unique(elem_ty), &[array_ptr.into()], "")
                    .unwrap();
                self.emit_array_indexing(array, index)
            }
            Expr::Call { .. } => todo!("Projections"),
            _ => unreachable!("ICE: Tried to use non-place expr as place"),
        }
    }

    fn emit_ident(&self, id: VarId) -> LayoutValue<'hir, 'ctx> {
        // If it's the name of a top-level function, wrap it in a function pointer.
        self.emit_dup(self.vars[id])
    }

    fn emit_lit(&self, expr: ExprId, lit: &LitExpr) -> LayoutValue<'hir, 'ctx> {
        match lit {
            &LitExpr::Int(value) => {
                let (int, size) = match &self.ty_map[expr] {
                    Ty::Int => (self.const_int(value.saturating_cast()), IntSize::Bits64),
                    Ty::UInt => (self.const_uint(value), IntSize::Bits64),
                    Ty::Byte => (self.const_byte(value.saturating_cast()), IntSize::Bits8),
                    _ => unreachable!("ICE: int literal inferred as non-int type"),
                };
                LayoutValue::int(size, int)
            }
            &LitExpr::Float(value) => LayoutValue::float(self.const_float(value)),
            LitExpr::Char(_) => todo!("Strings"),
            LitExpr::String(_) => todo!("Strings"),
            &LitExpr::Bool(value) => LayoutValue::int(IntSize::Bits8, self.const_bool(value)),
        }
    }

    fn emit_array(&mut self, ty: &'hir Ty, exprs: &[ExprId]) -> LayoutValue<'hir, 'ctx> {
        let Ty::Array(elem_ty) = ty else {
            unreachable!("ICE")
        };

        // Allocate the array.
        let array = self
            .builder
            .build_call(
                self.array_new(ty, elem_ty),
                &[self
                    .const_uint(
                        u64::try_from(exprs.len())
                            .expect("I doubt we'll see 128bit CPUs any time soon"),
                    )
                    .into()],
                "",
            )
            .unwrap()
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let array = LayoutValue::array(elem_ty, array);

        // Initialize each element.
        for (index, expr) in exprs.iter().enumerate() {
            let index = self.const_uint(
                u64::try_from(index).expect("I doubt we'll see 128bit CPUs any time soon"),
            );
            let elem_ptr =
                self.emit_array_indexing(array, LayoutValue::int(IntSize::Bits64, index));
            let elem = self.emit_expr(*expr);
            self.emit_move(elem, elem_ptr);
        }

        array
    }

    fn emit_tuple(&mut self, ty: &'hir Ty, exprs: &[ExprId]) -> LayoutValue<'hir, 'ctx> {
        // Unit.
        if exprs.is_empty() {
            return LayoutValue::Zst;
        }

        let lowered_ty = self.lower_ty(ty);
        let out = self.emit_alloca_entry(lowered_ty, "tuple");
        for (idx, expr) in exprs.iter().enumerate() {
            let tmp = self.emit_expr(*expr);
            let ptr = self
                .builder
                .build_struct_gep(lowered_ty, out, u32::try_from(idx).unwrap(), "")
                .unwrap();
            self.emit_move(tmp, self.layout_indirect(&self.ty_map[*expr], ptr));
        }
        LayoutValue::Tuple(ty, out)
    }

    fn emit_infix(&mut self, op: InfixOp, lhs: ExprId, rhs: ExprId) -> LayoutValue<'hir, 'ctx> {
        match op {
            InfixOp::Assign => {
                let dst = self.emit_unique_place(lhs);
                let tmp = self.emit_expr(rhs);
                // Drop the current value in the assigned-to variable
                self.emit_drop(dst);
                // Move the temporary value into the variable
                self.emit_move(tmp, dst);
                LayoutValue::Zst
            }
            _ => {
                let lhs = self.emit_expr(lhs);
                let rhs = self.emit_expr(rhs);
                self.emit_math_infix(op, lhs, rhs)
            }
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "Any given arm is readable on it's own"
    )]
    fn emit_math_infix(
        &self,
        op: InfixOp,
        lhs: LayoutValue<'hir, 'ctx>,
        rhs: LayoutValue<'hir, 'ctx>,
    ) -> LayoutValue<'hir, 'ctx> {
        match op {
            InfixOp::Assign => unreachable!("should not be called when the op is assignment"),
            InfixOp::Add => LayoutValue::int_op(lhs, rhs, |l, r| {
                self.builder.build_int_add(l, r, "").unwrap()
            }),
            InfixOp::AddF => LayoutValue::float(
                self.builder
                    .build_float_add(lhs.as_float(), rhs.as_float(), "")
                    .unwrap(),
            ),
            InfixOp::Sub => LayoutValue::int_op(lhs, rhs, |l, r| {
                self.builder.build_int_sub(l, r, "").unwrap()
            }),
            InfixOp::SubF => LayoutValue::float(
                self.builder
                    .build_float_sub(lhs.as_float(), rhs.as_float(), "")
                    .unwrap(),
            ),
            InfixOp::Mul => LayoutValue::int_op(lhs, rhs, |l, r| {
                self.builder.build_int_mul(l, r, "").unwrap()
            }),
            InfixOp::MulF => LayoutValue::float(
                self.builder
                    .build_float_mul(lhs.as_float(), rhs.as_float(), "")
                    .unwrap(),
            ),
            InfixOp::Div => todo!(),
            InfixOp::DivF => LayoutValue::float(
                self.builder
                    .build_float_div(lhs.as_float(), rhs.as_float(), "")
                    .unwrap(),
            ),
            InfixOp::Exp => todo!(),
            // FIXME: Short-circuiting
            InfixOp::And => {
                LayoutValue::int_op(lhs, rhs, |l, r| self.builder.build_and(l, r, "").unwrap())
            }
            InfixOp::Or => {
                LayoutValue::int_op(lhs, rhs, |l, r| self.builder.build_or(l, r, "").unwrap())
            }
            InfixOp::Xor => {
                LayoutValue::int_op(lhs, rhs, |l, r| self.builder.build_xor(l, r, "").unwrap())
            }
            op @ (InfixOp::Eqq | InfixOp::Neq) => {
                let equals = self.emit_equals(lhs, rhs);
                if op == InfixOp::Neq {
                    LayoutValue::int(IntSize::Bits8, self.builder.build_not(equals, "").unwrap())
                } else {
                    LayoutValue::int(IntSize::Bits8, equals)
                }
            }
            InfixOp::Gt => LayoutValue::int(
                IntSize::Bits8,
                self.builder
                    .build_float_compare(FloatPredicate::OGT, lhs.as_float(), rhs.as_float(), "")
                    .unwrap(),
            ),
            InfixOp::Lt => LayoutValue::int(
                IntSize::Bits8,
                self.builder
                    .build_float_compare(FloatPredicate::OLT, lhs.as_float(), rhs.as_float(), "")
                    .unwrap(),
            ),
            InfixOp::Geq => LayoutValue::int(
                IntSize::Bits8,
                self.builder
                    .build_float_compare(FloatPredicate::OGE, lhs.as_float(), rhs.as_float(), "")
                    .unwrap(),
            ),
            InfixOp::Leq => LayoutValue::int(
                IntSize::Bits8,
                self.builder
                    .build_float_compare(FloatPredicate::OLE, lhs.as_float(), rhs.as_float(), "")
                    .unwrap(),
            ),
        }
    }

    fn emit_prefix(&mut self, op: PrefixOp, expr: ExprId) -> LayoutValue<'hir, 'ctx> {
        let expr = self.emit_expr(expr);
        match op {
            PrefixOp::Not => LayoutValue::int(
                IntSize::Bits8,
                self.builder.build_not(expr.as_int(), "").unwrap(),
            ),
            PrefixOp::Neg => {
                LayoutValue::float(self.builder.build_float_neg(expr.as_float(), "").unwrap())
            }
        }
    }

    fn emit_field(&mut self, base: ExprId, field: SpanIdent) -> LayoutValue<'hir, 'ctx> {
        let Ty::Named(id) = &self.ty_map[base] else {
            unreachable!("ICE")
        };

        let base = self.emit_expr(base);
        let (idx, field_ty) = self.hir.ty_info(*id).fields.get_ty_idx(field.ident);
        let field_ptr = self
            .builder
            .build_struct_gep(self.structs[*id], base.as_record(), idx, "fieldptr")
            .unwrap();

        let result = self.emit_dup(self.layout_indirect(field_ty, field_ptr));
        self.emit_drop(base);
        result
    }

    fn emit_index(&mut self, array: ExprId, index: ExprId) -> LayoutValue<'hir, 'ctx> {
        let array = self.emit_expr(array);
        let index = self.emit_expr(index);
        let elem_ptr = self.emit_array_indexing(array, index);

        let result = self.emit_dup(elem_ptr);
        self.emit_drop(array);
        result
    }

    fn emit_call(
        &mut self,
        func: ExprId,
        args: &[Arg],
        ret_ty: &'hir Ty,
    ) -> LayoutValue<'hir, 'ctx> {
        let mut tmps = Vec::new();
        let mut args: Vec<_> = args
            .iter()
            .filter_map(|a| {
                let arg_ty = &self.ty_map[a.val];
                if self.is_zst(arg_ty) {
                    None
                } else if let Expr::Ident(id) = self.hir.expr_info(a.val)
                    && !a.mutable
                    && self.is_indirect(arg_ty)
                    && self.funcs.get(*id).is_none()
                {
                    Some(self.vars[*id].as_value().into())
                } else {
                    let tmp = if a.mutable {
                        self.emit_unique_place(a.val)
                    } else {
                        self.emit_expr(a.val)
                    };
                    tmps.push(tmp);
                    Some(tmp.as_value().into())
                }
            })
            .collect();

        let result = match self.storage_class(ret_ty) {
            StorageClass::Zst => {
                self.emit_call_inner(func, args);
                LayoutValue::Zst
            }
            StorageClass::Indirect => {
                let ret_ptr = self.emit_alloca_entry(self.lower_ty(ret_ty), "out");
                args.insert(0, ret_ptr.into());
                self.emit_call_inner(func, args);
                self.layout_direct(ret_ty, ret_ptr)
            }
            StorageClass::Scalar => {
                let result = self
                    .emit_call_inner(func, args)
                    .try_as_basic_value()
                    .unwrap_basic();
                let kind = match ret_ty {
                    Ty::Int | Ty::UInt => ScalarKind::Int(IntSize::Bits64),
                    Ty::Byte | Ty::Bool => ScalarKind::Int(IntSize::Bits8),
                    Ty::Float => ScalarKind::Float,
                    Ty::Char => todo!("Strings"),
                    Ty::Array(elem_ty) => ScalarKind::Array(elem_ty),
                    _ => unreachable!("not a scalar"),
                };
                LayoutValue::Scalar(kind, ScalarLayout::Direct(result))
            }
        };

        for tmp in tmps {
            self.emit_drop(tmp);
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
            return self.builder.build_call(*func, &args, "").unwrap();
        }
        match self.emit_expr(func) {
            LayoutValue::Scalar(ScalarKind::FuncPtr(func_ty), ScalarLayout::Direct(func)) => self
                .builder
                .build_indirect_call(func_ty, func.into_pointer_value(), &args, "")
                .unwrap(),
            LayoutValue::Scalar(ScalarKind::FuncPtr(func_ty), ScalarLayout::Indirect(ptr)) => {
                let func = self.builder.build_load(self.ptr_ty(), ptr, "").unwrap();
                self.builder
                    .build_indirect_call(func_ty, func.into_pointer_value(), &args, "")
                    .unwrap()
            }
            LayoutValue::Closure(func_ty, closure) => {
                let ty = self.closure_ty();

                // Extract environment from closure and add it to arguments.
                let env = self.builder.build_struct_gep(ty, closure, 1, "").unwrap();
                let env = self.builder.build_load(self.ptr_ty(), env, "").unwrap();
                args.push(env.as_basic_value_enum().into());

                // Extract function pointer from closure and call it.
                let func = self.builder.build_struct_gep(ty, closure, 0, "").unwrap();
                let func = self.builder.build_load(self.ptr_ty(), func, "").unwrap();
                self.builder
                    .build_indirect_call(func_ty, func.into_pointer_value(), &args, "")
                    .unwrap()
            }
            _ => unreachable!("wrong type for function"),
        }
    }

    fn emit_lambda(
        &mut self,
        ty: &Ty,
        params: &[VarId],
        body: ExprId,
        captures: &[VarId],
    ) -> LayoutValue<'hir, 'ctx> {
        // Create a unique name for this lambda, used for it's witnesses and it's defunctionalised body
        let func_name = format!("_lambda{}", self.lambda_counter);
        self.lambda_counter += 1;

        // Create the environment, if one is needed
        let (env, env_ty) = if captures.is_empty() {
            (self.const_null(), None)
        } else {
            // Allocate the environment.
            let capture_tys: Vec<_> = captures
                .iter()
                .map(|id| self.lower_ty(self.hir.var_ty(*id)))
                .collect();
            let env_ty = self.ctx.struct_type(&capture_tys, false);
            let env = self
                .builder
                .build_call(
                    self.malloc(),
                    &[env_ty.size_of().unwrap().as_basic_value_enum().into()],
                    "",
                )
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic()
                .into_pointer_value();

            // Initialize the environment
            for (idx, capture) in captures.iter().enumerate() {
                let dst = self
                    .builder
                    .build_struct_gep(env_ty, env, u32::try_from(idx).unwrap(), "")
                    .unwrap();
                let ty = self.hir.var_ty(*capture);
                self.emit_copy(self.vars[*capture], self.layout_indirect(ty, dst));
            }

            (env, Some(env_ty))
        };

        // Create the defunctionalised function
        let func = {
            let Ty::Func(param_tys, ret_ty) = ty else {
                unreachable!("ICE")
            };
            let func = self.module.add_function(
                &func_name,
                self.func_ty(param_tys, ret_ty, true),
                Some(Linkage::Private),
            );
            self.emit_lifted_body(func, body, params, ret_ty, captures, env_ty);
            func
        };

        // Create the final closure
        let closure = self.emit_closure(&func_name, func, captures, env, env_ty);
        LayoutValue::Closure(func.get_type(), closure)
    }

    fn emit_closure(
        &self,
        name: &str,
        func: FunctionValue<'ctx>,
        captures: &[VarId],
        env: PointerValue<'ctx>,
        env_ty: Option<StructType<'ctx>>,
    ) -> PointerValue<'ctx> {
        let ty = self.closure_ty();
        let closure = self.emit_alloca_entry(ty, "");

        // Store everything into the closure, emitting the witness functions along the way.
        let store = |idx, val: PointerValue<'ctx>| {
            let ptr = self.builder.build_struct_gep(ty, closure, idx, "").unwrap();
            self.builder.build_store(ptr, val).unwrap();
        };
        store(0, func.as_global_value().as_pointer_value());
        store(1, env);
        store(
            2,
            self.emit_closure_drop(name, captures, env_ty)
                .as_global_value()
                .as_pointer_value(),
        );
        store(
            3,
            self.emit_closure_copy(name, captures, env_ty)
                .as_global_value()
                .as_pointer_value(),
        );
        store(
            4,
            self.emit_closure_equals(name, captures, env_ty)
                .as_global_value()
                .as_pointer_value(),
        );

        closure
    }

    fn emit_lifted_body(
        &mut self,
        func: FunctionValue<'ctx>,
        body: ExprId,
        params: &[VarId],
        ret_ty: &Ty,
        captures: &[VarId],
        env_ty: Option<StructType<'ctx>>,
    ) {
        // Save the builder's current insertion block to restore at the end
        let old_insert_block = self.builder.get_insert_block().unwrap();

        let entry_block = self.ctx.append_basic_block(func, "entry");
        self.builder.position_at_end(entry_block);

        // Skip the first argument if it's an out-pointer.
        let offset = if self.is_indirect(ret_ty) { 1 } else { 0 };
        self.bind_params(params.iter().copied(), func.get_param_iter().skip(offset));

        // Bind the captures, saving the original values to restore later
        let mut overwritten_vars = Vec::new();
        if let Some(env_ty) = env_ty {
            let env = func.get_last_param().unwrap().into_pointer_value();
            for (idx, id) in captures.iter().enumerate() {
                let capture_ptr = self
                    .builder
                    .build_struct_gep(env_ty, env, u32::try_from(idx).unwrap(), "captureptr")
                    .unwrap();
                let capture = self.layout_direct(self.hir.var_ty(*id), capture_ptr);
                if let Some(old_ptr) = self.vars.insert(*id, capture) {
                    overwritten_vars.push((*id, old_ptr));
                }
            }
        }

        // Emit the body and return
        let body = self.emit_expr(body);

        match self.storage_class(ret_ty) {
            StorageClass::Zst => self.builder.build_return(None).unwrap(),
            StorageClass::Indirect => {
                let out_ptr = func.get_first_param().unwrap().into_pointer_value();
                self.emit_move(body, self.layout_indirect(ret_ty, out_ptr));
                self.builder.build_return(None).unwrap()
            }
            StorageClass::Scalar => self.builder.build_return(Some(&body.as_scalar())).unwrap(),
        };

        assert!(func.verify(true));

        // Clear the parameters to keep the variable map small.
        for id in params {
            self.vars.remove(*id);
        }

        // Restore the insert block and the vars overwritten by captures
        for (id, ptr) in overwritten_vars {
            self.vars.insert(id, ptr);
        }
        self.builder.position_at_end(old_insert_block);
    }

    fn emit_if(
        &mut self,
        ty: &'hir Ty,
        cond: ExprId,
        th: &BlockExpr,
        el: Option<&BlockExpr>,
    ) -> LayoutValue<'hir, 'ctx> {
        match el {
            Some(el) => self.emit_if_else(ty, cond, th, el),
            None => self.emit_if_no_else(cond, th),
        }
    }

    fn emit_if_else(
        &mut self,
        ty: &'hir Ty,
        cond: ExprId,
        th: &BlockExpr,
        el: &BlockExpr,
    ) -> LayoutValue<'hir, 'ctx> {
        let function = self.curr_function();

        // Set up blocks and result value alloc.
        let result = match self.storage_class(ty) {
            StorageClass::Zst => None,
            StorageClass::Indirect => Some(self.emit_alloca_entry(self.ptr_ty(), "if_result")),
            StorageClass::Scalar => Some(self.emit_alloca_entry(self.lower_ty(ty), "if_result")),
        };
        let th_block = self.ctx.append_basic_block(function, "then");
        let el_block = self.ctx.append_basic_block(function, "else");
        let merge_block = self.ctx.append_basic_block(function, "merge");

        // Branch.
        let cond = self.emit_expr(cond);
        self.builder
            .build_conditional_branch(cond.as_int(), th_block, el_block)
            .unwrap();

        // Then block.
        {
            self.builder.position_at_end(th_block);
            let th = self.emit_block_expr(th);
            if let Some(result) = result {
                self.builder.build_store(result, th.as_value()).unwrap();
            }
            self.builder
                .build_unconditional_branch(merge_block)
                .unwrap();
        }

        // Else block.
        {
            // Reposition after any sub-blocks of the then block.
            el_block
                .move_after(function.get_last_basic_block().unwrap())
                .unwrap();
            self.builder.position_at_end(el_block);
            let el = self.emit_block_expr(el);
            if let Some(result) = result {
                self.builder.build_store(result, el.as_value()).unwrap();
            }
            self.builder
                .build_unconditional_branch(merge_block)
                .unwrap();
        }

        // Extract result value.
        {
            // Reposition after any sub-blocks of the else block.
            merge_block
                .move_after(function.get_last_basic_block().unwrap())
                .unwrap();
            self.builder.position_at_end(merge_block);
            result.map_or(LayoutValue::Zst, |result| self.layout_direct(ty, result))
        }
    }

    fn emit_if_no_else(&mut self, cond: ExprId, th: &BlockExpr) -> LayoutValue<'hir, 'ctx> {
        let func = self.curr_function();

        // Append blocks to current function.
        let th_block = self.ctx.append_basic_block(func, "then");
        let merge_block = self.ctx.append_basic_block(func, "merge");

        // Branch on the condition.
        let cond = self.emit_expr(cond);
        self.builder
            .build_conditional_branch(cond.as_int(), th_block, merge_block)
            .unwrap();

        // Emit the then block.
        self.builder.position_at_end(th_block);
        let _ = self.emit_block_expr(th);
        self.builder
            .build_unconditional_branch(merge_block)
            .unwrap();

        // Reposition the merge block after any sub-blocks of the then block.
        merge_block
            .move_after(func.get_last_basic_block().unwrap())
            .unwrap();
        self.builder.position_at_end(merge_block);

        LayoutValue::Zst
    }

    fn emit_loop(&mut self, body: &BlockExpr) -> LayoutValue<'hir, 'ctx> {
        let function = self.curr_function();

        let body_block = self.ctx.append_basic_block(function, "body");
        self.builder.build_unconditional_branch(body_block).unwrap();

        self.builder.position_at_end(body_block);
        let _ = self.emit_block_expr(body);
        self.builder.build_unconditional_branch(body_block).unwrap();

        let post_block = self.ctx.append_basic_block(function, "post");
        self.builder.position_at_end(post_block);

        LayoutValue::Zst
    }

    fn emit_block_expr(&mut self, block: &BlockExpr) -> LayoutValue<'hir, 'ctx> {
        let mut locals = Vec::new();
        let mut last_expr = None;

        for stmt in &block.stmts {
            // Drop the previous expression, if there was one.
            if let Some(expr) = last_expr.take() {
                self.emit_drop(expr);
            }
            match stmt {
                Stmt::Decl { id, val, .. } => {
                    let ty = self.hir.var_ty(*id);
                    let val_tmp = self.emit_expr(*val);

                    // ZSTs and non-mutable values can be referenced directly, without a pointer.
                    // Sized, mutable values must be behind pointers for SSA reasons.
                    // Indirect values are already behind pointers, so they don't need a new allocation.
                    let val = if self.is_zst(ty)
                        || self.is_indirect(ty)
                        || !self.hir.var_info(*id).mutable
                    {
                        val_tmp
                    } else {
                        let alloc = self.emit_alloca_entry(
                            self.lower_ty(ty),
                            &self.hir.var_info(*id).ident.str(),
                        );
                        let val = self.layout_indirect(ty, alloc);
                        self.emit_move(val_tmp, val);
                        val
                    };

                    self.vars.insert(*id, val);
                    locals.push(*id);
                }
                Stmt::Expr(expr) => {
                    last_expr = Some(self.emit_expr(*expr));
                }
            }
        }

        // Drop all local variables.
        for var in locals {
            let var = self.vars.remove(var).expect("variable was just added");
            self.emit_drop(var);
        }

        last_expr.unwrap_or(LayoutValue::Zst)
    }
}
