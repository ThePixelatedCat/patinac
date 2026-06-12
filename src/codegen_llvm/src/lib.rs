//! Generates LLVM-IR and emits object files from the [`Hir`].
//!
//! The entry point to this crate is the [`Codegen`] type, and the [`codegen`][Codegen::codegen] method on it.
//! Use the [`create_ctx`] function to acquire a [`Context`] for use in [`Codegen::new`].

#![feature(integer_casts)]
#![allow(
    clippy::unwrap_used,
    reason = "A large number of Inkwell functions return Results for error conditions we don't want to recover from"
)]

mod arrays;
mod exprs;
mod layout;
mod runtime;
#[cfg(test)]
mod test;
mod witnesses;

use std::{fmt::Write as _, iter, path::PathBuf, str::FromStr};

use inkwell::{
    AddressSpace, FloatPredicate, IntPredicate,
    basic_block::BasicBlock,
    builder::Builder,
    context::Context,
    module::Module,
    passes::PassBuilderOptions,
    targets::{FileType, InitializationConfig, Target, TargetMachine, TargetMachineOptions},
    types::{BasicType, BasicTypeEnum, FunctionType, StructType},
    values::{BasicValueEnum, FloatValue, FunctionValue, IntValue, PointerValue},
};
use slotmap::SecondaryMap;

use errors::ErrorHandler;
use hir::{ExecKind, ExprId, Hir, Param, Ty, TyId, VarId};

use crate::layout::{IntSize, LayoutValue, ScalarKind, ScalarLayout, StorageClass};

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
    ty_map: &'hir SecondaryMap<ExprId, Ty>,
    handler: ErrorHandler<'handler>,
    ctx: &'ctx Context,
    builder: Builder<'ctx>,
    module: Module<'ctx>,
    target: TargetMachine,
    structs: SecondaryMap<TyId, StructType<'ctx>>,
    funcs: SecondaryMap<VarId, FunctionValue<'ctx>>,
    vars: SecondaryMap<VarId, LayoutValue<'hir, 'ctx>>,
    lambda_counter: u32,
}

/// Creates a new [`Context`].
///
/// This is a direct wrapper over [`Context::create()`], and exists so that other crates don't have to depend on Inkwell directly.
pub fn create_ctx() -> Context {
    Context::create()
}

