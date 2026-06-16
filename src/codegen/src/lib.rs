//! Generates LLVM-IR and emits object files from the [`Mir`].
//!
//! The entry point to this crate is the [`Codegen`] type, and the [`codegen`][Codegen::codegen] method on it.
//! Use the [`create_ctx`] function to acquire a [`Context`] for use in [`Codegen::new`].

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

use std::{fmt::Write as _, path::PathBuf, str::FromStr};

use inkwell::{
    AddressSpace, FloatPredicate, IntPredicate,
    basic_block::BasicBlock,
    builder::Builder,
    context::Context,
    module::Module,
    passes::PassBuilderOptions,
    targets::{FileType, InitializationConfig, Target, TargetMachine, TargetMachineOptions},
    types::{BasicType, BasicTypeEnum, FunctionType},
    values::{BasicValueEnum, FloatValue, FunctionValue, IntValue, PointerValue},
};
use slotmap::SecondaryMap;

use mir::{ExprId, ItemKind, Mir, Param, Ty, VarId};

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

pub struct Codegen<'mir, 'ctx> {
    mir: &'mir Mir,
    ctx: &'ctx Context,
    builder: Builder<'ctx>,
    module: Module<'ctx>,
    target: TargetMachine,
    funcs: SecondaryMap<VarId, FunctionValue<'ctx>>,
    vars: SecondaryMap<VarId, LayoutValue<'mir, 'ctx>>,
    lambda_counter: u32,
}

/// Creates a new [`Context`].
///
/// This is a direct wrapper over [`Context::create()`], and exists so that other crates don't have to depend on Inkwell directly.
pub fn create_ctx() -> Context {
    Context::create()
}

impl<'mir, 'ctx> Codegen<'mir, 'ctx> {
    /// Creates a new [`Codegen`] for a package with the given name.
    ///
    /// The context should be obtained via [`create_ctx()`].
    ///
    /// # Panics
    /// Panics if there is an issue initialising the target.
    pub fn new(mir: &'mir Mir, ctx: &'ctx Context, package_name: &str) -> Self {
        let module = ctx.create_module(package_name);

        Target::initialize_native(&InitializationConfig::default()).unwrap();
        let triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&triple).unwrap();
        let target_machine = target
            .create_target_machine_from_options(&triple, TargetMachineOptions::default())
            .unwrap();

