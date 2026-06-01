mod exprs;
mod runtime;
#[cfg(test)]
mod test;
mod witnesses;

use std::{iter, path::PathBuf, range::Range, str::FromStr};

use inkwell::{
    AddressSpace,
    builder::Builder,
    context::Context,
    module::Module,
    passes::PassBuilderOptions,
    targets::{FileType, InitializationConfig, Target, TargetMachine, TargetMachineOptions},
    types::{BasicType as _, BasicTypeEnum, FunctionType, StructType},
    values::{BasicValueEnum, FunctionValue, PointerValue},
};
use slotmap::SecondaryMap;

use errors::ErrorHandler;
use hir::{
    Hir, TyMap, VarId,
    exprs::ExprId,
    items::{ExecKind, TyId},
    types::{Param, Ty},
};

#[derive(PartialEq, Eq)]
pub enum CodegenMode {
    IRDump,
    Emit(PathBuf),
    Silent,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OptLevel {
    #[default]
    O0 = 0,
    O1 = 1,
    O2 = 2,
    O3 = 3,
}

impl FromStr for OptLevel {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "0" => Ok(Self::O0),
            "1" => Ok(Self::O1),
            "2" => Ok(Self::O2),
            "3" => Ok(Self::O3),
            _ => Err(r#"expected "0", "1", "2", or "3""#),
        }
    }
}

impl OptLevel {
    #[expect(clippy::as_conversions, reason = "Casting to access enum discriminant")]
    pub fn opt_string(self) -> String {
        match self {
            Self::O0 | Self::O1 | Self::O2 | Self::O3 => {
                format!("default<O{}>", self as u8)
            }
        }
    }
}

pub struct Codegen<'hir, 'handler, 'ctx> {
    hir: &'hir Hir,
    ty_map: &'hir TyMap,
    handler: ErrorHandler<'handler>,
    ctx: &'ctx Context,
    builder: Builder<'ctx>,
    module: Module<'ctx>,
    target: TargetMachine,
    structs: SecondaryMap<TyId, StructType<'ctx>>,
    funcs: SecondaryMap<VarId, FunctionValue<'ctx>>,
    vars: SecondaryMap<VarId, PointerValue<'ctx>>,
    lambda_counter: u32,
}

pub fn create_ctx() -> Context {
    Context::create()
}

impl<'hir, 'handler, 'ctx> Codegen<'hir, 'handler, 'ctx> {
    pub fn new(
        hir: &'hir Hir,
        ty_map: &'hir TyMap,
        handler: ErrorHandler<'handler>,
        ctx: &'ctx Context,
        module_name: &str,
    ) -> Self {
        let module = ctx.create_module(module_name);

        Target::initialize_native(&InitializationConfig::default()).unwrap();
        let triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&triple).unwrap();
        let target_machine = target
            .create_target_machine_from_options(&triple, TargetMachineOptions::default())
            .unwrap();

