use hir::{VarId, items::AdtId, types::Ty};
use inkwell::{
    FloatPredicate, IntPredicate,
    module::Linkage,
    types::{BasicType, BasicTypeEnum, FunctionType, StructType},
    values::{BasicValue, BasicValueEnum, FunctionValue, PointerValue},
};
use itertools::Itertools;

use crate::Codegen;

impl<'ctx> Codegen<'ctx, '_> {
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

    // pub(crate) fn witness_ty(&self) -> StructType<'ctx> {
    //     if let Some(ty) = self.module.get_struct_type("_Witness") {
    //         return ty;
    //     }

    //     let ty = self.ctx.opaque_struct_type("_Witness");
    //     ty.set_body(
    //         &[
    //             // Size
    //             self.ctx.i64_type().as_basic_type_enum(),
    //             // Drop
    //             self.ptr_ty(),
    //             // Copy
    //             self.ptr_ty(),
    //             // Eq
    //             self.ptr_ty(),
    //         ],
    //         false,
    //     );
    //     ty
    // }

    pub(crate) fn emit_drop(&self, ty: &Ty, val: BasicValueEnum<'ctx>) {
        match ty {
            Ty::Int | Ty::UInt | Ty::Byte | Ty::Float | Ty::Bool => {} // Trivial types
            Ty::Char => todo!("Strings"),
            Ty::Tuple(_) => {
                if !self.is_trivial(ty) {
                    self.builder
                        .build_call(self.tuple_drop(ty), &[val.into()], "drop")
                        .unwrap();
                }
            }
            Ty::Array(_) => todo!("Arrays"),
            Ty::Fn(_, _) => {
                let drop_func_ptr = self
                    .builder
                    .build_struct_gep(self.closure_ty(), val.into_pointer_value(), 2, "dropfn")
                    .unwrap();
                let drop_func = self
                    .builder
                    .build_load(self.ptr_ty(), drop_func_ptr, "dropfn")
                    .unwrap();
                self.builder
                    .build_indirect_call(
                        self.drop_fn_ty(),
                        drop_func.into_pointer_value(),
                        &[val.into()],
                        "drop",
                    )
                    .unwrap();
            }
            Ty::Adt(id) => {
                self.builder
                    .build_call(self.struct_drop(*id), &[val.into()], "drop")
                    .unwrap();
            }
        }
    }

    pub(crate) fn emit_copy(&self, ty: &Ty, val: BasicValueEnum<'ctx>, dst: PointerValue<'ctx>) {
        match ty {
            Ty::Int | Ty::UInt | Ty::Byte | Ty::Float | Ty::Bool => {
                self.builder.build_store(dst, val).unwrap();
            }
            Ty::Char => todo!(),
            Ty::Tuple(tys) => {
                // If it's empty, it's unit and therefore trivial + direct
                if tys.is_empty() {
                    self.builder.build_store(dst, val).unwrap();
                } else {
                    self.builder
                        .build_call(
                            self.tuple_copy(ty),
                            &[dst.as_basic_value_enum().into(), val.into()],
                            "copy",
                        )
                        .unwrap();
                }
            }
            Ty::Array(_) => todo!(),
            Ty::Fn(..) => {
                let copy_func_ptr = self
                    .builder
                    .build_struct_gep(self.closure_ty(), val.into_pointer_value(), 3, "copyfn")
                    .unwrap();
                let copy_func = self
                    .builder
                    .build_load(self.ptr_ty(), copy_func_ptr, "copyfn")
                    .unwrap();
                self.builder
                    .build_indirect_call(
                        self.copy_fn_ty(),
                        copy_func.into_pointer_value(),
                        &[dst.as_basic_value_enum().into(), val.into()],
                        "copy",
                    )
                    .unwrap();
            }
            Ty::Adt(id) => {
                self.builder
                    .build_call(
                        self.struct_copy(*id),
                        &[dst.as_basic_value_enum().into(), val.into()],
                        "copy",
                    )
                    .unwrap();
            }
        }
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
            Ty::Tuple(_) => self
                .builder
                .build_call(self.tuple_equals(ty), &[lhs.into(), rhs.into()], "equals")
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic(),
            Ty::Array(_) => todo!("Array"),
            Ty::Fn(_, _) => {
                let equals_func_ptr = self
                    .builder
                    .build_struct_gep(self.closure_ty(), lhs.into_pointer_value(), 4, "equalsfn")
                    .unwrap();
                let equals_func = self
                    .builder
                    .build_load(self.ptr_ty(), equals_func_ptr, "equalsfn")
                    .unwrap();
                self.builder
                    .build_indirect_call(
                        self.equals_fn_ty(),
                        equals_func.into_pointer_value(),
                        &[lhs.into(), rhs.into()],
                        "equals",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .unwrap_basic()
            }
            Ty::Adt(id) => self
                .builder
                .build_call(self.struct_equals(*id), &[lhs.into(), rhs.into()], "equals")
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic(),
        }
    }

    /// # Panics
    /// Panics if `ty` is not [`Ty::Tuple`]
    pub(crate) fn tuple_drop(&self, ty: &Ty) -> FunctionValue<'ctx> {
        let Ty::Tuple(tys) = ty else { panic!() };
        self.fields_drop(ty, tys)
    }

    /// # Panics
    /// Panics if `ty` is not [`Ty::Tuple`]
    pub(crate) fn tuple_copy(&self, ty: &Ty) -> FunctionValue<'ctx> {
        let Ty::Tuple(tys) = ty else { panic!() };
        self.fields_copy(ty, tys)
    }

    /// # Panics
    /// Panics if `ty` is not [`Ty::Tuple`]
    pub(crate) fn tuple_equals(&self, ty: &Ty) -> FunctionValue<'ctx> {
        let Ty::Tuple(tys) = ty else { panic!() };
        self.fields_equals(ty, tys)
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

    fn fields_drop<'a>(
        &self,
        ty: &Ty,
        fields: impl IntoIterator<Item = &'a Ty>,
    ) -> FunctionValue<'ctx> {
        let func_name = format!("_{}.drop", self.name_of(ty));

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

    fn fields_copy<'a>(
        &self,
        ty: &Ty,
        fields: impl IntoIterator<Item = &'a Ty>,
    ) -> FunctionValue<'ctx> {
        let func_name = format!("_{}.copy", self.name_of(ty));

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

    fn fields_equals<'a>(
        &self,
        ty: &Ty,
        fields: impl IntoIterator<Item = &'a Ty>,
    ) -> FunctionValue<'ctx> {
        let func_name = format!("_{}.equals", self.name_of(ty));

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

    pub(crate) fn closure_drop(
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

    pub(crate) fn closure_copy(
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

    pub(crate) fn closure_equals(
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

    fn name_of(&self, ty: &Ty) -> String {
        match ty {
            Ty::Int => "Int".to_string(),
            Ty::UInt => "UInt".to_string(),
            Ty::Byte => "Byte".to_string(),
            Ty::Float => "Float".to_string(),
            Ty::Char => "Char".to_string(),
            Ty::Bool => "Bool".to_string(),
            Ty::Tuple(tys) => format!("#({})", tys.iter().map(|ty| self.name_of(ty)).join(",")),
            Ty::Array(ty) => todo!(),
            Ty::Fn(params, ty) => todo!(),
            Ty::Adt(id) => self.hir.adt_ident(*id).ident.to_string(),
        }
    }
}
