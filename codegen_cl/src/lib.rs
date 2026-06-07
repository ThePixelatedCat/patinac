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

use std::{cmp::Reverse, fmt::Write as _, fs, iter, path::PathBuf, str::FromStr};

use cranelift::{
    codegen::{Context, ir::StackSlot, isa::CallConv, settings::Flags},
    module::{FuncId, FuncOrDataId, Linkage, Module},
    object::{ObjectBuilder, ObjectModule},
    prelude::*,
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

pub struct Codegen<'hir, 'handler> {
    hir: &'hir Hir,
    ty_map: &'hir TyMap,
    handler: ErrorHandler<'handler>,
    module: ObjectModule,
    funcs: SecondaryMap<VarId, FuncId>,
    vars: SecondaryMap<VarId, VirtualValue>,
    lambda_counter: u32,
}

#[derive(Clone, Copy)]
enum VirtualValue {
    Direct(Value),
    Indirect(Value),
    Variable(Variable),
}

impl VirtualValue {
    fn get_val(self, builder: &mut FunctionBuilder) -> Value {
        match self {
            Self::Indirect(value) => value,
            Self::Direct(value) => value,
            Self::Variable(var) => builder.use_var(var),
        }
    }
}

impl<'hir, 'handler> Codegen<'hir, 'handler> {
    /// Creates a new [`Codegen`] for a package with the given name.
    ///
    /// The context should be obtained via [`create_ctx()`].
    ///
    /// # Panics
    /// Panics if there is an issue initialising the target.
    pub fn new(
        hir: &'hir Hir,
        ty_map: &'hir TyMap,
        handler: ErrorHandler<'handler>,
        package_name: &str,
    ) -> Self {
        // The ISA contains information about our intended target and acts as the settings for cranelift.
        let isa = {
            let mut builder = settings::builder();
            builder.set("opt_level", "speed").unwrap();
            isa::lookup(TARGET_TRIPLE)
                .unwrap()
                .finish(Flags::new(builder))
                .unwrap()
        };
        let builder = ObjectBuilder::new(
            isa.clone(),
            package_name,
            cranelift::module::default_libcall_names(),
        )
        .unwrap();
        let module = ObjectModule::new(builder);

        Self {
            hir,
            ty_map,
            handler,
            module,
            funcs: SecondaryMap::new(),
            vars: SecondaryMap::new(),
            lambda_counter: 0,
        }
    }

    /// # Panics
    /// Panics if any functions are invalid, or if writing to the output file fails.
    pub fn codegen(mut self, opt_level: OptLevel, mode: CodegenMode) {
        let mut ctx = Context::new();
        let mut func_ctx = FunctionBuilderContext::new();

        for (ty, _) in self.hir.tys() {
            self.build_constructor(&mut ctx, &mut func_ctx, ty);
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
            let ExecKind::Fn { body, .. } = main.kind else {
                unreachable!("ICE")
            };

            let sig = Signature {
                call_conv: self.module.isa().default_call_conv(),
                params: vec![],
                returns: vec![AbiParam::new(types::I32)],
            };
            let func = self
                .module
                .declare_function("main", Linkage::Export, &sig)
                .unwrap();
            self.funcs.insert(main.id, func);

            let mut builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
            builder.func.signature = sig;

            let entry_block = builder.create_block();
            builder.switch_to_block(entry_block);
            self.emit_expr(&mut builder, body);
            let exit_code = builder.ins().iconst(types::I32, 0);
            builder.ins().return_(&[exit_code]);

            codegen::verify_function(&builder.func, self.module.isa()).unwrap();
            builder.finalize();
            self.module.define_function(func, &mut ctx).unwrap();
            ctx.clear();
        }

        for exec in self.hir.execs() {
            match &exec.kind {
                ExecKind::Const { .. } => todo!("Constants"),
                ExecKind::Fn { params, body } => {
                    let Ty::Fn(_, ret_ty) = self.hir.var_ty(exec.id) else {
                        unreachable!("ICE")
                    };
                    self.build_func(
                        &mut ctx,
                        &mut func_ctx,
                        self.funcs[exec.id],
                        params,
                        ret_ty,
                        *body,
                    );
                }
            }
        }

        let product = self.module.finish();

        match mode {
            CodegenMode::IRDump => todo!(),
            CodegenMode::Emit(path) => {
                fs::write(path, product.emit().unwrap()).unwrap();
            }
            CodegenMode::Silent => {}
        }
    }

    fn create_func(&mut self, id: VarId) -> FuncId {
        let Ty::Fn(params, ret_ty) = self.hir.var_ty(id) else {
            unreachable!("ICE")
        };
        let name = Self::mangle_name(self.hir.var_info(id).ident.to_string());
        let sig = self.create_signature(params, ret_ty);
        let func = self
            .module
            .declare_function(&name, Linkage::Local, &sig)
            .unwrap();
        self.funcs.insert(id, func);
        // self.vars
        //     .insert(id, func.as_global_value().as_pointer_value());
        func
    }

    fn create_signature(&self, params: &[Param], ret_ty: &Ty) -> Signature {
        let mut params: Vec<_> = params
            .iter()
            .map(|p| {
                let ty = if p.mutable || Self::is_indirect(&p.ty) {
                    self.ptr_ty()
                } else {
                    self.lower_ty(&p.ty)
                };
                AbiParam::new(ty)
            })
            .collect();

        // Add parameter for the environment
        params.push(AbiParam::new(self.ptr_ty()));

        // Return structs by out-pointer
        let returns = if Self::is_indirect(ret_ty) {
            params.insert(0, AbiParam::new(self.ptr_ty()));
            Vec::new()
        } else {
            vec![AbiParam::new(self.lower_ty(ret_ty))]
        };

        Signature {
            call_conv: CallConv::Fast,
            params,
            returns,
        }
    }

    fn get_signature(&self, func: FuncId) -> Signature {
        self.module
            .declarations()
            .get_function_decl(func)
            .signature
            .clone()
    }

    fn build_constructor(
        &mut self,
        ctx: &mut Context,
        func_ctx: &mut FunctionBuilderContext,
        ty: TyId,
    ) {
        let info = self.hir.ty_info(ty);

        let func = self.create_func(info.constructor_id);

        let mut builder = FunctionBuilder::new(&mut ctx.func, func_ctx);
        builder.func.signature = self.get_signature(func);

        // Create the function's entry block.
        let entry_block = builder.create_block();
        builder.switch_to_block(entry_block);
        builder.append_block_params_for_function_params(entry_block);
        builder.seal_block(entry_block);

        let mut params = builder.block_params(entry_block).to_vec().into_iter();
        let out_ptr = params.next().unwrap();
        for (idx, (arg, field_ty)) in iter::zip(params, info.fields.tys()).enumerate() {
            let field_ptr = self.gep_record(&mut builder, ty, out_ptr, idx);
            self.emit_copy(&mut builder, field_ty, arg, field_ptr);
        }

        builder.ins().return_(&[]);

        codegen::verify_function(&builder.func, self.module.isa()).unwrap();
        builder.finalize();
        self.module.define_function(func, ctx).unwrap();
        ctx.clear();
    }

    fn build_func(
        &mut self,
        ctx: &mut Context,
        func_ctx: &mut FunctionBuilderContext,
        func: FuncId,
        params: &[VarId],
        ret_ty: &Ty,
        body: ExprId,
    ) {
        let mut builder = FunctionBuilder::new(&mut ctx.func, func_ctx);
        builder.func.signature = self.get_signature(func);

        // Create the function's entry block.
        let entry_block = builder.create_block();
        builder.switch_to_block(entry_block);
        builder.append_block_params_for_function_params(entry_block);
        builder.seal_block(entry_block);

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
    }

    fn lower_ty(&self, ty: &Ty) -> Type {
        match ty {
            Ty::Int | Ty::UInt => types::I64,
            Ty::Byte | Ty::Bool => types::I8,
            Ty::Float => types::F64,
            Ty::Char => todo!("Strings"),
            Ty::Tuple(elem_tys) => {
                let elem_tys: Vec<_> = elem_tys.iter().map(|ty| self.lower_ty(ty)).collect();
                self.ctx.struct_type(&elem_tys, false).as_basic_type_enum()
            }
            Ty::Array(_) => self.array_ty(),
            Ty::Fn(..) => self.closure_ty(),
            Ty::Named(id) => self.structs[*id].as_basic_type_enum(),
        }
    }

    fn array_ty(&self) -> Type {
        self.ptr_ty()
    }

    fn array_header_ty(&self) -> Type {
        if let Some(ty) = self.module.get_struct_type("ArrayHeader") {
            return ty.as_basic_type_enum();
        }

        let ty = self.ctx.opaque_struct_type("ArrayHeader");
        let i64_ty = self.ctx.i64_type().as_basic_type_enum();
        // Refcount, element count, capacity
        ty.set_body(&[i64_ty, i64_ty, i64_ty], false);
        ty.as_basic_type_enum()
    }

    fn get_array_payload(&self, array: Value) -> PointerValue<'ctx> {
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
        let payload = self.get_array_payload(array);
        let header = self.get_array_header_from_payload(payload);
        let is_null = self
            .builder
            .build_int_compare(IntPredicate::EQ, payload, self.null_ptr(), "")
            .unwrap();
        self.builder
            .build_select(is_null, self.null_ptr(), header, "")
            .unwrap()
            .into_pointer_value()
    }

    fn get_array_header_from_payload(&self, payload: PointerValue<'ctx>) -> PointerValue<'ctx> {
        unsafe {
            self.builder
                .build_in_bounds_gep(
                    self.array_header_ty(),
                    payload,
                    &[self.ctx.i64_type().const_int(1, true).const_neg()],
                    "header",
                )
                .unwrap()
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

    fn ptr_ty(&self) -> Type {
        self.module.isa().pointer_type()
    }

    fn null_ptr(&self) -> Value {
        todo!()
    }

    fn gep_record(
        &self,
        builder: &mut FunctionBuilder,
        ty: TyId,
        base_ptr: Value,
        idx: usize,
    ) -> Value {
        let fields = self.hir.ty_info(ty).fields;
        assert!(idx < fields.len(), "field index out of bounds");
        let offset = fields.tys().map(|ty| self.size_of(ty)).take(idx + 1).sum();
        builder.ins().iadd_imm(base_ptr, i64::from(offset))
    }

    /// Returns the stack size of the given type in bytes, accounting for struct padding.
    fn size_of(&self, ty: &Ty) -> u32 {
        match ty {
            Ty::Int | Ty::UInt => types::I64.bytes(),
            Ty::Byte | Ty::Bool => types::I8.bytes(),
            Ty::Float => types::F64.bytes(),
            Ty::Char => todo!("Strings"),
            Ty::Tuple(elem_tys) => self.size_of_fields(elem_tys),
            Ty::Array(_) => self.array_ty().bytes(),
            Ty::Fn(params, ty) => todo!(),
            Ty::Named(id) => self.size_of_fields(self.hir.ty_info(*id).fields.tys()),
        }
    }

    fn size_of_fields<'ty>(&self, fields: impl IntoIterator<Item = &'ty Ty>) -> u32 {
        // Get sizes of fields.
        let mut fields: Vec<_> = fields
            .into_iter()
            .map(|ty| (ty, self.lower_ty(ty).bytes()))
            .collect();
        // Sort fields in descending order of size to optimise final size.
        fields.sort_by_key(|(_, s)| Reverse(*s));

        let mut total_size = 0;
        for &(ty, size) in &fields {
            total_size += size;

            // Pad each field.
            let align = self.align_of(ty);
            let padding = (align - total_size % align) % align;
            total_size += padding;
        }

        // Pad the overall size.
        let self_align = self.align_of_fields(fields.into_iter().map(|(ty, _)| ty));
        let end_padding = (self_align - total_size % self_align) % self_align;
        total_size + end_padding
    }

    /// Returns the alignment of the given type in bytes.
    fn align_of(&self, ty: &Ty) -> u32 {
        match ty {
            Ty::Int | Ty::UInt | Ty::Byte | Ty::Float | Ty::Bool | Ty::Array(_) => self.size_of(ty),
            Ty::Char => todo!("Strings"),
            Ty::Tuple(elem_tys) => self.align_of_fields(elem_tys),
            Ty::Fn(params, ty) => todo!(),
            Ty::Named(id) => self.align_of_fields(self.hir.ty_info(*id).fields.tys()),
        }
    }

    fn align_of_fields<'ty>(&self, fields: impl IntoIterator<Item = &'ty Ty>) -> u32 {
        fields
            .into_iter()
            .map(|ty| self.align_of(ty))
            .max()
            .unwrap_or(0)
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
            Ty::Int | Ty::UInt | Ty::Byte | Ty::Float | Ty::Char | Ty::Bool | Ty::Array(_) => false,
            Ty::Fn(_, _) | Ty::Named(_) => true,
            Ty::Tuple(inner) => !inner.is_empty(),
        }
    }

    pub(crate) fn emit_drop(&self, builder: &mut FunctionBuilder, ty: &Ty, val: VirtualValue) {
        let func = match ty {
            Ty::Int | Ty::UInt | Ty::Byte | Ty::Float | Ty::Bool => return, // Trivial types
            Ty::Char => todo!("Strings"),
            Ty::Tuple(elem_tys) => {
                // If it's empty, it's unit and therefore trivial + direct
                if elem_tys.is_empty() {
                    return;
                }
                self.tuple_drop(ty, elem_tys)
            }
            Ty::Array(elem_ty) => self.array_drop(ty, elem_ty),
            Ty::Fn(_, _) => self.closure_drop(),
            Ty::Named(id) => self.struct_drop(*id),
        };
        self.builder
            .build_call(func, &[val.into()], "drop")
            .unwrap();
    }

    pub(crate) fn emit_copy(
        &self,
        builder: &mut FunctionBuilder,
        ty: &Ty,
        src: VirtualValue,
        dst: Value,
    ) {
        let func = match ty {
            Ty::Int | Ty::UInt | Ty::Byte | Ty::Float | Ty::Bool => {
                builder
                    .ins()
                    .store(MemFlags::trusted(), src.get_val(builder), dst, 0);
                return;
            }
            Ty::Char => todo!("Strings"),
            Ty::Tuple(elem_tys) => {
                // If it's empty, it's unit and therefore trivial + direct
                if elem_tys.is_empty() {
                    builder
                        .ins()
                        .store(MemFlags::trusted(), src.get_val(builder), dst, 0);
                    return;
                }
                self.tuple_copy(ty, elem_tys)
            }
            Ty::Array(_) => self.array_copy(ty),
            Ty::Fn(..) => self.closure_copy(),
            Ty::Named(id) => self.struct_copy(*id),
        };
        builder
            .ins()
            .call(
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
    ) -> IntValue<'ctx> {
        match ty {
            Ty::Int | Ty::UInt | Ty::Byte | Ty::Bool => self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    lhs.into_int_value(),
                    rhs.into_int_value(),
                    "equals",
                )
                .unwrap(),
            Ty::Float => self
                .builder
                .build_float_compare(
                    FloatPredicate::OEQ,
                    lhs.into_float_value(),
                    rhs.into_float_value(),
                    "equals",
                )
                .unwrap(),
            Ty::Char => todo!("Strings"),
            Ty::Tuple(inner_tys) => {
                // If it's empty, it's unit and therefore always equals
                if inner_tys.is_empty() {
                    self.ctx.bool_type().const_int(1, false)
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
                        .into_int_value()
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
                .unwrap_basic()
                .into_int_value(),
            Ty::Fn(_, _) => self
                .builder
                .build_call(self.closure_equals(), &[lhs.into(), rhs.into()], "equals")
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic()
                .into_int_value(),
            Ty::Named(id) => self
                .builder
                .build_call(self.struct_equals(*id), &[lhs.into(), rhs.into()], "equals")
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic()
                .into_int_value(),
        }
    }

    fn emit_move(&self, builder: &mut FunctionBuilder, ty: &Ty, val: VirtualValue, to: Value) {
        self.emit_copy(builder, ty, val, to);
        self.emit_drop(builder, ty, val);
    }

    /// # Panics
    /// Panics if the provided type is unsized.
    fn emit_memcpy(&self, builder: &mut FunctionBuilder, dst: Value, src: Value, ty: Type) {
        let size = builder.ins().iconst(self.ptr_ty(), i64::from(ty.bytes()));
        builder.call_memcpy(self.module.target_config(), dst, src, size);
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
                "T{}E",
                tys.iter().map(|ty| self.mangle_ty(ty)).collect::<String>()
            ),
            Ty::Array(ty) => format!("A{}", self.mangle_ty(ty)),
            Ty::Fn(params, ret_ty) => {
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

    fn get_func(&self, name: &str) -> Option<FuncId> {
        if let Some(FuncOrDataId::Func(func)) = self.module.declarations().get_name(name) {
            Some(func)
        } else {
            None
        }
    }
}
