use hir::{items::AdtId, types::Ty};
use inkwell::{
    FloatPredicate, IntPredicate,
    module::Linkage,
    types::{BasicType, FunctionType, StructType},
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

    pub(crate) fn eq_fn_ty(&self) -> FunctionType<'ctx> {
        let ptr = self.ptr_ty().into();
        self.ctx.bool_type().fn_type(&[ptr, ptr], false)
    }

    pub(crate) fn witness_ty(&self) -> StructType<'ctx> {
        if let Some(ty) = self.module.get_struct_type("_Witness") {
            return ty;
        }

        let ty = self.ctx.opaque_struct_type("_Metatype");
        ty.set_body(
            &[
                // Size
                self.ctx.i64_type().as_basic_type_enum(),
                // Drop
                self.ptr_ty(),
                // Copy
                self.ptr_ty(),
                // Eq
                self.ptr_ty(),
            ],
            false,
        );
        ty
    }

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
            Ty::Fn(_, _) => todo!(),
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
            Ty::Fn(_, _) => todo!(),
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
            Ty::Fn(_, _) => todo!(),
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
        let func_name = format!("{}.drop", self.name_of(ty));

        // Check if we already built this function
        if let Some(func) = self.module.get_function(&func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end.
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the drop function
        let func = self
            .module
            .add_function(&func_name, self.drop_fn_ty(), Some(Linkage::Private));
        self.builder
            .position_at_end(self.ctx.append_basic_block(func, "entry"));
        let lowered_ty = self.lower_ty(ty);
        let out = func.get_nth_param(0).unwrap().into_pointer_value();
        if !self.is_trivial(ty) {
            // If the struct is not trivial, then we need to drop each non-trivial field individually
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
        let func_name = format!("{}.copy", self.name_of(ty));

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

                let val = if crate::is_indirect(field) {
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
        let func_name = format!("{}.equals", self.name_of(ty));

        // Check if we already built this function
        if let Some(func) = self.module.get_function(&func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end.
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the equality function
        let func = self
            .module
            .add_function(&func_name, self.eq_fn_ty(), Some(Linkage::Private));
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

            let (lhs, rhs) = if crate::is_indirect(field) {
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