        let this = Self {
            hir,
            ty_map,
            handler,
            ctx,
            builder: ctx.create_builder(),
            module,
            target: target_machine,
            structs: Self::build_structs(hir, ctx),
            funcs: SecondaryMap::new(),
            vars: SecondaryMap::new(),
            lambda_counter: 0,
        };
        this.populate_structs();
        this
    }

    #[allow(
        clippy::unwrap_used,
        reason = "A large number of Inkwell functions return Results for error conditions we don't want to recover from"
    )]
    pub fn codegen(&mut self, opt_level: OptLevel, mode: CodegenMode) {
        for (ty, _) in self.hir.tys() {
            self.build_constructor(ty);
        }

        for exec in self.hir.execs() {
            match &exec.kind {
                ExecKind::Const { .. } => todo!("Constants"),
                ExecKind::Fn { .. } => {
                    self.build_func(exec.id);
                }
            }
        }

        if let Some(main) = self.hir.main() {
            let fn_ty = self.ctx.i32_type().fn_type(&[], false);
            let func = self.module.add_function("main", fn_ty, None);
            self.funcs.insert(main.id, func);

            let ExecKind::Fn { body, .. } = main.kind else {
                unreachable!("ICE")
            };

            let entry_block = self.ctx.append_basic_block(func, "entry");
            self.builder.position_at_end(entry_block);
            let _ = self.emit_expr(body);
            self.builder
                .build_return(Some(&self.ctx.i32_type().const_zero()))
                .unwrap();

            assert!(func.verify(true));
        }

        for exec in self.hir.execs() {
            match &exec.kind {
                ExecKind::Const { .. } => todo!("Constants"),
                ExecKind::Fn { params, body } => {
                    let Ty::Fn(_, ret_ty) = self.hir.var_ty(exec.id) else {
                        unreachable!("ICE")
                    };
                    self.populate_func(self.funcs[exec.id], params, ret_ty, *body);
                }
            }
        }

        self.module.verify().unwrap();

        self.module
            .set_data_layout(&self.target.get_target_data().get_data_layout());
        self.module.set_triple(&self.target.get_triple());

        self.module
            .run_passes(
                &opt_level.opt_string(),
                &self.target,
                PassBuilderOptions::create(),
            )
            .unwrap();

        match mode {
            CodegenMode::IRDump => self.module.print_to_stderr(),
            CodegenMode::Emit(path) => {
                self.target
                    .write_to_file(&self.module, FileType::Object, &path)
                    .unwrap();
            }
            CodegenMode::Silent => {}
        }
    }

    fn report_warning(&mut self, msg: &str, span: Range<usize>) {
        self.handler.warn(msg, span);
    }

    fn build_structs(hir: &Hir, ctx: &'ctx Context) -> SecondaryMap<TyId, StructType<'ctx>> {
        hir.tys()
            .map(|(id, ident)| (id, ctx.opaque_struct_type(&ident.ident.str())))
            .collect()
    }

    fn populate_structs(&self) {
        for (id, ty) in &self.structs {
            let field_tys: Vec<_> = (&self.hir.ty_info(id).fields)
                .into_iter()
                .map(|(_, ty)| {
                    if let Ty::Named(field_id) = ty
                        && *field_id == id
                    {
                        todo!("Recursive records")
                    } else {
                        self.lower_ty(ty)
                    }
                })
                .collect();
            ty.set_body(&field_tys, false);
        }
    }

    fn build_func(&mut self, id: VarId) -> FunctionValue<'ctx> {
        let Ty::Fn(params, ret_ty) = self.hir.var_ty(id) else {
            unreachable!("ICE")
        };
        let func = self.module.add_function(
            &self.hir.var_info(id).ident.str(),
            self.build_func_ty(params, ret_ty),
            None,
        );
        self.funcs.insert(id, func);
        self.vars
            .insert(id, func.as_global_value().as_pointer_value());
        func
    }

    fn build_func_ty(&self, params: &[Param], ret_ty: &Ty) -> FunctionType<'ctx> {
        let mut param_tys: Vec<_> = params
            .iter()
            .map(|p| {
                if p.mutable || Self::is_indirect(&p.ty) {
                    self.ptr_ty()
                } else {
                    self.lower_ty(&p.ty)
                }
                .into()
            })
            .collect();

        // Add parameter for the environment
        param_tys.push(self.ptr_ty().into());

        if Self::is_indirect(ret_ty) {
            param_tys.insert(0, self.ptr_ty().into());
            self.ctx.void_type().fn_type(&param_tys, false)
        } else {
            self.lower_ty(ret_ty).fn_type(&param_tys, false)
        }
    }

    #[allow(
        clippy::unwrap_used,
        reason = "A large number of Inkwell functions return Results for error conditions we don't want to recover from"
    )]
    fn build_constructor(&mut self, ty: TyId) {
        let info = self.hir.ty_info(ty);

        let func = self.build_func(info.constructor_id);
        let entry_block = self.ctx.append_basic_block(func, "entry");
        self.builder.position_at_end(entry_block);

        let ty = self.lower_ty(&Ty::Named(ty));
        let out_ptr = func.get_first_param().unwrap().into_pointer_value();
        for (idx, (arg, field_ty)) in
            iter::zip(func.get_param_iter().skip(1), info.fields.tys()).enumerate()
        {
            let field_ptr = self
                .builder
                .build_struct_gep(ty, out_ptr, u32::try_from(idx).unwrap(), "fieldptr")
                .unwrap();
            self.emit_copy(field_ty, arg, field_ptr);
        }

        self.builder.build_return(None).unwrap();

        assert!(func.verify(true));
    }

    #[allow(
        clippy::unwrap_used,
        reason = "A large number of Inkwell functions return Results for error conditions we don't want to recover from"
    )]
    fn populate_func(
        &mut self,
        func: FunctionValue<'ctx>,
        params: &[VarId],
        ret_ty: &Ty,
        body: ExprId,
    ) {
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

        let body = self.emit_expr(body);

        if Self::is_indirect(ret_ty) {
            let out_ptr = func.get_first_param().unwrap().into_pointer_value();
            self.emit_move(ret_ty, body, out_ptr);
            self.builder.build_return(None).unwrap();
        } else {
            self.builder.build_return(Some(&body)).unwrap();
        }

        assert!(func.verify(true));
    }

    fn lower_ty(&self, ty: &Ty) -> BasicTypeEnum<'ctx> {
        match ty {
            Ty::Int | Ty::UInt => self.ctx.i64_type().as_basic_type_enum(),
            Ty::Byte => self.ctx.i8_type().as_basic_type_enum(),
            Ty::Float => self.ctx.f64_type().as_basic_type_enum(),
            Ty::Char => todo!("Strings"),
            Ty::Bool => self.ctx.bool_type().as_basic_type_enum(),
            Ty::Tuple(inner_tys) => {
                let inner_tys: Vec<_> = inner_tys.iter().map(|ty| self.lower_ty(ty)).collect();
                self.ctx.struct_type(&inner_tys, false).as_basic_type_enum()
            }
            Ty::Array(_) => self.array_ty(),
            Ty::Fn(..) => self.closure_ty(),
            Ty::Named(id) => self.structs[*id].as_basic_type_enum(),
        }
    }

    fn array_ty(&self) -> BasicTypeEnum<'ctx> {
        if let Some(ty) = self.module.get_struct_type("_Array") {
            return ty.as_basic_type_enum();
        }

        let ty = self.ctx.opaque_struct_type("_Array");
        ty.set_body(&[self.ptr_ty()], false);
        ty.as_basic_type_enum()
    }

    fn array_header_ty(&self) -> BasicTypeEnum<'ctx> {
        if let Some(ty) = self.module.get_struct_type("_ArrayHeader") {
            return ty.as_basic_type_enum();
        }

        let ty = self.ctx.opaque_struct_type("_ArrayHeader");
        let i64_ty = self.ctx.i64_type().as_basic_type_enum();
        // Refcount, element count, capacity
        ty.set_body(&[i64_ty, i64_ty, i64_ty], false);
        ty.as_basic_type_enum()
    }

    fn closure_ty(&self) -> BasicTypeEnum<'ctx> {
        if let Some(ty) = self.module.get_struct_type("_Closure") {
            return ty.as_basic_type_enum();
        }

        let ty = self.ctx.opaque_struct_type("_Closure");
        ty.set_body(
            &[
                // Function
                self.ptr_ty(),
                // Environment
                self.ptr_ty(),
                // Drop
                self.ptr_ty(),
                // Copy
                self.ptr_ty(),
                // Eq
                self.ptr_ty(),
            ],
            false,
        );
        ty.as_basic_type_enum()
    }

    fn ptr_ty(&self) -> BasicTypeEnum<'ctx> {
        self.ctx
            .ptr_type(AddressSpace::default())
            .as_basic_type_enum()
    }

    fn null_ptr(&self) -> PointerValue<'ctx> {
        self.ctx.ptr_type(AddressSpace::default()).const_null()
    }

    fn is_trivial(&self, ty: &Ty) -> bool {
        match ty {
            Ty::Int | Ty::UInt | Ty::Byte | Ty::Float | Ty::Char | Ty::Bool => true,
            Ty::Array(_) | Ty::Fn(_, _) => false,
            Ty::Tuple(tys) => tys.iter().all(|ty| self.is_trivial(ty)),
            Ty::Named(id) => (&self.hir.ty_info(*id).fields)
                .into_iter()
                .all(|(_, ty)| self.is_trivial(ty)),
        }
    }

    const fn is_indirect(ty: &Ty) -> bool {
        match ty {
            Ty::Int | Ty::UInt | Ty::Byte | Ty::Float | Ty::Char | Ty::Bool => false,
            Ty::Array(_) | Ty::Fn(_, _) | Ty::Named(_) => true,
            Ty::Tuple(inner) => !inner.is_empty(),
        }
    }

    #[allow(
        clippy::unwrap_used,
        reason = "A large number of Inkwell functions return Results for error conditions we don't want to recover from"
    )]
    fn emit_alloca(&self, ty: BasicTypeEnum<'ctx>, name: &str) -> PointerValue<'ctx> {
        self.builder.build_alloca(ty, name).unwrap()
    }

    /// # Panics
    /// Panics if the builder is not positioned, or is positioned but not within a function.
    fn emit_alloca_entry(&self, ty: BasicTypeEnum<'ctx>, name: &str) -> PointerValue<'ctx> {
        let curr_block = self
            .builder
            .get_insert_block()
            .expect("builder has been positioned");
        let head_block = curr_block
            .get_parent()
            .expect("builder is within function")
            .get_first_basic_block()
            .expect("function has at least one block; we got this function via a block");

        if let Some(first_instr) = head_block.get_first_instruction() {
            self.builder.position_before(&first_instr);
        } else {
            self.builder.position_at_end(head_block);
        }

        let ptr = self.emit_alloca(ty, name);

        self.builder.position_at_end(curr_block);

        ptr
    }

    fn emit_move(&self, ty: &Ty, val: BasicValueEnum<'ctx>, to: PointerValue<'ctx>) {
        self.emit_copy(ty, val, to);
        self.emit_drop(ty, val);
    }

    /// # Panics
    /// Panics if the builder is not positioned, or is positioned but not within a function.
    fn curr_function(&self) -> FunctionValue<'ctx> {
        self.builder
            .get_insert_block()
            .expect("builder has been positioned")
            .get_parent()
            .expect("builder is within function")
    }
}
