use std::iter;

use cranelift::{
    codegen::{
        self, Context,
        ir::{
            BlockArg, Inst, InstBuilder, MemFlags, StackSlotData, StackSlotKind, Type, Value,
            condcodes::FloatCC, types,
        },
    },
    frontend::{FunctionBuilder, FunctionBuilderContext},
    module::{FuncId, Linkage, Module as _},
};
use hir::{
    VarId,
    exprs::{Arg, BlockExpr, Expr, ExprId, InfixOp, LitExpr, PrefixOp, Stmt},
    types::Ty,
};
use ident::SpanIdent;

use crate::{Codegen, Var};

impl Codegen<'_, '_> {
    pub(crate) fn emit_expr(&mut self, builder: &mut FunctionBuilder, expr: ExprId) -> Value {
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
            Expr::If { cond, th, el } => {
                self.emit_if(builder, self.ty_map.ty(expr), *cond, th, el.as_ref())
            }
            Expr::For { .. } => todo!(),
            Expr::Loop(body) => self.emit_loop(builder, body),
            Expr::Break => todo!("Unconditional Control Flow"),
            Expr::Continue => todo!("Unconditional Control Flow"),
            Expr::Return(_) => todo!("Unconditional Control Flow"),
            Expr::Block(stmts) => self.emit_block_expr(builder, stmts),

            Expr::Print(expr) => self.emit_print(builder, *expr),
        }
    }

    fn emit_print(&mut self, builder: &mut FunctionBuilder, expr: ExprId) -> Value {
        let format_string: &[u8] = match self.ty_map.ty(expr) {
            Ty::Int => b"%lld\n\0",
            Ty::UInt => b"%llu\n\0",
            Ty::Byte => b"%hhu\n\0",
            Ty::Float => b"%f\n\0",
            Ty::Bool => b"%hhd\n\0",
            Ty::Char => todo!("Strings"),
            Ty::Named(_) => todo!(),
            Ty::Tuple(_) => todo!(),
            Ty::Array(_) => todo!(),
            Ty::Fn(_, _) => todo!(),
        };
        let format_ptr = self.emit_global_string(builder, "format_string", format_string);

        let expr = self.emit_expr(builder, expr);

        let printf = self.printf();
        self.call(builder, printf, &[format_ptr, expr.into()]);

        self.unit()
    }

    fn emit_place(&mut self, builder: &mut FunctionBuilder, expr: ExprId) -> Value {
        match self.hir.expr_info(expr) {
            Expr::Ident(id) => {
                match self.vars[*id] {
                    Var::Direct(value) => value,
                    Var::Mutable(var) => {
                        // Promote to stack allocation
                        let value = builder.use_var(var);
                        let ptr = self.emit_alloc(builder, self.hir.var_ty(*id));
                        builder.ins().store(MemFlags::trusted(), value, ptr, 0);
                        self.vars[*id] = Var::Direct(ptr);
                        ptr
                    }
                }
            }
            Expr::Field { base, field } => {
                let Ty::Named(id) = self.ty_map.ty(*base) else {
                    unreachable!("ICE")
                };
                let base = self.emit_place(builder, *base);
                let (idx, _) = self.hir.ty_info(*id).fields.get_ty_idx(field.ident);
                todo!("Records/Tuples")
                // builder
                //     .ins()
                //     .build_struct_gep(self.structs[*id], base, idx, "fieldptr")
                //     .unwrap()
            }
            Expr::Index { arr, idx } => {
                let Ty::Array(elem_ty) = self.ty_map.ty(*arr) else {
                    unreachable!("ICE")
                };
                let arr = self.emit_place(builder, *arr);
                let idx = self.emit_expr(builder, *idx);
                todo!("Arrays")
                // builder
                //     .ins()
                //     .build_call(self.array_bounds_check(), &[arr.into(), idx.into()], "")
                //     .unwrap();
                // unsafe {
                //     builder
                //         .ins()
                //         .build_in_bounds_gep(
                //             self.lower_ty(elem_ty),
                //             self.get_array_payload(arr),
                //             &[idx.into_int_value()],
                //             "elemptr",
                //         )
                //         .unwrap()
                // }
            }
            Expr::Call { .. } => todo!("Projections"),
            _ => unreachable!("ICE: Tried to use non-place expr as place"),
        }
    }

    fn emit_unique_place(&mut self, builder: &mut FunctionBuilder, expr: ExprId) -> Value {
        match self.hir.expr_info(expr) {
            Expr::Ident(_) => self.emit_place(builder, expr),
            Expr::Field { base, field } => {
                let Ty::Named(id) = self.ty_map.ty(*base) else {
                    unreachable!("ICE")
                };
                let base = self.emit_unique_place(builder, *base);
                let (idx, _) = self.hir.ty_info(*id).fields.get_ty_idx(field.ident);
                todo!("Records/Tuples")
                // builder
                //     .ins()
                //     .build_struct_gep(self.structs[*id], base, idx, "fieldptr")
                //     .unwrap()
            }
            Expr::Index { arr, idx } => {
                let ty = self.ty_map.ty(*arr);
                let elem_ty = self.ty_map.ty(expr);
                let arr = self.emit_unique_place(builder, *arr);
                let idx = self.emit_expr(builder, *idx);
                todo!("Records/Tuples")
                // builder
                //     .ins()
                //     .build_call(self.array_bounds_check(), &[arr.into(), idx.into()], "")
                //     .unwrap();
                // builder
                //     .ins()
                //     .build_call(self.array_unique(ty, elem_ty), &[arr.into()], "")
                //     .unwrap();
                // unsafe {
                //     builder
                //         .ins()
                //         .build_in_bounds_gep(
                //             self.lower_ty(elem_ty),
                //             self.get_array_payload(arr),
                //             &[idx.into_int_value()],
                //             "elemptr",
                //         )
                //         .unwrap()
                // }
            }
            Expr::Call { .. } => todo!("Projections"),
            _ => unreachable!("ICE: Tried to use non-place expr as place"),
        }
    }

    fn unit(&self) -> Value {
        self.ctx.const_struct(&[], false).as_basic_value_enum()
    }

    fn emit_ident(&self, builder: &mut FunctionBuilder, id: VarId) -> Value {
        // If it's the name of a top-level function, convert it into a closure
        if let Some(func) = self.funcs.get(id) {
            return self.emit_closure(
                &self.hir.var_info(id).ident.str(),
                *func,
                &[],
                self.null_ptr(builder),
                None,
            );
        }

        let var = self.vars[id];
        let ty = self.hir.var_ty(id);

        if let Ty::Array(_) = ty {
            todo!("properly copy arrays");
        }

        match var {
            Var::Direct(val) => {
                if Self::is_indirect(ty) {
                    let new_ptr = self.emit_alloc(builder, ty);
                    self.emit_copy(builder, ty, val, new_ptr);
                    new_ptr
                } else {
                    val
                }
            }
            Var::Mutable(variable) => builder.use_var(variable),
        }
    }

    fn emit_lit(&self, builder: &mut FunctionBuilder, expr: ExprId, lit: &LitExpr) -> Value {
        match lit {
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
                        }
                    };
                    builder.ins().iconst(types::I64, val)
                }
                // FIXME: verify that casting like this works
                Ty::UInt => builder.ins().iconst(types::I64, *val as i64),
                Ty::Byte => {
                    let val = match u8::try_from(*val) {
                        Ok(val) => val,
                        Err(_) => {
                            self.handler.warn(
                                &format!("int literal {val} overflowed and was clamped"),
                                self.hir.expr_span(expr),
                            );
                            u8::MAX
                        }
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
        }
    }

    fn emit_array(&self, builder: &mut FunctionBuilder, ty: &Ty, exprs: &[ExprId]) -> Value {
        let Ty::Array(elem_ty) = ty else {
            unreachable!("ICE")
        };

        todo!("Arrays")

        // // Allocate the array.
        // let alloc = self.emit_alloca_entry(self.array_ty(), "array");
        // let count = builder.ins().iconst(types::I64, i64::try_from(exprs.len()).unwrap());
        // let ret = builder
        //     .ins()
        //     .call(
        //         self.array_init(ty, elem_ty),
        //         &[
        //             alloc.into(),
        //             count
        //         ],
        //     );
        // let array = builder.inst_results(ret)[0];

        // // Initialize each element.
        // let payload = self.get_array_payload(alloc);
        // let lowered_elem_ty = self.lower_ty(elem_ty);
        // for (idx, expr) in exprs.iter().enumerate() {
        //     let idx = self.ctx.i64_type().const_int(
        //         u64::try_from(idx).expect("I doubt we'll see 128bit CPUs any time soon"),
        //         false,
        //     );
        //     let ptr = unsafe {
        //         builder
        //             .ins()
        //             .build_in_bounds_gep(lowered_elem_ty, payload, &[idx], "ptr")
        //             .unwrap()
        //     };
        //     let elem = self.emit_expr(*expr);
        //     self.emit_move(elem_ty, elem, ptr);
        // }

        // alloc.as_basic_value_enum()
    }

    fn emit_tuple(&self, builder: &mut FunctionBuilder, ty: &Ty, exprs: &[ExprId]) -> Value {
        // Fast-path explicit units
        if exprs.is_empty() {
            return self.unit();
        }

        todo!("Tuples/Records")
        // let ty = self.lower_ty(ty);
        // let out = self.emit_alloca_entry(ty, "tuple");
        // for (idx, expr) in exprs.iter().enumerate() {
        //     let tmp = self.emit_expr(*expr);
        //     let ptr = self
        //         .builder
        //         .build_struct_gep(ty, out, u32::try_from(idx).unwrap(), &format!("field{idx}"))
        //         .unwrap();
        //     self.emit_move(self.ty_map.ty(*expr), tmp, ptr);
        // }
        // out.as_basic_value_enum()
    }

    fn emit_infix(
        &mut self,
        builder: &mut FunctionBuilder,
        op: InfixOp,
        lhs: ExprId,
        rhs: ExprId,
    ) -> Value {
        let ty = self.ty_map.ty(lhs);
        match op {
            InfixOp::Assign => {
                let dst = self.emit_unique_place(builder, lhs);
                let tmp = self.emit_expr(builder, rhs);
                // Drop the current value in the assigned-to variable
                self.emit_drop(builder, ty, dst);
                // Move the temporary value into the variable
                self.emit_move(builder, ty, tmp, dst);
                self.unit()
            }
            _ => {
                let lhs = self.emit_expr(builder, lhs);
                let rhs = self.emit_expr(builder, rhs);
                self.emit_math_infix(builder, ty, op, lhs, rhs)
            }
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "Any given arm is readable on it's own"
    )]
    fn emit_math_infix(
        &self,
        builder: &mut FunctionBuilder,
        ty: &Ty,
        op: InfixOp,
        lhs: Value,
        rhs: Value,
    ) -> Value {
        match op {
            InfixOp::Assign => unreachable!("ICE: Should not be called when the op is assignment"),
            InfixOp::Add => builder.ins().iadd(lhs, rhs),
            InfixOp::AddF => builder.ins().fadd(lhs, rhs),
            InfixOp::Sub => builder.ins().isub(lhs, rhs),
            InfixOp::SubF => builder.ins().fsub(lhs, rhs),
            InfixOp::Mul => builder.ins().imul(lhs, rhs),
            InfixOp::MulF => builder.ins().fmul(lhs, rhs),
            InfixOp::Div => match ty {
                Ty::Int => builder.ins().sdiv(lhs, rhs),
                Ty::UInt => builder.ins().udiv(lhs, rhs),
                _ => unreachable!(),
            },
            InfixOp::DivF => builder.ins().fdiv(lhs, rhs),
            InfixOp::Exp => todo!(),
            InfixOp::And => todo!("Short-circuiting operators"),
            InfixOp::Or => todo!("Short-circuiting operators"),
            InfixOp::Xor => builder.ins().bxor(lhs, rhs),
            InfixOp::Eqq | InfixOp::Neq => {
                let equals = self.emit_equals(builder, ty, lhs, rhs);
                if op == InfixOp::Neq {
                    builder.ins().bnot(equals)
                } else {
                    equals
                }
            }
            InfixOp::Gt => builder.ins().fcmp(FloatCC::GreaterThan, lhs, rhs),
            InfixOp::Lt => builder.ins().fcmp(FloatCC::LessThan, lhs, rhs),
            InfixOp::Geq => builder.ins().fcmp(FloatCC::GreaterThanOrEqual, lhs, rhs),
            InfixOp::Leq => builder.ins().fcmp(FloatCC::LessThanOrEqual, lhs, rhs),
        }
    }

    fn emit_prefix(&mut self, builder: &mut FunctionBuilder, op: PrefixOp, expr: ExprId) -> Value {
        let expr = self.emit_expr(builder, expr);
        match op {
            PrefixOp::Not => builder.ins().bnot(expr),
            PrefixOp::Neg => builder.ins().ineg(expr),
        }
    }

    fn emit_field(
        &mut self,
        builder: &mut FunctionBuilder,
        base: ExprId,
        field: SpanIdent,
    ) -> Value {
        let Ty::Named(id) = self.ty_map.ty(base) else {
            unreachable!("ICE")
        };

        let base = self.emit_expr(builder, base);
        let (idx, field_ty) = self.hir.ty_info(*id).fields.get_ty_idx(field.ident);
        todo!("Records/Tuples")
        // let field_ptr = self
        //     .builder
        //     .build_struct_gep(
        //         self.structs[*id],
        //         base.into_pointer_value(),
        //         idx,
        //         "fieldptr",
        //     )
        //     .unwrap();

        // let result = if Self::is_indirect(field_ty) {
        //     let new_alloc = self.emit_alloca_entry(self.lower_ty(field_ty), &field.ident.str());
        //     self.emit_copy(field_ty, field_ptr.as_basic_value_enum(), new_alloc);
        //     new_alloc.as_basic_value_enum()
        // } else {
        //     builder
        //         .ins()
        //         .build_load(self.lower_ty(field_ty), field_ptr, &field.ident.str())
        //         .unwrap()
        // };
        // self.emit_drop(&Ty::Named(*id), base);
        // result
    }

    fn emit_index(&mut self, builder: &mut FunctionBuilder, arr: ExprId, idx: ExprId) -> Value {
        let ty = self.ty_map.ty(arr);
        let Ty::Array(elem_ty) = ty else {
            unreachable!("ICE")
        };

        let arr = self.emit_expr(builder, arr);
        let idx = self.emit_expr(builder, idx);
        todo!("Arrays")
        // builder
        //     .ins()
        //     .build_call(self.array_bounds_check(), &[arr.into(), idx.into()], "")
        //     .unwrap();
        // let elem_ptr = unsafe {
        //     builder
        //         .ins()
        //         .build_in_bounds_gep(
        //             self.lower_ty(elem_ty),
        //             self.get_array_payload(arr.into_pointer_value()),
        //             &[idx.into_int_value()],
        //             "elemptr",
        //         )
        //         .unwrap()
        // };

        // let result = if Self::is_indirect(elem_ty) {
        //     let new_alloc = self.emit_alloca_entry(self.lower_ty(elem_ty), "elem");
        //     self.emit_copy(elem_ty, elem_ptr.as_basic_value_enum(), new_alloc);
        //     new_alloc.as_basic_value_enum()
        // } else {
        //     builder
        //         .ins()
        //         .build_load(self.lower_ty(elem_ty), elem_ptr, "elem")
        //         .unwrap()
        // };
        // self.emit_drop(ty, arr);
        // result
    }

    fn emit_call(
        &mut self,
        builder: &mut FunctionBuilder,
        func: ExprId,
        args: &[Arg],
        ret_ty: &Ty,
    ) -> Value {
        let mut tmps = Vec::new();
        let mut args: Vec<_> = args
            .iter()
            .map(|a| {
                let arg_ty = self.ty_map.ty(a.val);
                // Alias records where possible
                if let Expr::Ident(id) = self.hir.expr_info(a.val) // Argument is plain variable name
                    && !a.mutable // Immutable, safe to alias
                    && Self::is_indirect(arg_ty) // Indirect, caller is expecting pointer
                    && self.funcs.get(*id).is_none()
                // Not global function, which need to be made into closures by `emit_ident`
                {
                    let Var::Direct(ptr) = self.vars[*id] else {
                        unreachable!()
                    };
                    ptr
                } else {
                    let tmp = if a.mutable {
                        self.emit_unique_place(builder, a.val)
                    } else {
                        self.emit_expr(builder, a.val)
                    };
                    tmps.push((arg_ty, tmp));
                    tmp
                }
            })
            .collect();

        let result = if Self::is_indirect(ret_ty) {
            let ret_ptr = self.emit_alloc(builder, ret_ty);
            args.insert(0, ret_ptr);
            self.emit_call_inner(builder, func, args);
            ret_ptr
        } else {
            let inst = self.emit_call_inner(builder, func, args);
            builder.inst_results(inst)[0]
        };

        for (ty, val) in tmps {
            self.emit_drop(builder, ty, val);
        }

        result
    }

    fn emit_call_inner(
        &mut self,
        builder: &mut FunctionBuilder,
        func: ExprId,
        mut args: Vec<Value>,
    ) -> Inst {
        if let Expr::Ident(id) = self.hir.expr_info(func)
            && let Some(func) = self.funcs.get(*id)
        {
            // Can use null environment if we're calling a top-level function
            args.push(self.null_ptr(builder));
            let func = self.decl_func(builder, *func);
            builder.ins().call(func, &args)
        } else {
            todo!("Closures")
            // let closure = self.emit_expr(builder, func);
            // let ty = self.closure_ty();

            // let env = builder
            //     .build_struct_gep(ty, closure, 1, "env")
            //     .unwrap();
            // let env = builder.ins().build_load(self.ptr_ty(), env, "env").unwrap();
            // args.push(env.as_basic_value_enum().into());

            // let Ty::Fn(params, ret_ty) = self.ty_map.ty(func) else {
            //     unreachable!()
            // };
            // let func_ty = self.func_ty(params, ret_ty);
            // let func = self
            //     .builder
            //     .build_struct_gep(ty, closure, 0, "func")
            //     .unwrap();
            // let func = self
            //     .builder
            //     .build_load(self.ptr_ty(), func, "func")
            //     .unwrap();
            // builder
            //     .ins()
            //     .call_indirect(func_ty, func.into_pointer_value(), &args)
        }
    }

    fn emit_lambda(
        &mut self,
        builder: &mut FunctionBuilder,
        ty: &Ty,
        params: &[VarId],
        body: ExprId,
        captures: &[VarId],
    ) -> Value {
        // Create a unique name for this lambda, used for it's witnesses and it's defunctionalised body
        let func_name = format!("_lambda{}", self.lambda_counter);
        self.lambda_counter += 1;

        // Create the environment, if one is needed
        let (env, env_ty) = if captures.is_empty() {
            (self.null_ptr(builder), None)
        } else {
            todo!("Closures")
            // Allocate the environment.
            // let env_ty = self.ctx.opaque_struct_type(&format!("{func_name}.Env"));
            // let capture_tys: Vec<_> = captures
            //     .iter()
            //     .map(|id| self.lower_ty(self.hir.var_ty(*id)))
            //     .collect();
            // env_ty.set_body(&capture_tys, false);
            // let env = self
            //     .builder
            //     .build_call(
            //         self.malloc(),
            //         &[env_ty.size_of().unwrap().as_basic_value_enum().into()],
            //         "malloc",
            //     )
            //     .unwrap()
            //     .try_as_basic_value()
            //     .unwrap_basic()
            //     .into_pointer_value();

            // // Initialize the environment
            // for (idx, capture) in captures.iter().enumerate() {
            //     let dst = self
            //         .builder
            //         .build_struct_gep(env_ty, env, u32::try_from(idx).unwrap(), "captureptr")
            //         .unwrap();
            //     let ty = self.hir.var_ty(*capture);
            //     let val = if Self::is_indirect(ty) {
            //         self.vars[*capture].as_basic_value_enum()
            //     } else {
            //         builder
            //             .ins()
            //             .build_load(self.lower_ty(ty), self.vars[*capture], "captureval")
            //             .unwrap()
            //     };
            //     self.emit_copy(ty, val, dst);
            // }

            // (env, Some(env_ty))
        };

        // Create the defunctionalised function
        let func = {
            let Ty::Fn(param_tys, ret_ty) = ty else {
                unreachable!("ICE")
            };
            let func = self
                .module
                .declare_function(
                    &func_name,
                    Linkage::Local,
                    &self.create_signature(param_tys, ret_ty),
                )
                .unwrap();
            self.emit_defunc_body(func, body, params, ret_ty, captures, env_ty);
            func
        };

        // Create the final closure
        self.emit_closure(&func_name, func, captures, env, env_ty)
    }

    fn emit_closure(
        &self,
        name: &str,
        func: FuncId,
        captures: &[VarId],
        env: Value,
        env_ty: Option<Type>,
    ) -> Value {
        todo!("Closures")
        // let closure_ty = self.closure_ty();
        // let closure = self.emit_alloca_entry(closure_ty, "closure");

        // let store_closure = |idx, val: PointerValue<'ctx>| {
        //     let ptr = self
        //         .builder
        //         .build_struct_gep(closure_ty, closure, idx, "fieldptr")
        //         .unwrap();
        //     builder.ins().build_store(ptr, val).unwrap();
        // };

        // store_closure(0, func.as_global_value().as_pointer_value());
        // store_closure(1, env);
        // store_closure(
        //     2,
        //     self.emit_closure_drop(name, captures, env_ty)
        //         .as_global_value()
        //         .as_pointer_value(),
        // );
        // store_closure(
        //     3,
        //     self.emit_closure_copy(name, captures, env_ty)
        //         .as_global_value()
        //         .as_pointer_value(),
        // );
        // store_closure(
        //     4,
        //     self.emit_closure_equals(name, captures, env_ty)
        //         .as_global_value()
        //         .as_pointer_value(),
        // );
        // closure
    }

    fn emit_defunc_body(
        &mut self,
        ctx: &mut Context,
        func_ctx: &mut FunctionBuilderContext,
        func: FuncId,
        body: ExprId,
        params: &[VarId],
        ret_ty: &Ty,
        captures: &[VarId],
        env_ty: Option<Type>,
    ) {
        let mut builder = FunctionBuilder::new(&mut ctx.func, func_ctx);
        builder.func.signature = self.get_signature(func);

        // Create the function's entry block.
        let entry_block = builder.create_block();
        builder.switch_to_block(entry_block);
        builder.append_block_params_for_function_params(entry_block);
        builder.seal_block(entry_block);

        // Skip the first argument if it's an out-pointer
        let offset = if Self::is_indirect(ret_ty) { 1 } else { 0 };
        for (param, arg) in iter::zip(
            params,
            builder.block_params(entry_block).iter().skip(offset),
        ) {
            self.vars.insert(*param, Var::Direct(*arg));
        }

        // Bind the captures, saving the original values to restore later
        let mut overwritten_vars = Vec::new();
        if let Some(env_ty) = env_ty {
            let env = builder.block_params(entry_block).last().unwrap();
            todo!("Closures")
            // for (idx, capture) in captures.iter().enumerate() {
            //     let capture_ptr = self
            //         .builder
            //         .build_struct_gep(env_ty, env, u32::try_from(idx).unwrap(), "captureptr")
            //         .unwrap();
            //     if let Some(old_ptr) = self.vars.insert(*capture, capture_ptr) {
            //         overwritten_vars.push((*capture, old_ptr));
            //     }
            // }
        }

        // Emit the body and return
        let body = self.emit_expr(&mut builder, body);
        if Self::is_indirect(ret_ty) {
            let out_ptr = builder.block_params(entry_block)[0];
            self.emit_move(&mut builder, ret_ty, body, out_ptr);
            builder.ins().return_(&[]);
        } else {
            builder.ins().return_(&[body]);
        }

        codegen::verify_function(&builder.func, self.module.isa()).unwrap();
        builder.finalize();
        self.module.define_function(func, ctx).unwrap();
        ctx.clear();

        // Restore the vars overwritten by captures
        for (id, ptr) in overwritten_vars {
            self.vars.insert(id, ptr);
        }
    }

    fn emit_if(
        &mut self,
        builder: &mut FunctionBuilder,
        ty: &Ty,
        cond: ExprId,
        th: &BlockExpr,
        el: Option<&BlockExpr>,
    ) -> Value {
        match el {
            Some(el) => self.emit_if_else(builder, ty, cond, th, el),
            None => self.emit_if_no_else(builder, cond, th),
        }
    }

    fn emit_if_else(
        &mut self,
        builder: &mut FunctionBuilder,
        ty: &Ty,
        cond: ExprId,
        th: &BlockExpr,
        el: &BlockExpr,
    ) -> Value {
        // Emit the condition in whatever block we're currently in.
        let cond = self.emit_expr(builder, cond);

        // Set up the blocks needed for the if-then-else.
        let mut th_block = builder.create_block();
        let mut el_block = builder.create_block();
        let merge_block = builder.create_block();
        builder.append_block_param(merge_block, self.lower_ty(ty));

        // Conditionally branch to either the then or else blocks.
        builder.ins().brif(cond, th_block, [], el_block, []);
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
        &mut self,
        builder: &mut FunctionBuilder,
        cond: ExprId,
        th: &BlockExpr,
    ) -> Value {
        let cond = self.emit_expr(builder, cond);

        // Set up the blocks needed for the if-then.
        let mut th_block = builder.create_block();
        let merge_block = builder.create_block();

        // Conditionally branch to the then block or straight to the merge block
        builder.ins().brif(cond, th_block, [], merge_block, []);
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

    fn emit_loop(&mut self, builder: &mut FunctionBuilder, body: &BlockExpr) -> Value {
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

    fn emit_block_expr(&mut self, builder: &mut FunctionBuilder, block: &BlockExpr) -> Value {
        let mut tmps: Vec<_> = block
            .stmts
            .iter()
            .map(|stmt| match stmt {
                Stmt::Decl { id, val, .. } => {
                    let ty = self.hir.var_ty(*id);
                    let val = if Self::is_indirect(ty) {
                        let ptr = self.emit_alloc(builder, ty);
                        let val = self.emit_expr(builder, *val);
                        self.emit_move(builder, ty, val, ptr);

                        Var::Direct(ptr)
                    } else if self.hir.var_info(*id).mutable {
                        let var = builder.declare_var(self.lower_ty(ty));
                        let val = self.emit_expr(builder, *val);
                        builder.def_var(var, val);
                        self.emit_drop(builder, ty, val);

                        Var::Mutable(var)
                    } else {
                        Var::Direct(self.emit_expr(builder, *val))
                    };
                    self.vars.insert(*id, val);
                    (ty, val)
                }
                Stmt::Expr(expr) => (
                    self.ty_map.ty(*expr),
                    Var::Direct(self.emit_expr(builder, *expr)),
                ),
            })
            .collect();
        let result = tmps
            .pop()
            .map_or_else(|| self.unit(), |(_, var)| var.get_val(builder));
        for (ty, var) in tmps {
            let val = var.get_val(builder);
            self.emit_drop(builder, ty, val);
        }
        result
    }
}
