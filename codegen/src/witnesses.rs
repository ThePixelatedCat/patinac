#![allow(
    clippy::unwrap_used,
    reason = "A large number of Inkwell functions return Results for error conditions we don't want to recover from"
)]

use std::fmt::Write as _;

use inkwell::{
    FloatPredicate, IntPredicate,
    module::Linkage,
    types::{BasicType as _, FunctionType, StructType},
    values::{BasicValue as _, BasicValueEnum, FunctionValue, PointerValue},
};

use hir::{VarId, items::AdtId, types::Ty};

use crate::Codegen;

impl<'ctx> Codegen<'_, '_, 'ctx> {
    pub(crate) fn drop_fn_ty(&self) -> FunctionType<'ctx> {
        self.ctx.void_type().fn_type(&[self.ptr_ty().into()], false)
    }

    pub(crate) fn copy_fn_ty(&self) -> FunctionType<'ctx> {
        let ptr = self.ptr_ty().into();
        self.ctx.void_type().fn_type(&[ptr, ptr], false)
    }

    pub(crate) fn equals_fn_ty(&self) -> FunctionType<'ctx> {
        let ptr = self.ptr_ty().into();
        self.ctx.bool_type().fn_type(&[ptr, ptr], false)
    }

    pub(crate) fn emit_drop(&self, ty: &Ty, val: BasicValueEnum<'ctx>) {
        let func = match ty {
            Ty::Int | Ty::UInt | Ty::Byte | Ty::Float | Ty::Bool => return, // Trivial types
            Ty::Char => todo!("Strings"),
            Ty::Tuple(inner_tys) => {
                // If it's empty, it's unit and therefore trivial + direct
                if inner_tys.is_empty() {
                    return;
                }
                self.tuple_drop(ty, inner_tys)
            }
            Ty::Array(inner_ty) => self.array_drop(inner_ty),
            Ty::Fn(_, _) => self.closure_drop(),
            Ty::Adt(id) => self.struct_drop(*id),
        };
        self.builder
            .build_call(func, &[val.into()], "drop")
            .unwrap();
    }

    pub(crate) fn emit_copy(&self, ty: &Ty, val: BasicValueEnum<'ctx>, dst: PointerValue<'ctx>) {
        let func = match ty {
            Ty::Int | Ty::UInt | Ty::Byte | Ty::Float | Ty::Bool => {
                self.builder.build_store(dst, val).unwrap();
                return;
            }
            Ty::Char => todo!("Strings"),
            Ty::Tuple(inner_tys) => {
                // If it's empty, it's unit and therefore trivial + direct
                if inner_tys.is_empty() {
                    self.builder.build_store(dst, val).unwrap();
                    return;
                }
                self.tuple_copy(ty, inner_tys)
            }
            Ty::Array(inner_ty) => self.array_copy(inner_ty),
            Ty::Fn(..) => self.closure_copy(),
            Ty::Adt(id) => self.struct_copy(*id),
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
                    self.array_equals(inner_ty),
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
            Ty::Adt(id) => self
                .builder
                .build_call(self.struct_equals(*id), &[lhs.into(), rhs.into()], "equals")
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic(),
        }
    }

    pub(crate) fn int_equals(&self, ty: &Ty) -> FunctionValue<'ctx> {
        let func_name = format!("{}.equals", self.mangle(ty));

        // Check if we already built this function
        if let Some(func) = self.module.get_function(&func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end.
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the equality function
        let func =
            self.module
                .add_function(&func_name, self.equals_fn_ty(), Some(Linkage::Private));
        self.builder
            .position_at_end(self.ctx.append_basic_block(func, "entry"));
        let ty = self.lower_ty(ty);
        let lhs = func.get_nth_param(0).unwrap().into_pointer_value();
        let lhs = self.builder.build_load(ty, lhs, "lhs").unwrap();
        let rhs = func.get_nth_param(1).unwrap().into_pointer_value();
        let rhs = self.builder.build_load(ty, rhs, "rhs").unwrap();
        let equals = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                lhs.into_int_value(),
                rhs.into_int_value(),
                "equals",
            )
            .unwrap()
            .as_basic_value_enum();
        self.builder.build_return(Some(&equals)).unwrap();

        self.builder.position_at_end(old_insert_block);

        func
    }

    pub(crate) fn float_equals(&self) -> FunctionValue<'ctx> {
        let func_name = format!("{}.equals", self.mangle(&Ty::Float));

        // Check if we already built this function
        if let Some(func) = self.module.get_function(&func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end.
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the equality function
        let func =
            self.module
                .add_function(&func_name, self.equals_fn_ty(), Some(Linkage::Private));
        self.builder
            .position_at_end(self.ctx.append_basic_block(func, "entry"));
        let ty = self.ctx.f64_type();
        let lhs = func.get_nth_param(0).unwrap().into_pointer_value();
        let lhs = self.builder.build_load(ty, lhs, "lhs").unwrap();
        let rhs = func.get_nth_param(1).unwrap().into_pointer_value();
        let rhs = self.builder.build_load(ty, rhs, "rhs").unwrap();
        let equals = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                lhs.into_int_value(),
                rhs.into_int_value(),
                "equals",
            )
            .unwrap()
            .as_basic_value_enum();
        self.builder.build_return(Some(&equals)).unwrap();

        self.builder.position_at_end(old_insert_block);

        func
    }

    pub(crate) fn tuple_drop(&self, ty: &Ty, inner_tys: &[Ty]) -> FunctionValue<'ctx> {
        self.fields_drop(ty, inner_tys)
    }

    pub(crate) fn tuple_copy(&self, ty: &Ty, inner_tys: &[Ty]) -> FunctionValue<'ctx> {
        self.fields_copy(ty, inner_tys)
    }

    pub(crate) fn tuple_equals(&self, ty: &Ty, inner_tys: &[Ty]) -> FunctionValue<'ctx> {
        self.fields_equals(ty, inner_tys)
    }

    pub(crate) fn struct_drop(&self, id: AdtId) -> FunctionValue<'ctx> {
        self.fields_drop(&Ty::Adt(id), self.hir.adt_info(id).fields.tys())
    }

    pub(crate) fn struct_copy(&self, id: AdtId) -> FunctionValue<'ctx> {
        self.fields_copy(&Ty::Adt(id), self.hir.adt_info(id).fields.tys())
    }

    pub(crate) fn struct_equals(&self, id: AdtId) -> FunctionValue<'ctx> {
        self.fields_equals(&Ty::Adt(id), self.hir.adt_info(id).fields.tys())
    }

    fn fields_drop<'fields>(
        &self,
        ty: &Ty,
        fields: impl IntoIterator<Item = &'fields Ty>,
    ) -> FunctionValue<'ctx> {
        let func_name = format!("{}.drop", self.mangle(ty));

        // Check if we already built this function
        if let Some(func) = self.module.get_function(&func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the drop function
        let func = self
            .module
            .add_function(&func_name, self.drop_fn_ty(), Some(Linkage::Private));
        self.builder
            .position_at_end(self.ctx.append_basic_block(func, "entry"));
        let lowered_ty = self.lower_ty(ty);
        let out = func.get_nth_param(0).unwrap().into_pointer_value();
        // Drop each non-trivial field
        for (idx, field) in fields
            .into_iter()
            .enumerate()
            .filter(|(_, ty)| !self.is_trivial(ty))
        {
            let field_ptr = self
                .builder
                .build_struct_gep(lowered_ty, out, u32::try_from(idx).unwrap(), "fieldptr")
                .unwrap();
            self.emit_drop(field, field_ptr.as_basic_value_enum());
        }
        self.builder.build_return(None).unwrap();

        self.builder.position_at_end(old_insert_block);

        func
    }

    fn fields_copy<'fields>(
        &self,
        ty: &Ty,
        fields: impl IntoIterator<Item = &'fields Ty>,
    ) -> FunctionValue<'ctx> {
        let func_name = format!("{}.copy", self.mangle(ty));

        // Check if we already built this function
        if let Some(func) = self.module.get_function(&func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end.
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the copy function
        let func = self
            .module
            .add_function(&func_name, self.copy_fn_ty(), Some(Linkage::Private));
        self.builder
            .position_at_end(self.ctx.append_basic_block(func, "entry"));
        let lowered_ty = self.lower_ty(ty);
        let dst = func.get_nth_param(0).unwrap().into_pointer_value();
        let src = func.get_nth_param(1).unwrap().into_pointer_value();
        if self.is_trivial(ty) {
            let size = lowered_ty.size_of().unwrap();
            let align = self.target.get_target_data().get_abi_alignment(&lowered_ty);
            self.builder
                .build_memcpy(dst, align, src, align, size)
                .unwrap();
        } else {
            // If the struct/tuple is not trivial, we need to copy each field individually
            for (idx, field) in fields.into_iter().enumerate() {
                let idx = u32::try_from(idx).unwrap();

                let dst = self
                    .builder
                    .build_struct_gep(lowered_ty, dst, idx, "dstfieldptr")
                    .unwrap();
                let src = self
                    .builder
                    .build_struct_gep(lowered_ty, src, idx, "srcfieldptr")
                    .unwrap();

                let val = if Self::is_indirect(field) {
                    src.as_basic_value_enum()
                } else {
                    self.builder
                        .build_load(self.lower_ty(field), src, "srcfield")
                        .unwrap()
                        .as_basic_value_enum()
                };

                self.emit_copy(field, val, dst);
            }
        }
        self.builder.build_return(None).unwrap();

        self.builder.position_at_end(old_insert_block);

        func
    }

    fn fields_equals<'fields>(
        &self,
        ty: &Ty,
        fields: impl IntoIterator<Item = &'fields Ty>,
    ) -> FunctionValue<'ctx> {
        let func_name = format!("{}.equals", self.mangle(ty));

        // Check if we already built this function
        if let Some(func) = self.module.get_function(&func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end.
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the equality function
        let func =
            self.module
                .add_function(&func_name, self.equals_fn_ty(), Some(Linkage::Private));
        self.builder
            .position_at_end(self.ctx.append_basic_block(func, "entry"));
        let lowered_ty = self.lower_ty(ty);
        let lhs = func.get_nth_param(0).unwrap().into_pointer_value();
        let rhs = func.get_nth_param(1).unwrap().into_pointer_value();
        let ne_block = self.ctx.append_basic_block(func, "ne");
        for (idx, field) in fields.into_iter().enumerate() {
            let idx = u32::try_from(idx).unwrap();

            let lhs = self
                .builder
                .build_struct_gep(lowered_ty, lhs, idx, "lhsfieldptr")
                .unwrap();
            let rhs = self
                .builder
                .build_struct_gep(lowered_ty, rhs, idx, "rhsfieldptr")
                .unwrap();

            let (lhs, rhs) = if Self::is_indirect(field) {
                (lhs.as_basic_value_enum(), rhs.as_basic_value_enum())
            } else {
                let field_ty = self.lower_ty(field);
                let lhs = self
                    .builder
                    .build_load(field_ty, lhs, "lhsfield")
                    .unwrap()
                    .as_basic_value_enum();
                let rhs = self
                    .builder
                    .build_load(field_ty, rhs, "rhsfield")
                    .unwrap()
                    .as_basic_value_enum();
                (lhs, rhs)
            };

            // If the fields are equal, continue to a new block for the next comparison, else branch to the not-equal block
            let eq_block = self.ctx.append_basic_block(func, "eq");
            let equals = self.emit_equals(field, lhs, rhs).into_int_value();
            self.builder
                .build_conditional_branch(equals, eq_block, ne_block)
                .unwrap();
            self.builder.position_at_end(eq_block);
        }

        self.builder
            .build_return(Some(&self.ctx.bool_type().const_int(1, false)))
            .unwrap();

        self.builder.position_at_end(ne_block);
        self.builder
            .build_return(Some(&self.ctx.bool_type().const_zero()))
            .unwrap();

        self.builder.position_at_end(old_insert_block);

        func
    }

    pub(crate) fn array_drop(&self, inner_ty: &Ty) -> FunctionValue<'ctx> {
        let func_name = format!("a[{}].drop", self.mangle(inner_ty));

        // Check if we already built this function
        if let Some(func) = self.module.get_function(&func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the drop function
        let func = self
            .module
            .add_function(&func_name, self.drop_fn_ty(), Some(Linkage::Private));
        self.builder
            .position_at_end(self.ctx.append_basic_block(func, "entry"));
        let elem_drop = match inner_ty {
            Ty::Int | Ty::UInt | Ty::Byte | Ty::Float | Ty::Char | Ty::Bool => self.null_ptr(),
            Ty::Tuple(inner_tys) => self
                .tuple_drop(inner_ty, inner_tys)
                .as_global_value()
                .as_pointer_value(),
            Ty::Array(inner_ty) => self
                .array_drop(inner_ty)
                .as_global_value()
                .as_pointer_value(),
            Ty::Fn(..) => self.closure_drop().as_global_value().as_pointer_value(),
            Ty::Adt(id) => self.struct_drop(*id).as_global_value().as_pointer_value(),
        };
        self.builder
            .build_call(
                self.runtime_array_drop(),
                &[
                    func.get_first_param().unwrap().into(),
                    elem_drop.into(),
                    self.lower_ty(inner_ty).size_of().unwrap().into(),
                ],
                "",
            )
            .unwrap();
        self.builder.build_return(None).unwrap();

        self.builder.position_at_end(old_insert_block);

        func
    }

    pub(crate) fn array_copy(&self, inner_ty: &Ty) -> FunctionValue<'ctx> {
        let func_name = format!("a[{}].copy", self.mangle(inner_ty));

        // Check if we already built this function
        if let Some(func) = self.module.get_function(&func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the copy function
        let func = self
            .module
            .add_function(&func_name, self.copy_fn_ty(), Some(Linkage::Private));
        self.builder
            .position_at_end(self.ctx.append_basic_block(func, "entry"));
        let dst = func.get_nth_param(0).unwrap().into();
        let src = func.get_nth_param(1).unwrap().into();
        self.builder
            .build_call(self.runtime_array_copy(), &[dst, src], "")
            .unwrap();
        self.builder.build_return(None).unwrap();

        self.builder.position_at_end(old_insert_block);

        func
    }

    pub(crate) fn array_equals(&self, inner_ty: &Ty) -> FunctionValue<'ctx> {
        let func_name = format!("a[{}].equals", self.mangle(inner_ty));

        // Check if we already built this function
        if let Some(func) = self.module.get_function(&func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the copy function
        let func =
            self.module
                .add_function(&func_name, self.equals_fn_ty(), Some(Linkage::Private));
        self.builder
            .position_at_end(self.ctx.append_basic_block(func, "entry"));
        let lhs = func.get_nth_param(0).unwrap().into();
        let rhs = func.get_nth_param(1).unwrap().into();
        let elem_equals = match inner_ty {
            Ty::Int | Ty::UInt | Ty::Byte | Ty::Bool => self.int_equals(inner_ty),
            Ty::Float => self.float_equals(),
            Ty::Char => todo!("Strings"),
            Ty::Tuple(inner_tys) => self.tuple_drop(inner_ty, inner_tys),
            Ty::Array(inner_ty) => self.array_drop(inner_ty),
            Ty::Fn(..) => self.closure_drop(),
            Ty::Adt(id) => self.struct_drop(*id),
        };
        let result = self
            .builder
            .build_call(
                self.runtime_array_equals(),
                &[
                    lhs,
                    rhs,
                    elem_equals.as_global_value().as_pointer_value().into(),
                    self.lower_ty(inner_ty).size_of().unwrap().into(),
                ],
                "",
            )
            .unwrap()
            .try_as_basic_value()
            .unwrap_basic();
        self.builder.build_return(Some(&result)).unwrap();

        self.builder.position_at_end(old_insert_block);

        func
    }

    pub(crate) fn closure_drop(&self) -> FunctionValue<'ctx> {
        let func_name = "_Closure.drop";

        // Check if we already built this function
        if let Some(func) = self.module.get_function(func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the drop function
        let func = self
            .module
            .add_function(func_name, self.drop_fn_ty(), Some(Linkage::Private));
        self.builder
            .position_at_end(self.ctx.append_basic_block(func, "entry"));
        let val = func.get_first_param().unwrap();
        let drop_func = self
            .builder
            .build_struct_gep(self.closure_ty(), val.into_pointer_value(), 2, "dropfn")
            .unwrap();
        let drop_func = self
            .builder
            .build_load(self.ptr_ty(), drop_func, "dropfn")
            .unwrap();
        self.builder
            .build_indirect_call(
                self.drop_fn_ty(),
                drop_func.into_pointer_value(),
                &[val.into()],
                "",
            )
            .unwrap();
        self.builder.build_return(None).unwrap();

        self.builder.position_at_end(old_insert_block);

        func
    }

    pub(crate) fn closure_copy(&self) -> FunctionValue<'ctx> {
        let func_name = "_Closure.copy";

        // Check if we already built this function
        if let Some(func) = self.module.get_function(func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the drop function
        let func = self
            .module
            .add_function(func_name, self.copy_fn_ty(), Some(Linkage::Private));
        self.builder
            .position_at_end(self.ctx.append_basic_block(func, "entry"));
        let dst = func.get_nth_param(0).unwrap();
        let src = func.get_nth_param(1).unwrap();
        let copy_func = self
            .builder
            .build_struct_gep(self.closure_ty(), src.into_pointer_value(), 3, "copyfn")
            .unwrap();
        let copy_func = self
            .builder
            .build_load(self.ptr_ty(), copy_func, "copyfn")
            .unwrap();
        self.builder
            .build_indirect_call(
                self.copy_fn_ty(),
                copy_func.into_pointer_value(),
                &[dst.into(), src.into()],
                "",
            )
            .unwrap();
        self.builder.build_return(None).unwrap();

        self.builder.position_at_end(old_insert_block);

        func
    }

    pub(crate) fn closure_equals(&self) -> FunctionValue<'ctx> {
        let func_name = "_Closure.equals";

        // Check if we already built this function
        if let Some(func) = self.module.get_function(func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the drop function
        let func = self
            .module
            .add_function(func_name, self.drop_fn_ty(), Some(Linkage::Private));
        self.builder
            .position_at_end(self.ctx.append_basic_block(func, "entry"));
        let lhs = func.get_nth_param(0).unwrap();
        let rhs = func.get_nth_param(1).unwrap();
        let equals_func = self
            .builder
            .build_struct_gep(self.closure_ty(), lhs.into_pointer_value(), 4, "equalsfn")
            .unwrap();
        let equals_func = self
            .builder
            .build_load(self.ptr_ty(), equals_func, "equalsfn")
            .unwrap();
        self.builder
            .build_indirect_call(
                self.equals_fn_ty(),
                equals_func.into_pointer_value(),
                &[lhs.into(), rhs.into()],
                "",
            )
            .unwrap();
        self.builder.build_return(None).unwrap();

        self.builder.position_at_end(old_insert_block);

        func
    }

    pub(crate) fn emit_closure_drop(
        &self,
        name: &str,
        captures: &[VarId],
        env_ty: Option<StructType<'ctx>>,
    ) -> FunctionValue<'ctx> {
        let func_name = format!("{name}.drop");

        // Check if we already built this function
        if let Some(func) = self.module.get_function(&func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the drop function
        let func = self
            .module
            .add_function(&func_name, self.drop_fn_ty(), Some(Linkage::Private));
        self.builder
            .position_at_end(self.ctx.append_basic_block(func, "entry"));
        // Don't need to do anything if there's no captures
        if let Some(env_ty) = env_ty {
            let closure = func.get_first_param().unwrap().into_pointer_value();
            let ty = self.closure_ty();

            // Get the environment.
            let env_ptr = self
                .builder
                .build_struct_gep(ty, closure, 1, "envptr")
                .unwrap();
            let env = self
                .builder
                .build_load(self.ptr_ty(), env_ptr, "env")
                .unwrap()
                .into_pointer_value();

            // Drop each non-trivial capture
            for (idx, ty) in captures.iter().enumerate().filter_map(|(idx, id)| {
                let ty = self.hir.var_ty(*id);
                (!self.is_trivial(ty)).then_some((idx, ty))
            }) {
                let capture_ptr = self
                    .builder
                    .build_struct_gep(env_ty, env, u32::try_from(idx).unwrap(), "captureptr")
                    .unwrap();
                self.emit_drop(ty, capture_ptr.as_basic_value_enum());
            }

            // Free the environment's memory
            self.builder
                .build_call(self.free(), &[env.as_basic_value_enum().into()], "free")
                .unwrap();
        }
        self.builder.build_return(None).unwrap();

        self.builder.position_at_end(old_insert_block);

        func
    }

    pub(crate) fn emit_closure_copy(
        &self,
        name: &str,
        captures: &[VarId],
        env_ty: Option<StructType<'ctx>>,
    ) -> FunctionValue<'ctx> {
        let func_name = format!("{name}.copy");

        // Check if we already built this function
        if let Some(func) = self.module.get_function(&func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the copy function
        let func = self
            .module
            .add_function(&func_name, self.copy_fn_ty(), Some(Linkage::Private));
        self.builder
            .position_at_end(self.ctx.append_basic_block(func, "entry"));
        // Copy the source into the target.
        let dst = func.get_nth_param(0).unwrap().into_pointer_value();
        let src = func.get_nth_param(1).unwrap().into_pointer_value();
        let ty = self.closure_ty();
        let align = self.target.get_target_data().get_abi_alignment(&ty);
        self.builder
            .build_memcpy(dst, align, src, align, ty.size_of().unwrap())
            .unwrap();
        // Don't need to clone the environment if there isn't one
        if let Some(env_ty) = env_ty {
            // Allocate the new target environment
            let size = env_ty.size_of().unwrap();
            let dst_env = self
                .builder
                .build_call(
                    self.malloc(),
                    &[size.as_basic_value_enum().into()],
                    "malloc",
                )
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic()
                .into_pointer_value();

            // Get the source environment
            let src_env_ptr = self
                .builder
                .build_struct_gep(ty, src, 1, "srcenvptr")
                .unwrap();
            let src_env = self
                .builder
                .build_load(self.ptr_ty(), src_env_ptr, "srcenv")
                .unwrap()
                .into_pointer_value();

            if captures
                .iter()
                .all(|id| self.is_trivial(self.hir.var_ty(*id)))
            {
                // If all of the captures are trivial, we can memcpy the whole environment
                let align = self.target.get_target_data().get_abi_alignment(&env_ty);
                self.builder
                    .build_memcpy(dst_env, align, src_env, align, size)
                    .unwrap();
            } else {
                // If some of the captures aren't trivial, we need to copy each of them individually
                for (idx, ty) in captures.iter().map(|id| self.hir.var_ty(*id)).enumerate() {
                    let idx = u32::try_from(idx).unwrap();
                    let dst_capture = self
                        .builder
                        .build_struct_gep(env_ty, dst_env, idx, "dstcapture")
                        .unwrap();
                    let src_capture = self
                        .builder
                        .build_struct_gep(env_ty, src_env, idx, "srccapture")
                        .unwrap();
                    let src_capture = if Self::is_indirect(ty) {
                        src_capture.as_basic_value_enum()
                    } else {
                        self.builder
                            .build_load(self.lower_ty(ty), src_capture, "srccapture")
                            .unwrap()
                    };
                    self.emit_copy(ty, src_capture, dst_capture);
                }
            }

            // Store the target environment
            let dst_env_ptr = self
                .builder
                .build_struct_gep(ty, dst, 1, "dstenvptr")
                .unwrap();
            self.builder.build_store(dst_env_ptr, dst_env).unwrap();
        }
        self.builder.build_return(None).unwrap();

        self.builder.position_at_end(old_insert_block);

        func
    }

    pub(crate) fn emit_closure_equals(
        &self,
        name: &str,
        captures: &[VarId],
        env_ty: Option<StructType<'ctx>>,
    ) -> FunctionValue<'ctx> {
        let func_name = format!("{name}.equals");

        // Check if we already built this function
        if let Some(func) = self.module.get_function(&func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the equality function
        let func =
            self.module
                .add_function(&func_name, self.equals_fn_ty(), Some(Linkage::Private));
        self.builder
            .position_at_end(self.ctx.append_basic_block(func, "entry"));
        let lhs = func.get_nth_param(0).unwrap().into_pointer_value();
        let rhs = func.get_nth_param(1).unwrap().into_pointer_value();
        let ty = self.closure_ty();
        // Test whether the lifted closure pointers are equal.
        let lhs_fn = self.builder.build_struct_gep(ty, lhs, 0, "lhsfn").unwrap();
        let lhs_fn = self
            .builder
            .build_load(self.ptr_ty(), lhs_fn, "lhsfn")
            .unwrap()
            .into_pointer_value();
        let rhs_fn = self.builder.build_struct_gep(ty, rhs, 0, "rhsfn").unwrap();
        let rhs_fn = self
            .builder
            .build_load(self.ptr_ty(), rhs_fn, "rhsfn")
            .unwrap()
            .into_pointer_value();
        let test = self
            .builder
            .build_int_compare(IntPredicate::EQ, lhs_fn, rhs_fn, "fntest")
            .unwrap();
        // If there's no env, we can just return whether the pointers are equal
        match env_ty {
            None => {
                self.builder.build_return(Some(&test)).unwrap();
            }
            Some(env_ty) => {
                let ne_block = self.ctx.append_basic_block(func, "ne");
                let eq_block = self.ctx.append_basic_block(func, "eq");
                self.builder
                    .build_conditional_branch(test, eq_block, ne_block)
                    .unwrap();
                self.builder.position_at_end(eq_block);

                let lhs_env = self.builder.build_struct_gep(ty, lhs, 1, "lhsenv").unwrap();
                let lhs_env = self
                    .builder
                    .build_load(self.ptr_ty(), lhs_env, "lhs_env")
                    .unwrap()
                    .into_pointer_value();
                let rhs_env = self.builder.build_struct_gep(ty, rhs, 1, "rhsenv").unwrap();
                let rhs_env = self
                    .builder
                    .build_load(self.ptr_ty(), rhs_env, "rhs_env")
                    .unwrap()
                    .into_pointer_value();

                for (idx, ty) in captures.iter().map(|id| self.hir.var_ty(*id)).enumerate() {
                    let idx = u32::try_from(idx).unwrap();
                    let lhs_capture = self
                        .builder
                        .build_struct_gep(env_ty, lhs_env, idx, "lhscapture")
                        .unwrap();
                    let rhs_capture = self
                        .builder
                        .build_struct_gep(env_ty, rhs_env, idx, "rhscapture")
                        .unwrap();
                    let (lhs_capture, rhs_capture) = if Self::is_indirect(ty) {
                        (
                            lhs_capture.as_basic_value_enum(),
                            lhs_capture.as_basic_value_enum(),
                        )
                    } else {
                        let ty = self.lower_ty(ty);
                        (
                            self.builder
                                .build_load(ty, lhs_capture, "lhscapture")
                                .unwrap(),
                            self.builder
                                .build_load(ty, rhs_capture, "rhscapture")
                                .unwrap(),
                        )
                    };
                    // Bail out if we found a difference.
                    let eq_block = self.ctx.append_basic_block(func, "eq");
                    let test = self
                        .emit_equals(ty, lhs_capture, rhs_capture)
                        .into_int_value();
                    self.builder
                        .build_conditional_branch(test, eq_block, ne_block)
                        .unwrap();
                    self.builder.position_at_end(eq_block);
                }
                self.builder
                    .build_return(Some(&self.ctx.bool_type().const_int(1, false)))
                    .unwrap();
                self.builder.position_at_end(ne_block);
                self.builder
                    .build_return(Some(&self.ctx.bool_type().const_zero()))
                    .unwrap();
            }
        }

        self.builder.position_at_end(old_insert_block);

        func
    }

    fn mangle(&self, ty: &Ty) -> String {
        match ty {
            Ty::Int => "i".to_string(),
            Ty::UInt => "u".to_string(),
            Ty::Byte => "h".to_string(),
            Ty::Float => "f".to_string(),
            Ty::Char => "c".to_string(),
            Ty::Bool => "b".to_string(),
            Ty::Tuple(tys) => format!(
                "t[{}]",
                tys.iter().map(|ty| self.mangle(ty)).collect::<String>()
            ),
            Ty::Array(ty) => format!("a[{}]", self.mangle(ty)),
            Ty::Fn(params, ret_ty) => {
                let param_names = params.iter().fold(String::new(), |mut s, p| {
                    let mut_str = if p.mutable { "m" } else { "" };
                    let _ = write!(s, "{mut_str}{}", self.mangle(&p.ty));
                    s
                });
                format!("f[{param_names};{}]", self.mangle(ret_ty))
            }
            Ty::Adt(id) => {
                let mut name = self.hir.adt_ident(*id).ident.to_string();
                name.insert(0, '_');
                name
            }
        }
    }
}
