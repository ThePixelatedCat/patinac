//! Generates LLVM-IR and emits object files from the [`Hir`].
//!
//! The entry point to this crate is the [`Codegen`] type, and the [`codegen`][Codegen::codegen] method on it.
//! Use the [`create_ctx`] function to acquire a [`Context`] for use in [`Codegen::new`].

#![allow(
    clippy::unwrap_used,
    reason = "A large number of Inkwell functions return Results for error conditions we don't want to recover from"
)]

mod exprs;
mod runtime;
#[cfg(test)]
mod test;
mod witnesses;

use std::{fmt::Write as _, iter, path::PathBuf, str::FromStr};

use inkwell::{
    AddressSpace, FloatPredicate, IntPredicate,
    builder::Builder,
    context::Context,
    module::Module,
    passes::PassBuilderOptions,
    targets::{FileType, InitializationConfig, Target, TargetMachine, TargetMachineOptions},
    types::{BasicType, BasicTypeEnum, FunctionType, StructType},
    values::{BasicValue as _, BasicValueEnum, FunctionValue, PointerValue},
};
use slotmap::SecondaryMap;

use errors::ErrorHandler;
use hir::{
    Hir, TyMap, VarId,
    exprs::ExprId,
    items::{ExecKind, TyId},
    types::{Param, Ty},
};

/// What to produce, if anything.
#[derive(PartialEq, Eq)]
pub enum CodegenMode {
    /// Dump the LLVM IR to stderr.
    IRDump,
    /// Emit an object file at the given path.
    Emit(PathBuf),
    /// Run verification checks but do nothing else (for testing).
    Silent,
}