        Self {
            mir,
            ctx,
            builder: ctx.create_builder(),
            module,
            target: target_machine,
            funcs: SecondaryMap::new(),
            vars: SecondaryMap::new(),
            lambda_counter: 0,
        }
    }

    /// # Panics
    /// Panics if any functions are invalid, or if writing to the output file fails.
    pub fn codegen(&mut self, opt_level: OptLevel, mode: CodegenMode) {
        for exec in self.mir.execs() {
            match &exec.kind {
                ItemKind::Const { .. } => todo!("Constants"),
                ItemKind::Func { .. } => {
                    self.create_func(exec.var);
                }
            }
        }

        if let Some(main) = self.mir.main() {
            let fn_ty = self.ctx.i32_type().fn_type(&[], false);
            let func = self.module.add_function("main", fn_ty, None);
            self.funcs.insert(main.var, func);

            let ItemKind::Func { body, .. } = main.kind else {
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

        for exec in self.mir.execs() {
            match &exec.kind {
                ItemKind::Const { .. } => todo!("Constants"),
                ItemKind::Func { params, body } => {
                    let Ty::FuncPtr(_, ret_ty) = &self.mir.var(exec.var).ty else {
                        unreachable!("ICE")
                    };
                    self.build_func(self.funcs[exec.var], params, ret_ty, *body);
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

    fn create_func(&mut self, id: VarId) -> FunctionValue<'ctx> {
        let Ty::FuncPtr(params, ret_ty) = &self.mir.var(id).ty else {
            unreachable!("ICE")
        };
        let name = Self::mangle_name(self.mir.var(id).ident.to_string());
        let func_ty = self.func_ty(params, ret_ty, false);
        let func = self.module.add_function(&name, func_ty, None);
        self.funcs.insert(id, func);
        self.vars.insert(
            id,
            LayoutValue::func_ptr(func_ty, func.as_global_value().as_pointer_value()),
        );
        func
    }

    // fn build_ctor(&mut self, ctor: VarId, ty: TyId) {
    //     let info = self.mir.ty(ty);

    //     let func = self.create_func(ctor);
    //     let entry_block = self.ctx.append_basic_block(func, "entry");
    //     self.builder.position_at_end(entry_block);

    //     let ty = self.fields_ty(&info.fields);
    //     let out_ptr = func.get_first_param().unwrap().into_pointer_value();
    //     for (idx, (arg, field_ty)) in
    //         iter::zip(func.get_param_iter().skip(1), &info.fields).enumerate()
    //     {
    //         let field_ptr = self
    //             .builder
    //             .build_struct_gep(ty, out_ptr, u32::try_from(idx).unwrap(), "")
    //             .unwrap();
    //         self.emit_copy(
    //             self.layout(field_ty, arg),
    //             self.layout_indirect(field_ty, field_ptr),
    //         );
    //     }

    //     self.builder.build_return(None).unwrap();

    //     assert!(func.verify(true));
    // }

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
        let offset = if layout::indirect(ret_ty) { 1 } else { 0 };
        self.bind_params(params.iter().copied(), func.get_param_iter().skip(offset));

        let body = self.emit_expr(body);

        match layout::storage_class(ret_ty) {
            StorageClass::Zst => self.builder.build_return(None).unwrap(),
            StorageClass::Indirect => {
                let out_ptr = func.get_first_param().unwrap().into_pointer_value();
                self.emit_move(body, out_ptr);
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
            let info = self.mir.var(id);
            // We don't actually pass ZSTs.
            if layout::zst(&info.ty) {
                self.vars.insert(id, LayoutValue::Zst);
                continue;
            }
            let value = args.next().expect("there should be enough args");
            let value = if info.mutable {
                self.layout_indirect(&info.ty, value.into_pointer_value())
            } else {
                self.layout(&info.ty, value)
            };
            self.vars.insert(id, value);
        }
    }

    fn lower_ty(&self, ty: &Ty) -> BasicTypeEnum<'ctx> {
        match ty {
            Ty::Int | Ty::UInt => self.ctx.i64_type().as_basic_type_enum(),
            Ty::Byte => self.ctx.i8_type().as_basic_type_enum(),
            Ty::Float => self.ctx.f64_type().as_basic_type_enum(),
            Ty::Bool => self.ctx.bool_type().as_basic_type_enum(),
            Ty::Fields(fields) => self.fields_ty(fields),
            Ty::Array(_) => self.array_ty(),
            Ty::FuncPtr(..) => self.ptr_ty(),
            Ty::Closure(..) => self.closure_ty(),
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
                if layout::zst(&p.ty) {
                    None // Skip passing ZSTs.
                } else if p.mutable || layout::indirect(&p.ty) {
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
        match layout::storage_class(ret_ty) {
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

    fn fields_ty(&self, fields: &[Ty]) -> BasicTypeEnum<'ctx> {
        let field_tys: Vec<_> = fields.iter().map(|ty| self.lower_ty(ty)).collect();
        self.ctx.struct_type(&field_tys, false).as_basic_type_enum()
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

    fn layout(&self, ty: &'mir Ty, value: BasicValueEnum<'ctx>) -> LayoutValue<'mir, 'ctx> {
        match ty {
            Ty::Int | Ty::UInt => LayoutValue::int(IntSize::Bits64, value),
            Ty::Byte | Ty::Bool => LayoutValue::int(IntSize::Bits8, value),
            Ty::Float => LayoutValue::float(value),
            Ty::Array(elem_ty) => LayoutValue::array(elem_ty, value),
            Ty::FuncPtr(params, ret_ty) => LayoutValue::func_ptr(
                self.func_ty(params, ret_ty, true),
                value.into_pointer_value(),
            ),
            Ty::Closure(params, ret_ty) => LayoutValue::Closure(
                self.func_ty(params, ret_ty, true),
                value.into_pointer_value(),
            ),
            Ty::Fields(fields) => LayoutValue::Fields(fields, value.into_pointer_value()),
        }
    }

    fn layout_direct(&self, ty: &'mir Ty, ptr: PointerValue<'ctx>) -> LayoutValue<'mir, 'ctx> {
        match ty {
            Ty::Int | Ty::UInt => {
                let int = self
                    .builder
                    .build_load(self.ctx.i64_type(), ptr, "")
                    .unwrap();
                LayoutValue::int(IntSize::Bits64, int)
            }
            Ty::Byte | Ty::Bool => {
                let int = self
                    .builder
                    .build_load(self.ctx.i8_type(), ptr, "")
                    .unwrap();
                LayoutValue::int(IntSize::Bits8, int)
            }
            Ty::Float => {
                let float = self
                    .builder
                    .build_load(self.ctx.f64_type(), ptr, "")
                    .unwrap();
                LayoutValue::float(float)
            }
            Ty::Array(elem_ty) => {
                let array = self.builder.build_load(self.ptr_ty(), ptr, "").unwrap();
                LayoutValue::array(elem_ty, array)
            }
            Ty::FuncPtr(params, ret_ty) => {
                let func_ptr = self.builder.build_load(self.ptr_ty(), ptr, "").unwrap();
                LayoutValue::func_ptr(
                    self.func_ty(params, ret_ty, true),
                    func_ptr.into_pointer_value(),
                )
            }
            Ty::Closure(params, ret_ty) => {
                LayoutValue::Closure(self.func_ty(params, ret_ty, true), ptr)
            }
            Ty::Fields(fields) => LayoutValue::Fields(fields, ptr),
        }
    }

    fn layout_indirect(&self, ty: &'mir Ty, ptr: PointerValue<'ctx>) -> LayoutValue<'mir, 'ctx> {
        match ty {
            Ty::Int | Ty::UInt => LayoutValue::indirect_int(IntSize::Bits64, ptr),
            Ty::Byte | Ty::Bool => LayoutValue::indirect_int(IntSize::Bits8, ptr),
            Ty::Float => LayoutValue::indirect_float(ptr),
            Ty::Array(elem_ty) => LayoutValue::indirect_array(elem_ty, ptr),
            Ty::FuncPtr(params, ret_ty) => {
                LayoutValue::indirect_func_ptr(self.func_ty(params, ret_ty, true), ptr)
            }
            Ty::Closure(params, ret_ty) => {
                LayoutValue::Closure(self.func_ty(params, ret_ty, true), ptr)
            }
            Ty::Fields(fields) => LayoutValue::Fields(fields, ptr),
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

    fn emit_drop(&self, value: LayoutValue<'mir, 'ctx>) {
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
            LayoutValue::Fields(fields, ptr) => {
                self.builder
                    .build_call(self.fields_drop(fields), &[ptr.into()], "")
                    .unwrap();
            }
        }
    }

    fn emit_dup(&self, value: LayoutValue<'mir, 'ctx>) -> LayoutValue<'mir, 'ctx> {
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
            LayoutValue::Fields(fields, ptr) => {
                let new_ptr = self.emit_alloca_entry(self.fields_ty(fields), "");
                self.builder
                    .build_call(self.fields_copy(fields), &[new_ptr.into(), ptr.into()], "")
                    .unwrap();
                LayoutValue::Fields(fields, new_ptr)
            }
            LayoutValue::Zst => LayoutValue::Zst,
        }
    }

    fn emit_copy(&self, value: LayoutValue<'mir, 'ctx>, dst: PointerValue<'ctx>) {
        if value == LayoutValue::Zst {
            return;
        }
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
            LayoutValue::Fields(fields, ptr) => {
                self.builder
                    .build_call(self.fields_copy(fields), &[dst.into(), ptr.into()], "")
                    .unwrap();
            }
            LayoutValue::Zst => {}
        }
    }

    fn emit_equals(
        &self,
        lhs: LayoutValue<'mir, 'ctx>,
        rhs: LayoutValue<'mir, 'ctx>,
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
            (LayoutValue::Fields(fields, lhs), LayoutValue::Fields(_, rhs)) => self
                .builder
                .build_call(self.fields_equals(fields), &[lhs.into(), rhs.into()], "")
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

    fn emit_move(&self, value: LayoutValue<'mir, 'ctx>, dst: PointerValue<'ctx>) {
        self.emit_copy(value, dst);
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
            Ty::Bool => "b".to_string(),
            Ty::Array(elem_ty) => self.mangle_array_ty(elem_ty),
            Ty::FuncPtr(params, ret_ty) => {
                let param_names = params.iter().fold(String::new(), |mut s, p| {
                    let prefix = if p.mutable { "M" } else { "P" };
                    let _ = write!(s, "{prefix}{}", self.mangle_ty(&p.ty));
                    s
                });
                format!("F{param_names}R{}", self.mangle_ty(ret_ty))
            }
            Ty::Closure(params, ret_ty) => {
                let param_names = params.iter().fold(String::new(), |mut s, p| {
                    let prefix = if p.mutable { "M" } else { "P" };
                    let _ = write!(s, "{prefix}{}", self.mangle_ty(&p.ty));
                    s
                });
                format!("C{param_names}R{}", self.mangle_ty(ret_ty))
            }
            Ty::Fields(fields) => self.mangle_fields_ty(fields),
        }
    }

    fn mangle_fields_ty(&self, fields: &[Ty]) -> String {
        format!(
            "({})",
            fields
                .iter()
                .map(|ty| self.mangle_ty(ty))
                .collect::<String>()
        )
    }

    fn mangle_array_ty(&self, elem_ty: &Ty) -> String {
        format!("A{}", self.mangle_ty(elem_ty))
    }
}