impl<'hir, 'handler, 'ctx> Codegen<'hir, 'handler, 'ctx> {
    /// Creates a new [`Codegen`] for a package with the given name.
    ///
    /// The context should be obtained via [`create_ctx()`].
    ///
    /// # Panics
    /// Panics if there is an issue initialising the target.
    pub fn new(
        hir: &'hir Hir,
        ty_map: &'hir SecondaryMap<ExprId, Ty>,
        handler: ErrorHandler<'handler>,
        ctx: &'ctx Context,
        package_name: &str,
    ) -> Self {
        let module = ctx.create_module(package_name);

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
                    let Ty::Func(_, ret_ty) = self.hir.var_ty(exec.id) else {
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
        let Ty::Func(params, ret_ty) = self.hir.var_ty(id) else {
            unreachable!("ICE")
        };
        let name = Self::mangle_name(self.hir.var_info(id).ident.to_string());
        let func_ty = self.func_ty(params, ret_ty, false);
        let func = self.module.add_function(&name, func_ty, None);
        self.funcs.insert(id, func);
        self.vars.insert(
            id,
            LayoutValue::func_ptr(func_ty, func.as_global_value().as_pointer_value()),
        );
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
                .build_struct_gep(ty, out_ptr, u32::try_from(idx).unwrap(), "")
                .unwrap();
            self.emit_copy(
                self.layout(field_ty, arg),
                self.layout_indirect(field_ty, field_ptr),
            );
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

        // Skip the first argument if it's an out-pointer.
        let offset = if self.is_indirect(ret_ty) { 1 } else { 0 };
        self.bind_params(params.iter().copied(), func.get_param_iter().skip(offset));

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

        // Clear the parameters to keep the variable map small.
        for id in params {
            self.vars.remove(*id);
        }

        assert!(func.verify(true));
    }

    fn bind_params<P: IntoIterator<Item = VarId>, A: Iterator<Item = BasicValueEnum<'ctx>>>(
        &mut self,
        params: P,
        mut args: A,
    ) {
        for id in params {
            let ty = self.hir.var_ty(id);
            let mutable = self.hir.var_info(id).mutable;
            // We don't actually pass ZSTs.
            if self.is_zst(ty) {
                self.vars.insert(id, LayoutValue::Zst);
                continue;
            }
            let value = args.next().expect("there should be enough args");
            let value = if mutable {
                self.layout_indirect(ty, value.into_pointer_value())
            } else {
                self.layout(ty, value)
            };
            self.vars.insert(id, value);
        }
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
            Ty::Func(..) => self.closure_ty(),
            Ty::Named(id) => self.structs[*id].as_basic_type_enum(),
        }
    }

    fn array_ty(&self) -> BasicTypeEnum<'ctx> {
        self.ptr_ty()
    }

    fn array_header_ty(&self) -> BasicTypeEnum<'ctx> {
        if let Some(ty) = self.module.get_struct_type("ArrayHeader") {
            return ty.as_basic_type_enum();
        }

        let ty = self.ctx.opaque_struct_type("ArrayHeader");
        let i64_ty = self.ctx.i64_type().as_basic_type_enum();
        // Refcount, element count, capacity
        ty.set_body(&[i64_ty, i64_ty, i64_ty], false);
        ty.as_basic_type_enum()
    }

    fn func_ty(&self, params: &[Param], ret_ty: &Ty, env: bool) -> FunctionType<'ctx> {
        let mut param_tys: Vec<_> = params
            .iter()
            .filter_map(|p| {
                if self.is_zst(&p.ty) {
                    None // Skip passing ZSTs.
                } else if p.mutable || self.is_indirect(&p.ty) {
                    Some(self.ptr_ty().into()) // Pass stack pointers for mutable parameters or indirect types.
                } else {
                    Some(self.lower_ty(&p.ty).into()) // Pass by value otherwise.
                }
            })
            .collect();

        // Add parameter for environment if necessary
        if env {
            param_tys.push(self.ptr_ty().into());
        }

        // Add parameter for return out-pointer if needed.
        match self.storage_class(ret_ty) {
            StorageClass::Zst => self.ctx.void_type().fn_type(&param_tys, false),
            StorageClass::Indirect => {
                param_tys.insert(0, self.ptr_ty().into());
                self.ctx.void_type().fn_type(&param_tys, false)
            }
            StorageClass::Scalar => self.lower_ty(ret_ty).fn_type(&param_tys, false),
        }
    }

    fn closure_ty(&self) -> BasicTypeEnum<'ctx> {
        if let Some(ty) = self.module.get_struct_type("Closure") {
            return ty.as_basic_type_enum();
        }

        let ty = self.ctx.opaque_struct_type("Closure");
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

    fn const_int(&self, value: i64) -> IntValue<'ctx> {
        self.ctx.i64_type().const_int(value.cast_unsigned(), false)
    }

    fn const_uint(&self, value: u64) -> IntValue<'ctx> {
        self.ctx.i64_type().const_int(value, false)
    }

    fn const_byte(&self, value: u8) -> IntValue<'ctx> {
        self.ctx.i8_type().const_int(u64::from(value), false)
    }

    fn const_float(&self, value: f64) -> FloatValue<'ctx> {
        self.ctx.f64_type().const_float(value)
    }

    fn const_bool(&self, value: bool) -> IntValue<'ctx> {
        match value {
            true => self.ctx.bool_type().const_all_ones(),
            false => self.ctx.bool_type().const_zero(),
        }
    }

    fn const_null(&self) -> PointerValue<'ctx> {
        self.ctx.ptr_type(AddressSpace::default()).const_null()
    }

    fn layout(&self, ty: &'hir Ty, value: BasicValueEnum<'ctx>) -> LayoutValue<'hir, 'ctx> {
        match ty {
            Ty::Int | Ty::UInt => LayoutValue::int(IntSize::Bits64, value),
            Ty::Byte | Ty::Bool => LayoutValue::int(IntSize::Bits8, value),
            Ty::Float => LayoutValue::float(value),
            Ty::Char => todo!("Strings"),
            Ty::Tuple(_) => LayoutValue::Tuple(ty, value.into_pointer_value()),
            Ty::Array(elem_ty) => LayoutValue::array(elem_ty, value),
            Ty::Func(params, ret_ty) => LayoutValue::Closure(
                self.func_ty(params, ret_ty, true),
                value.into_pointer_value(),
            ),
            Ty::Named(id) => LayoutValue::Record(*id, value.into_pointer_value()),
        }
    }

    fn layout_direct(&self, ty: &'hir Ty, ptr: PointerValue<'ctx>) -> LayoutValue<'hir, 'ctx> {
        match ty {
            Ty::Int | Ty::UInt => {
                let int = self.builder.build_load(self.lower_ty(ty), ptr, "").unwrap();
                LayoutValue::int(IntSize::Bits64, int)
            }
            Ty::Byte | Ty::Bool => {
                let int = self.builder.build_load(self.lower_ty(ty), ptr, "").unwrap();
                LayoutValue::int(IntSize::Bits8, int)
            }
            Ty::Float => {
                let float = self.builder.build_load(self.lower_ty(ty), ptr, "").unwrap();
                LayoutValue::float(float)
            }
            Ty::Char => todo!("Strings"),
            Ty::Tuple(_) => LayoutValue::Tuple(ty, ptr),
            Ty::Array(elem_ty) => {
                let array = self.builder.build_load(self.lower_ty(ty), ptr, "").unwrap();
                LayoutValue::array(elem_ty, array)
            }
            Ty::Func(params, ret_ty) => {
                let func_ty = self.func_ty(params, ret_ty, true);
                LayoutValue::Closure(func_ty, ptr)
            }
            Ty::Named(id) => LayoutValue::Record(*id, ptr),
        }
    }

    fn layout_indirect(&self, ty: &'hir Ty, ptr: PointerValue<'ctx>) -> LayoutValue<'hir, 'ctx> {
        match ty {
            Ty::Int | Ty::UInt => LayoutValue::indirect_int(IntSize::Bits64, ptr),
            Ty::Byte | Ty::Bool => LayoutValue::indirect_int(IntSize::Bits8, ptr),
            Ty::Float => LayoutValue::indirect_float(ptr),
            Ty::Char => todo!("Strings"),
            Ty::Tuple(_) => LayoutValue::Tuple(ty, ptr),
            Ty::Array(elem_ty) => LayoutValue::indirect_array(elem_ty, ptr),
            Ty::Func(params, ret_ty) => {
                let func_ty = self.func_ty(params, ret_ty, true);
                LayoutValue::Closure(func_ty, ptr)
            }
            Ty::Named(id) => LayoutValue::Record(*id, ptr),
        }
    }

    fn is_trivial(&self, ty: &Ty) -> bool {
        match ty {
            Ty::Int | Ty::UInt | Ty::Byte | Ty::Float | Ty::Char | Ty::Bool => true,
            Ty::Array(_) | Ty::Func(_, _) => false,
            Ty::Tuple(tys) => tys.iter().all(|ty| self.is_trivial(ty)),
            Ty::Named(id) => (&self.hir.ty_info(*id).fields)
                .into_iter()
                .all(|(_, ty)| self.is_trivial(ty)),
        }
    }

    /// # Panics
    /// Panics if the builder is not positioned, or is positioned but not within a function.
    fn emit_alloca_entry(&self, ty: BasicTypeEnum<'ctx>, name: &str) -> PointerValue<'ctx> {
        let curr_block = self.curr_block();
        let head_block = self
            .curr_function()
            .get_first_basic_block()
            .expect("function has at least one block; we got this function via a block");

        if let Some(first_instr) = head_block.get_first_instruction() {
            self.builder.position_before(&first_instr);
        } else {
            self.builder.position_at_end(head_block);
        }

        let alloc = self.builder.build_alloca(ty, name).unwrap();

        self.builder.position_at_end(curr_block);

        alloc
    }

    fn emit_drop(&self, value: LayoutValue<'hir, 'ctx>) {
        match value {
            LayoutValue::Scalar(ScalarKind::Int(_), _)
            | LayoutValue::Scalar(ScalarKind::Float, _)
            | LayoutValue::Scalar(ScalarKind::FuncPtr(_), _)
            | LayoutValue::Zst => return, // Trivial types
            LayoutValue::Scalar(ScalarKind::Array(elem_ty), ScalarLayout::Direct(ptr)) => {
                self.builder
                    .build_call(self.array_drop(elem_ty), &[ptr.into()], "")
                    .unwrap();
            }
            LayoutValue::Scalar(ScalarKind::Array(elem_ty), ScalarLayout::Indirect(ptr)) => {
                let ptr = self.builder.build_load(self.array_ty(), ptr, "").unwrap();
                self.builder
                    .build_call(self.array_drop(elem_ty), &[ptr.into()], "")
                    .unwrap();
            }
            LayoutValue::Closure(_, ptr) => {
                self.builder
                    .build_call(self.closure_drop(), &[ptr.into()], "")
                    .unwrap();
            }
            LayoutValue::Record(id, ptr) => {
                self.builder
                    .build_call(self.record_drop(id), &[ptr.into()], "")
                    .unwrap();
            }
            LayoutValue::Tuple(ty, ptr) => {
                self.builder
                    .build_call(self.tuple_drop(ty), &[ptr.into()], "")
                    .unwrap();
            }
        }
    }

    fn emit_dup(&self, value: LayoutValue<'hir, 'ctx>) -> LayoutValue<'hir, 'ctx> {
        match value {
            LayoutValue::Scalar(ScalarKind::Int(_), ScalarLayout::Direct(_))
            | LayoutValue::Scalar(ScalarKind::Float, ScalarLayout::Direct(_))
            | LayoutValue::Scalar(ScalarKind::FuncPtr(_), _) => value, // Trivial types
            LayoutValue::Scalar(ScalarKind::Int(size), ScalarLayout::Indirect(ptr)) => {
                let ty = match size {
                    IntSize::Bits8 => self.ctx.i8_type(),
                    IntSize::Bits64 => self.ctx.i64_type(),
                };
                LayoutValue::int(size, self.builder.build_load(ty, ptr, "").unwrap())
            }
            LayoutValue::Scalar(ScalarKind::Float, ScalarLayout::Indirect(ptr)) => {
                LayoutValue::float(
                    self.builder
                        .build_load(self.ctx.f64_type(), ptr, "")
                        .unwrap(),
                )
            }
            LayoutValue::Scalar(ScalarKind::Array(_), ScalarLayout::Direct(array)) => {
                self.builder
                    .build_call(self.array_incr_refc(), &[array.into()], "")
                    .unwrap();
                value
            }
            LayoutValue::Scalar(ScalarKind::Array(elem_ty), ScalarLayout::Indirect(ptr)) => {
                let array = self.builder.build_load(self.array_ty(), ptr, "").unwrap();
                self.builder
                    .build_call(self.array_incr_refc(), &[array.into()], "")
                    .unwrap();
                LayoutValue::array(elem_ty, array)
            }
            LayoutValue::Closure(func_ty, ptr) => {
                let new_ptr = self.emit_alloca_entry(self.closure_ty(), "");
                self.builder
                    .build_call(self.closure_copy(), &[new_ptr.into(), ptr.into()], "")
                    .unwrap();
                LayoutValue::Closure(func_ty, new_ptr)
            }
            LayoutValue::Record(id, ptr) => {
                let new_ptr = self.emit_alloca_entry(self.lower_ty(&Ty::Named(id)), "");
                self.builder
                    .build_call(self.record_copy(id), &[new_ptr.into(), ptr.into()], "")
                    .unwrap();
                LayoutValue::Record(id, new_ptr)
            }
            LayoutValue::Tuple(ty, ptr) => {
                let new_ptr = self.emit_alloca_entry(self.lower_ty(ty), "");
                self.builder
                    .build_call(self.tuple_copy(ty), &[new_ptr.into(), ptr.into()], "")
                    .unwrap();
                LayoutValue::Tuple(ty, new_ptr)
            }
            LayoutValue::Zst => LayoutValue::Zst,
        }
    }

    fn emit_copy(&self, value: LayoutValue<'hir, 'ctx>, dst: LayoutValue<'hir, 'ctx>) {
        if value == LayoutValue::Zst {
            return;
        }
        let dst = dst.as_pointer();
        match value {
            LayoutValue::Scalar(ScalarKind::Int(_), ScalarLayout::Direct(value))
            | LayoutValue::Scalar(ScalarKind::Float, ScalarLayout::Direct(value))
            | LayoutValue::Scalar(ScalarKind::FuncPtr(_), ScalarLayout::Direct(value)) => {
                self.builder.build_store(dst, value).unwrap();
            }
            LayoutValue::Scalar(ScalarKind::Int(size), ScalarLayout::Indirect(ptr)) => {
                let ty = match size {
                    IntSize::Bits8 => self.ctx.i8_type(),
                    IntSize::Bits64 => self.ctx.i64_type(),
                };
                let int = self.builder.build_load(ty, ptr, "").unwrap();
                self.builder.build_store(dst, int).unwrap();
            }
            LayoutValue::Scalar(ScalarKind::Float, ScalarLayout::Indirect(ptr)) => {
                let float = self
                    .builder
                    .build_load(self.ctx.f64_type(), ptr, "")
                    .unwrap();
                self.builder.build_store(dst, float).unwrap();
            }
            LayoutValue::Scalar(ScalarKind::Array(_), ScalarLayout::Direct(array)) => {
                self.builder
                    .build_call(self.array_incr_refc(), &[array.into()], "")
                    .unwrap();
                self.builder.build_store(dst, array).unwrap();
            }
            LayoutValue::Scalar(ScalarKind::Array(_), ScalarLayout::Indirect(ptr)) => {
                let array = self.builder.build_load(self.array_ty(), ptr, "").unwrap();
                self.builder
                    .build_call(self.array_incr_refc(), &[array.into()], "")
                    .unwrap();
                self.builder.build_store(dst, array).unwrap();
            }
            LayoutValue::Scalar(ScalarKind::FuncPtr(_), ScalarLayout::Indirect(ptr)) => {
                let func = self.builder.build_load(self.ptr_ty(), ptr, "").unwrap();
                self.builder.build_store(dst, func).unwrap();
            }
            LayoutValue::Closure(_, ptr) => {
                self.builder
                    .build_call(self.closure_copy(), &[dst.into(), ptr.into()], "")
                    .unwrap();
            }
            LayoutValue::Record(id, ptr) => {
                self.builder
                    .build_call(self.record_copy(id), &[dst.into(), ptr.into()], "")
                    .unwrap();
            }
            LayoutValue::Tuple(ty, ptr) => {
                self.builder
                    .build_call(self.tuple_copy(ty), &[dst.into(), ptr.into()], "")
                    .unwrap();
            }
            LayoutValue::Zst => {}
        }
    }

    fn emit_equals(
        &self,
        lhs: LayoutValue<'hir, 'ctx>,
        rhs: LayoutValue<'hir, 'ctx>,
    ) -> IntValue<'ctx> {
        match (lhs, rhs) {
            (
                LayoutValue::Scalar(ScalarKind::Int(_), ScalarLayout::Direct(lhs)),
                LayoutValue::Scalar(ScalarKind::Int(_), ScalarLayout::Direct(rhs)),
            ) => self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    lhs.into_int_value(),
                    rhs.into_int_value(),
                    "",
                )
                .unwrap(),
            (
                LayoutValue::Scalar(ScalarKind::Int(size), ScalarLayout::Indirect(lhs)),
                LayoutValue::Scalar(ScalarKind::Int(_), ScalarLayout::Indirect(rhs)),
            ) => {
                let ty = match size {
                    IntSize::Bits64 => self.ctx.i64_type(),
                    IntSize::Bits8 => self.ctx.i8_type(),
                };
                let lhs = self.builder.build_load(ty, lhs, "").unwrap();
                let rhs = self.builder.build_load(ty, rhs, "").unwrap();
                self.builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        lhs.into_int_value(),
                        rhs.into_int_value(),
                        "",
                    )
                    .unwrap()
            }
            (
                LayoutValue::Scalar(ScalarKind::Float, ScalarLayout::Direct(lhs)),
                LayoutValue::Scalar(ScalarKind::Float, ScalarLayout::Direct(rhs)),
            ) => self
                .builder
                .build_float_compare(
                    FloatPredicate::OEQ,
                    lhs.into_float_value(),
                    rhs.into_float_value(),
                    "",
                )
                .unwrap(),
            (
                LayoutValue::Scalar(ScalarKind::Float, ScalarLayout::Indirect(lhs)),
                LayoutValue::Scalar(ScalarKind::Float, ScalarLayout::Indirect(rhs)),
            ) => {
                let ty = self.ctx.f64_type();
                let lhs = self.builder.build_load(ty, lhs, "").unwrap();
                let rhs = self.builder.build_load(ty, rhs, "").unwrap();
                self.builder
                    .build_float_compare(
                        FloatPredicate::OEQ,
                        lhs.into_float_value(),
                        rhs.into_float_value(),
                        "",
                    )
                    .unwrap()
            }
            (
                LayoutValue::Scalar(ScalarKind::Array(elem_ty), ScalarLayout::Direct(lhs)),
                LayoutValue::Scalar(ScalarKind::Array(_), ScalarLayout::Direct(rhs)),
            ) => self
                .builder
                .build_call(self.array_equals(elem_ty), &[lhs.into(), rhs.into()], "")
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic()
                .into_int_value(),
            (
                LayoutValue::Scalar(ScalarKind::Array(elem_ty), ScalarLayout::Indirect(lhs)),
                LayoutValue::Scalar(ScalarKind::Array(_), ScalarLayout::Indirect(rhs)),
            ) => {
                let ty = self.array_ty();
                let lhs = self.builder.build_load(ty, lhs, "").unwrap();
                let rhs = self.builder.build_load(ty, rhs, "").unwrap();
                self.builder
                    .build_call(self.array_equals(elem_ty), &[lhs.into(), rhs.into()], "")
                    .unwrap()
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_int_value()
            }
            (
                LayoutValue::Scalar(ScalarKind::FuncPtr(_), ScalarLayout::Direct(lhs)),
                LayoutValue::Scalar(ScalarKind::FuncPtr(_), ScalarLayout::Direct(rhs)),
            ) => self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    lhs.into_pointer_value(),
                    rhs.into_pointer_value(),
                    "",
                )
                .unwrap(),
            (
                LayoutValue::Scalar(ScalarKind::FuncPtr(_), ScalarLayout::Indirect(lhs)),
                LayoutValue::Scalar(ScalarKind::FuncPtr(_), ScalarLayout::Indirect(rhs)),
            ) => {
                let lhs = self.builder.build_load(self.ptr_ty(), lhs, "").unwrap();
                let rhs = self.builder.build_load(self.ptr_ty(), rhs, "").unwrap();
                self.builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        lhs.into_pointer_value(),
                        rhs.into_pointer_value(),
                        "",
                    )
                    .unwrap()
            }
            (LayoutValue::Closure(_, lhs), LayoutValue::Closure(_, rhs)) => self
                .builder
                .build_call(self.closure_equals(), &[lhs.into(), rhs.into()], "")
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic()
                .into_int_value(),
            (LayoutValue::Record(id, lhs), LayoutValue::Record(_, rhs)) => self
                .builder
                .build_call(self.record_equals(id), &[lhs.into(), rhs.into()], "")
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic()
                .into_int_value(),
            (LayoutValue::Tuple(ty, lhs), LayoutValue::Tuple(_, rhs)) => self
                .builder
                .build_call(self.tuple_equals(ty), &[lhs.into(), rhs.into()], "equals")
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic()
                .into_int_value(),
            (LayoutValue::Zst, LayoutValue::Zst) => self.const_bool(true),
            _ => unreachable!("mismatched lhs and rhs types"),
        }
    }

    fn emit_panic(&self, msg: &str) {
        let msg = self
            .builder
            .build_global_string_ptr(msg, "")
            .unwrap()
            .as_pointer_value();
        self.builder
            .build_call(self.panic(), &[msg.into()], "")
            .unwrap();
        self.builder.build_unreachable().unwrap();
    }

    fn emit_move(&self, value: LayoutValue<'hir, 'ctx>, to: LayoutValue<'hir, 'ctx>) {
        self.emit_copy(value, to);
        self.emit_drop(value);
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
    /// Panics if the builder is not positioned.
    fn curr_block(&self) -> BasicBlock<'ctx> {
        self.builder
            .get_insert_block()
            .expect("builder has been positioned")
    }

    /// # Panics
    /// Panics if the builder is not positioned, or is positioned but not within a function.
    fn curr_function(&self) -> FunctionValue<'ctx> {
        self.curr_block()
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
                "T{}E",
                tys.iter().map(|ty| self.mangle_ty(ty)).collect::<String>()
            ),
            Ty::Array(elem_ty) => self.mangle_array_ty(elem_ty),
            Ty::Func(params, ret_ty) => {
                let param_names = params.iter().fold(String::new(), |mut s, p| {
                    let prefix = if p.mutable { "M" } else { "P" };
                    let _ = write!(s, "{prefix}{}", self.mangle_ty(&p.ty));
                    s
                });
                format!("F{param_names}R{}", self.mangle_ty(ret_ty))
            }
            Ty::Named(id) => Self::mangle_name(self.hir.ty_ident(*id).ident.to_string()),
        }
    }

    fn mangle_array_ty(&self, elem_ty: &Ty) -> String {
        format!("A{}", self.mangle_ty(elem_ty))
    }
}