/// What level of optimisation to use.
///
/// Currently directly corresponds to the LLVM optimisation levels of the same names.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OptLevel {
    /// `-O0`. No optimisation.
    #[default]
    O0 = 0,
    /// `-O1`.
    O1 = 1,
    /// `-O2`.
    O2 = 2,
    /// `-O3`. Full optimisations (minus LTO).
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
    /// Converts the optimisation level into a string of LLVM optimisation passes, as expected by `opt`.
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

        Self {
            hir,
            ty_map,
            handler,
            ctx,
            builder: ctx.create_builder(),
            module,
            target: target_machine,
            structs: SecondaryMap::new(),
            funcs: SecondaryMap::new(),
            vars: SecondaryMap::new(),
            lambda_counter: 0,
        }
    }

    /// # Panics
    /// Panics if any functions are invalid, or if writing to the output file fails.
    pub fn codegen(&mut self, opt_level: OptLevel, mode: CodegenMode) {
        for (ty, _) in self.hir.tys() {
            self.create_struct(ty);
        }
        for (ty, _) in self.hir.tys() {
            self.build_struct(ty);
        }
        for (ty, _) in self.hir.tys() {
            self.build_constructor(ty);
        }

        for exec in self.hir.execs() {
            match &exec.kind {
                ExecKind::Const { .. } => todo!("Constants"),
                ExecKind::Fn { .. } => {
                    self.create_func(exec.id);
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
                    self.build_func(self.funcs[exec.id], params, ret_ty, *body);
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

    fn create_struct(&mut self, id: TyId) {
        let name = Self::mangle_name(self.hir.ty_ident(id).ident.to_string());
        self.structs.insert(id, self.ctx.opaque_struct_type(&name));
    }

    fn build_struct(&self, id: TyId) {
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
        self.structs[id].set_body(&field_tys, false);
    }

    fn create_func(&mut self, id: VarId) -> FunctionValue<'ctx> {
        let Ty::Fn(params, ret_ty) = self.hir.var_ty(id) else {
            unreachable!("ICE")
        };
        let name = Self::mangle_name(self.hir.var_info(id).ident.to_string());
        let func = self
            .module
            .add_function(&name, self.func_ty(params, ret_ty), None);
        self.funcs.insert(id, func);
        self.vars
            .insert(id, func.as_global_value().as_pointer_value());
        func
    }

    fn build_constructor(&mut self, ty: TyId) {
        let info = self.hir.ty_info(ty);

        let func = self.create_func(info.constructor_id);
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

    fn build_func(
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

    fn get_array_payload(&self, array: PointerValue<'ctx>) -> PointerValue<'ctx> {
        let payload = self
            .builder
            .build_struct_gep(self.array_ty(), array, 0, "payload")
            .unwrap();
        self.builder
            .build_load(self.ptr_ty(), payload, "payload")
            .unwrap()
            .into_pointer_value()
    }

    fn get_array_header(&self, array: PointerValue<'ctx>) -> PointerValue<'ctx> {
        unsafe {
            self.builder
                .build_in_bounds_gep(
                    self.array_header_ty(),
                    self.get_array_payload(array),
                    &[self.ctx.i64_type().const_int(1, true).const_neg()],
                    "header",
                )
                .unwrap()
        }
    }

    fn func_ty(&self, params: &[Param], ret_ty: &Ty) -> FunctionType<'ctx> {
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

        // Add parameter for return out-pointer if needed
        if Self::is_indirect(ret_ty) {
            param_tys.insert(0, self.ptr_ty().into());
            self.ctx.void_type().fn_type(&param_tys, false)
        } else {
            self.lower_ty(ret_ty).fn_type(&param_tys, false)
        }
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

    pub(crate) fn emit_drop(&self, ty: &Ty, val: BasicValueEnum<'ctx>) {
        // Trivial types don't have a drop function and don't need dropping
        let Some(func) = self.drop_func(ty) else {
            return;
        };
        self.builder
            .build_call(func, &[val.into()], "drop")
            .unwrap();
    }

    pub(crate) fn emit_copy(&self, ty: &Ty, val: BasicValueEnum<'ctx>, dst: PointerValue<'ctx>) {
        // Trivial types don't have a copy function and don't need dropping
        let Some(func) = self.copy_func(ty) else {
            return;
        };
        self.builder
            .build_call(
                func,
                &[dst.as_basic_value_enum().into(), val.into()],
                "copy",
            )
            .unwrap();
    }

    pub(crate) fn emit_equals(
        &self,
        ty: &Ty,
        lhs: BasicValueEnum<'ctx>,
        rhs: BasicValueEnum<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        match ty {
            Ty::Int | Ty::UInt | Ty::Byte | Ty::Bool => self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    lhs.into_int_value(),
                    rhs.into_int_value(),
                    "equals",
                )
                .unwrap()
                .as_basic_value_enum(),
            Ty::Float => self
                .builder
                .build_float_compare(
                    FloatPredicate::OEQ,
                    lhs.into_float_value(),
                    rhs.into_float_value(),
                    "equals",
                )
                .unwrap()
                .as_basic_value_enum(),
            Ty::Char => todo!("Strings"),
            Ty::Tuple(inner_tys) => {
                // If it's empty, it's unit and therefore always equals
                if inner_tys.is_empty() {
                    self.ctx.bool_type().const_all_ones().as_basic_value_enum()
                } else {
                    self.builder
                        .build_call(
                            self.tuple_equals(ty, inner_tys),
                            &[lhs.into(), rhs.into()],
                            "equals",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .unwrap_basic()
                }
            }
            Ty::Array(inner_ty) => self
                .builder
                .build_call(
                    self.array_equals(ty, inner_ty),
                    &[lhs.into(), rhs.into()],
                    "equals",
                )
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic(),
            Ty::Fn(_, _) => self
                .builder
                .build_call(self.closure_equals(), &[lhs.into(), rhs.into()], "equals")
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic(),
            Ty::Named(id) => self
                .builder
                .build_call(self.struct_equals(*id), &[lhs.into(), rhs.into()], "equals")
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic(),
        }
    }

    fn emit_move(&self, ty: &Ty, val: BasicValueEnum<'ctx>, to: PointerValue<'ctx>) {
        self.emit_copy(ty, val, to);
        self.emit_drop(ty, val);
    }

    /// # Panics
    /// Panics if the provided type is unsized.
    fn emit_memcpy(&self, dst: PointerValue<'ctx>, src: PointerValue<'ctx>, ty: &dyn BasicType) {
        let align = self.target.get_target_data().get_abi_alignment(ty);
        let size = ty.size_of().expect("sized type");
        self.builder
            .build_memcpy(dst, align, src, align, size)
            .unwrap();
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

    fn mangle_name(mut name: String) -> String {
        name.insert(0, '_');
        name
    }

    fn mangle_ty(&self, ty: &Ty) -> String {
        match ty {
            Ty::Int => "i".to_string(),
            Ty::UInt => "u".to_string(),
            Ty::Byte => "h".to_string(),
            Ty::Float => "f".to_string(),
            Ty::Char => "c".to_string(),
            Ty::Bool => "b".to_string(),
            Ty::Tuple(tys) => format!(
                "T{}",
                tys.iter().map(|ty| self.mangle_ty(ty)).collect::<String>()
            ),
            Ty::Array(ty) => format!("A{}", self.mangle_ty(ty)),
            Ty::Fn(params, ret_ty) => {
                let param_names = params.iter().fold(String::new(), |mut s, p| {
                    let prefix = if p.mutable { "M" } else { "P" };
                    let _ = write!(s, "{prefix}{}", self.mangle_ty(&p.ty));
                    s
                });
                format!("f[{param_names};{}]", self.mangle_ty(ret_ty))
            }
            Ty::Named(id) => Self::mangle_name(self.hir.ty_ident(*id).ident.to_string()),
        }
    }
}
