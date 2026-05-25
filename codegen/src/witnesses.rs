use hir::{items::AdtId, types::Ty};
use inkwell::{
    FloatPredicate, IntPredicate,
    module::Linkage,
    types::{BasicType, FunctionType, StructType},
    values::{BasicValue, BasicValueEnum, FunctionValue, PointerValue},
};

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
            Ty::Char => todo!(),
            Ty::Tuple(_) => todo!(),
            Ty::Array(_) => todo!(),
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
            Ty::Tuple(_) => todo!(),
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
            Ty::Char => todo!(),
            Ty::Tuple(_) => todo!(),
            Ty::Array(_) => todo!(),
            Ty::Fn(_, _) => todo!(),
            Ty::Adt(id) => self
                .builder
                .build_call(self.struct_equals(*id), &[lhs.into(), rhs.into()], "equals")
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic(),
        }
    }

    pub(crate) fn struct_drop(&self, id: AdtId) -> FunctionValue<'ctx> {
        let struct_name = self.hir.adt_ident(id).ident.to_string();
        let func_name = format!("{struct_name}.drop");

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
        let struct_ptr = func.get_nth_param(0).unwrap().into_pointer_value();
        if !self.is_trivial(&Ty::Adt(id)) {
            // If the struct is not trivial, then we need to drop each non-trivial field individually
            for (idx, field_ty) in self
                .hir
                .adt_info(id)
                .fields
                .tys()
                .enumerate()
                .filter(|(_, ty)| !self.is_trivial(ty))
            {
                let field_ptr = self
                    .builder
                    .build_struct_gep(
                        self.lower_ty(field_ty),
                        struct_ptr,
                        u32::try_from(idx).unwrap(),
                        "fieldptr",
                    )
                    .unwrap();

                self.emit_drop(field_ty, field_ptr.as_basic_value_enum());
            }
        }
        self.builder.build_return(None).unwrap();

        self.builder.position_at_end(old_insert_block);

        func
    }

    pub(crate) fn struct_copy(&self, id: AdtId) -> FunctionValue<'ctx> {
        let struct_name = self.hir.adt_ident(id).ident.to_string();
        let func_name = format!("{struct_name}.copy");

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
        let dst = func.get_nth_param(0).unwrap().into_pointer_value();
        let src = func.get_nth_param(1).unwrap().into_pointer_value();
        if self.is_trivial(&Ty::Adt(id)) {
            let ty = self.structs[id];
            let size = ty.size_of().unwrap();
            let align = self.target.get_target_data().get_abi_alignment(&ty);
            self.builder
                .build_memcpy(dst, align, src, align, size)
                .unwrap();
        } else {
            // If the struct is not trivial, then we need to copy each field individually
            for (idx, field) in self.hir.adt_info(id).fields.tys().enumerate() {
                let ty = self.lower_ty(field);
                let idx = u32::try_from(idx).unwrap();

                let dst = self
                    .builder
                    .build_struct_gep(ty, dst, idx, "dstfieldptr")
                    .unwrap();
                let src = self
                    .builder
                    .build_struct_gep(ty, src, idx, "srcfieldptr")
                    .unwrap();

                let val = if crate::is_indirect(field) {
                    src.as_basic_value_enum()
                } else {
                    self.builder
                        .build_load(ty, src, "srcfield")
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

    pub(crate) fn struct_equals(&self, id: AdtId) -> FunctionValue<'ctx> {
        let struct_name = self.hir.adt_ident(id).ident.to_string();
        let func_name = format!("{struct_name}.equals");

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
        let lhs = func.get_nth_param(0).unwrap().into_pointer_value();
        let rhs = func.get_nth_param(1).unwrap().into_pointer_value();
        let ne_block = self.ctx.append_basic_block(func, "ne");
        for (idx, field) in self.hir.adt_info(id).fields.tys().enumerate() {
            let ty = self.lower_ty(field);
            let idx = u32::try_from(idx).unwrap();

            let lhs = self
                .builder
                .build_struct_gep(ty, lhs, idx, "lhsfieldptr")
                .unwrap();
            let rhs = self
                .builder
                .build_struct_gep(ty, rhs, idx, "rhsfieldptr")
                .unwrap();

            let (lhs, rhs) = if crate::is_indirect(field) {
                (lhs.as_basic_value_enum(), rhs.as_basic_value_enum())
            } else {
                let lhs = self
                    .builder
                    .build_load(ty, lhs, "lhsfield")
                    .unwrap()
                    .as_basic_value_enum();
                let rhs = self
                    .builder
                    .build_load(ty, rhs, "rhsfield")
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
            .build_return(Some(&self.ctx.bool_type().const_all_ones()))
            .unwrap();

        self.builder.position_at_end(ne_block);
        self.builder
            .build_return(Some(&self.ctx.bool_type().const_zero()))
            .unwrap();

        self.builder.position_at_end(old_insert_block);

        func
    }
}
