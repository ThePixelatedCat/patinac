use inkwell::{
    AtomicOrdering, AtomicRMWBinOp, IntPredicate,
    intrinsics::Intrinsic,
    module::Linkage,
    types::{BasicType as _, FunctionType, StructType},
    values::{BasicValue as _, FunctionValue},
};

use hir::{VarId, items::TyId, types::Ty};

use crate::Codegen;

impl<'ctx> Codegen<'_, '_, 'ctx> {
    pub(crate) fn drop_func_ty(&self) -> FunctionType<'ctx> {
        self.ctx.void_type().fn_type(&[self.ptr_ty().into()], false)
    }

    pub(crate) fn copy_func_ty(&self) -> FunctionType<'ctx> {
        let ptr = self.ptr_ty().into();
        self.ctx.void_type().fn_type(&[ptr, ptr], false)
    }

    pub(crate) fn equals_func_ty(&self) -> FunctionType<'ctx> {
        let ptr = self.ptr_ty().into();
        self.ctx.bool_type().fn_type(&[ptr, ptr], false)
    }

    /// Returns the drop witness function for the provided type, or `None` if it is a trivial type.
    pub(crate) fn drop_func(&self, ty: &Ty) -> Option<FunctionValue<'ctx>> {
        match ty {
            Ty::Int | Ty::UInt | Ty::Byte | Ty::Float | Ty::Bool => None,
            Ty::Char => todo!("Strings"),
            Ty::Tuple(elem_tys) => {
                // If it's empty, it's unit and therefore trivial + direct
                if elem_tys.is_empty() {
                    None
                } else {
                    Some(self.tuple_drop(ty, elem_tys))
                }
            }
            Ty::Array(elem_ty) => Some(self.array_drop(ty, elem_ty)),
            Ty::Fn(..) => Some(self.closure_drop()),
            Ty::Named(id) => Some(self.struct_drop(*id)),
        }
    }

    /// Returns the copy witness function for the provided type, or `None` if it is a trivial type.
    pub(crate) fn copy_func(&self, ty: &Ty) -> Option<FunctionValue<'ctx>> {
        match ty {
            Ty::Int | Ty::UInt | Ty::Byte | Ty::Float | Ty::Bool => None,
            Ty::Char => todo!("Strings"),
            Ty::Tuple(elem_tys) => {
                // If it's empty, it's unit and therefore trivial + direct
                if elem_tys.is_empty() {
                    None
                } else {
                    Some(self.tuple_copy(ty, elem_tys))
                }
            }
            Ty::Array(_) => Some(self.array_copy(ty)),
            Ty::Fn(..) => Some(self.closure_copy()),
            Ty::Named(id) => Some(self.struct_copy(*id)),
        }
    }

    /// Returns the equals witness function for the provided type.
    pub(crate) fn equals_func(&self, ty: &Ty) -> FunctionValue<'ctx> {
        match ty {
            Ty::Int | Ty::UInt | Ty::Byte | Ty::Bool => self.int_equals(ty),
            Ty::Float => self.float_equals(),
            Ty::Char => todo!("Strings"),
            Ty::Tuple(elem_tys) => self.tuple_equals(ty, elem_tys),
            Ty::Array(elem_ty) => self.array_equals(ty, elem_ty),
            Ty::Fn(..) => self.closure_equals(),
            Ty::Named(id) => self.struct_equals(*id),
        }
    }

    pub(crate) fn int_equals(&self, ty: &Ty) -> FunctionValue<'ctx> {
        let func_name = format!("{}.equals", self.mangle_ty(ty));

        // Check if we already built this function
        if let Some(func) = self.module.get_function(&func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end.
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the equality function
        let func =
            self.module
                .add_function(&func_name, self.equals_func_ty(), Some(Linkage::Private));
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
        let func_name = format!("{}.equals", self.mangle_ty(&Ty::Float));

        // Check if we already built this function
        if let Some(func) = self.module.get_function(&func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end.
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the equality function
        let func =
            self.module
                .add_function(&func_name, self.equals_func_ty(), Some(Linkage::Private));
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

    pub(crate) fn struct_drop(&self, id: TyId) -> FunctionValue<'ctx> {
        self.fields_drop(&Ty::Named(id), self.hir.ty_info(id).fields.tys())
    }

    pub(crate) fn struct_copy(&self, id: TyId) -> FunctionValue<'ctx> {
        self.fields_copy(&Ty::Named(id), self.hir.ty_info(id).fields.tys())
    }

    pub(crate) fn struct_equals(&self, id: TyId) -> FunctionValue<'ctx> {
        self.fields_equals(&Ty::Named(id), self.hir.ty_info(id).fields.tys())
    }

    fn fields_drop<'fields>(
        &self,
        ty: &Ty,
        fields: impl IntoIterator<Item = &'fields Ty>,
    ) -> FunctionValue<'ctx> {
        let func_name = format!("{}.drop", self.mangle_ty(ty));

        // Check if we already built this function
        if let Some(func) = self.module.get_function(&func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the drop function
        let func =
            self.module
                .add_function(&func_name, self.drop_func_ty(), Some(Linkage::Private));
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
        let func_name = format!("{}.copy", self.mangle_ty(ty));

        // Check if we already built this function
        if let Some(func) = self.module.get_function(&func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end.
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the copy function
        let func =
            self.module
                .add_function(&func_name, self.copy_func_ty(), Some(Linkage::Private));
        self.builder
            .position_at_end(self.ctx.append_basic_block(func, "entry"));
        let lowered_ty = self.lower_ty(ty);
        let dst = func.get_nth_param(0).unwrap().into_pointer_value();
        let src = func.get_nth_param(1).unwrap().into_pointer_value();
        if self.is_trivial(ty) {
            self.emit_memcpy(dst, src, &lowered_ty);
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
        let func_name = format!("{}.equals", self.mangle_ty(ty));

        // Check if we already built this function
        if let Some(func) = self.module.get_function(&func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end.
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the equality function
        let func =
            self.module
                .add_function(&func_name, self.equals_func_ty(), Some(Linkage::Private));
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

    pub(crate) fn array_drop(&self, ty: &Ty, elem_ty: &Ty) -> FunctionValue<'ctx> {
        let func_name = format!("{}.drop", self.mangle_ty(ty));

        // Check if we already built this function.
        if let Some(func) = self.module.get_function(&func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end.
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the function and blocks, and extract the arguments.
        let func =
            self.module
                .add_function(&func_name, self.drop_func_ty(), Some(Linkage::Private));
        let entry_block = self.ctx.append_basic_block(func, "entry");
        let decr_block = self.ctx.append_basic_block(func, "decr");
        let drop_block = self.ctx.append_basic_block(func, "drop");
        let loop_block = self.ctx.append_basic_block(func, "loop");
        let free_block = self.ctx.append_basic_block(func, "free");
        let ret_block = self.ctx.append_basic_block(func, "return");
        let array = func.get_first_param().unwrap().into_pointer_value();

        // Return immediately if the array hasn't been allocated.
        let header = {
            self.builder.position_at_end(entry_block);
            let header = self.get_array_header(array);
            let is_null = self
                .builder
                .build_int_compare(IntPredicate::EQ, header, self.null_ptr(), "")
                .unwrap();
            self.builder
                .build_conditional_branch(is_null, ret_block, decr_block)
                .unwrap();
            header
        };

        // Decrement refcount and branch to drop block if it hits 0 and the element type needs to be dropped.
        // We rely on LLVM to optimise out the drop block if it isn't needed for the element type.
        {
            self.builder.position_at_end(decr_block);
            let refc = self
                .builder
                .build_struct_gep(self.array_header_ty(), header, 0, "refc")
                .unwrap();
            let old_refc = self
                .builder
                .build_atomicrmw(
                    AtomicRMWBinOp::Sub,
                    refc,
                    self.ctx.i64_type().const_int(1, false),
                    AtomicOrdering::AcquireRelease,
                )
                .unwrap();
            let no_refs = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    old_refc,
                    self.ctx.i64_type().const_int(1, false),
                    "",
                )
                .unwrap();

            let target_block = if self.is_trivial(elem_ty) {
                free_block
            } else {
                drop_block
            };
            self.builder
                .build_conditional_branch(no_refs, target_block, ret_block)
                .unwrap();
        }

        // Initialise the loop to drop all the elements.
        let (count, index, payload) = {
            self.builder.position_at_end(drop_block);
            let count = self
                .builder
                .build_struct_gep(self.array_header_ty(), header, 1, "count")
                .unwrap();
            let count = self
                .builder
                .build_load(self.ctx.i64_type(), count, "")
                .unwrap()
                .into_int_value();
            let index = self.emit_alloca_entry(self.ctx.i64_type().as_basic_type_enum(), "index");
            let payload = self.get_array_payload(array);
            let empty = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    count,
                    self.ctx.i64_type().const_zero(),
                    "",
                )
                .unwrap();
            self.builder
                .build_conditional_branch(empty, free_block, loop_block)
                .unwrap();
            (count, index, payload)
        };

        // Loop over each element and free it.
        {
            self.builder.position_at_end(loop_block);
            let curr_index = self
                .builder
                .build_load(self.ctx.i64_type(), index, "")
                .unwrap()
                .into_int_value();
            let elem = unsafe {
                self.builder
                    .build_in_bounds_gep(self.lower_ty(elem_ty), payload, &[curr_index], "")
                    .unwrap()
            };
            self.emit_drop(elem_ty, elem.as_basic_value_enum());
            let new_index = self
                .builder
                .build_int_add(curr_index, self.ctx.i64_type().const_int(1, false), "")
                .unwrap();
            self.builder.build_store(index, new_index).unwrap();
            let done = self
                .builder
                .build_int_compare(IntPredicate::UGE, new_index, count, "")
                .unwrap();
            self.builder
                .build_conditional_branch(done, free_block, loop_block)
                .unwrap();
        }

        // Free the memory allocation.
        {
            self.builder.position_at_end(free_block);
            self.builder
                .build_call(self.free(), &[header.into()], "")
                .unwrap();
            let payload_ptr = self
                .builder
                .build_struct_gep(self.array_ty(), array, 0, "")
                .unwrap();
            self.builder
                .build_store(payload_ptr, self.null_ptr())
                .unwrap();
            self.builder.build_unconditional_branch(ret_block).unwrap();
        }

        // Return
        {
            self.builder.position_at_end(ret_block);
            self.builder.build_return(None).unwrap();
        }

        self.builder.position_at_end(old_insert_block);

        func
    }

    pub(crate) fn array_copy(&self, ty: &Ty) -> FunctionValue<'ctx> {
        let func_name = format!("{}.copy", self.mangle_ty(ty));

        // Check if we already built this function
        if let Some(func) = self.module.get_function(&func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the copy function
        let func =
            self.module
                .add_function(&func_name, self.copy_func_ty(), Some(Linkage::Private));
        let entry_block = self.ctx.append_basic_block(func, "entry");
        let incr_block = self.ctx.append_basic_block(func, "incr");
        let ret_block = self.ctx.append_basic_block(func, "return");
        let dst = func.get_nth_param(0).unwrap().into_pointer_value();
        let src = func.get_nth_param(1).unwrap().into_pointer_value();

        self.builder.position_at_end(entry_block);
        let array_ty = self.array_ty();
        self.emit_memcpy(dst, src, &array_ty);
        let header = self.get_array_header(src);
        let is_null = self
            .builder
            .build_int_compare(IntPredicate::EQ, header, self.null_ptr(), "nullchk")
            .unwrap();
        self.builder
            .build_conditional_branch(is_null, ret_block, incr_block)
            .unwrap();

        self.builder.position_at_end(incr_block);
        let refc = self
            .builder
            .build_struct_gep(self.array_header_ty(), header, 0, "refc")
            .unwrap();
        self.builder
            .build_atomicrmw(
                AtomicRMWBinOp::Add,
                refc,
                self.ctx.i64_type().const_int(1, false),
                AtomicOrdering::Monotonic,
            )
            .unwrap();
        self.builder.build_unconditional_branch(ret_block).unwrap();

        self.builder.position_at_end(ret_block);
        self.builder.build_return(None).unwrap();

        self.builder.position_at_end(old_insert_block);

        func
    }

    pub(crate) fn array_equals(&self, ty: &Ty, elem_ty: &Ty) -> FunctionValue<'ctx> {
        let func_name = format!("{}.equals", self.mangle_ty(ty));

        // Check if we already built this function
        if let Some(func) = self.module.get_function(&func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the copy function
        let func =
            self.module
                .add_function(&func_name, self.equals_func_ty(), Some(Linkage::Private));
        self.builder
            .position_at_end(self.ctx.append_basic_block(func, "entry"));
        let lhs = func.get_nth_param(0).unwrap().into();
        let rhs = func.get_nth_param(1).unwrap().into();
        let result = self
            .builder
            .build_call(
                self.runtime_array_equals(),
                &[
                    lhs,
                    rhs,
                    self.equals_func(elem_ty)
                        .as_global_value()
                        .as_pointer_value()
                        .into(),
                    self.lower_ty(elem_ty).size_of().unwrap().into(),
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

    pub(crate) fn array_init(&self, ty: &Ty, elem_ty: &Ty) -> FunctionValue<'ctx> {
        let func_name = format!("{}.init", self.mangle_ty(ty));

        // Check if we already built this function
        if let Some(func) = self.module.get_function(&func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the init function
        let func_ty = self
            .ctx
            .void_type()
            .fn_type(&[self.ptr_ty().into(), self.ctx.i64_type().into()], false);
        let func = self
            .module
            .add_function(&func_name, func_ty, Some(Linkage::Private));
        let entry_block = self.ctx.append_basic_block(func, "entry");
        let count_zero_block = self.ctx.append_basic_block(func, "count.zero");
        let count_not_zero_block = self.ctx.append_basic_block(func, "count.not_zero");
        let ret_block = self.ctx.append_basic_block(func, "return");
        let array = func.get_nth_param(0).unwrap().into_pointer_value();
        let count = func.get_nth_param(1).unwrap().into_int_value();

        // Check if the count is zero.
        {
            self.builder.position_at_end(entry_block);
            let cmp = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    count,
                    self.ctx.i64_type().const_zero(),
                    "",
                )
                .unwrap();
            self.builder
                .build_conditional_branch(cmp, count_zero_block, count_not_zero_block)
                .unwrap();
        }

        // If the count is zero, set the payload pointer to null and return.
        {
            self.builder.position_at_end(count_zero_block);
            let payload = self
                .builder
                .build_struct_gep(self.array_ty(), array, 0, "")
                .unwrap();
            self.builder.build_store(payload, self.null_ptr()).unwrap();
            self.builder.build_unconditional_branch(ret_block).unwrap();
        }

        // If the count isn't zero, calculate capacity and allocate space for capacity + header.
        {
            self.builder.position_at_end(count_not_zero_block);
            // Calculate capacity.
            let cap = self
                .builder
                .build_call(self.array_calc_cap(ty, elem_ty), &[count.into()], "")
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic()
                .into_int_value();
            let header_ty = self.array_header_ty();
            // Add room for header.
            let alloc_size = self
                .builder
                .build_int_add(cap, header_ty.size_of().unwrap(), "")
                .unwrap();
            // Allocate.
            let alloc = self
                .builder
                .build_call(self.malloc(), &[alloc_size.into()], "")
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic()
                .into_pointer_value();
            // Store offset pointer to alloc into array out parameter.
            let src_payload = unsafe {
                self.builder
                    .build_in_bounds_gep(
                        header_ty,
                        alloc,
                        &[self.ctx.i64_type().const_int(1, false)],
                        "",
                    )
                    .unwrap()
            };
            let dst_payload = self
                .builder
                .build_struct_gep(self.array_ty(), array, 0, "")
                .unwrap();
            self.builder.build_store(dst_payload, src_payload).unwrap();
            // Initialise each field of the header.
            let refc_ptr = self
                .builder
                .build_struct_gep(header_ty, alloc, 0, "")
                .unwrap();
            self.builder
                .build_store(refc_ptr, self.ctx.i64_type().const_int(1, false))
                .unwrap()
                .set_atomic_ordering(AtomicOrdering::SequentiallyConsistent)
                .unwrap();
            let count_ptr = self
                .builder
                .build_struct_gep(header_ty, alloc, 1, "")
                .unwrap();
            self.builder.build_store(count_ptr, count).unwrap();
            let cap_ptr = self
                .builder
                .build_struct_gep(header_ty, alloc, 2, "")
                .unwrap();
            self.builder.build_store(cap_ptr, cap).unwrap();
            self.builder.build_unconditional_branch(ret_block).unwrap();
        }

        self.builder.position_at_end(ret_block);
        self.builder.build_return(None).unwrap();

        self.builder.position_at_end(old_insert_block);

        func
    }

    pub(crate) fn array_calc_cap(&self, ty: &Ty, elem_ty: &Ty) -> FunctionValue<'ctx> {
        let func_name = format!("{}.calc_cap", self.mangle_ty(ty));

        // Check if we already built this function
        if let Some(func) = self.module.get_function(&func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the function
        let ty = self
            .ctx
            .i64_type()
            .fn_type(&[self.ctx.i64_type().into()], false);
        let func = self
            .module
            .add_function(&func_name, ty, Some(Linkage::Private));
        let entry_block = self.ctx.append_basic_block(func, "entry");
        let count_one_block = self.ctx.append_basic_block(func, "count.one");
        let size_not_one_block = self.ctx.append_basic_block(func, "size.not_one");
        let count_not_one_block = self.ctx.append_basic_block(func, "count.not_one");
        let count_small_block = self.ctx.append_basic_block(func, "count.small");
        let count_other_block = self.ctx.append_basic_block(func, "count.other");
        let count_not_pow2_block = self.ctx.append_basic_block(func, "count.not_pow2");
        let count_pow2_block = self.ctx.append_basic_block(func, "count.pow2");
        let ret_block = self.ctx.append_basic_block(func, "return");
        let count = func.get_nth_param(0).unwrap().into_int_value();
        let size = self.lower_ty(elem_ty).size_of().unwrap();

        // Go to special-case optimisation if the count is 1.
        {
            self.builder.position_at_end(entry_block);
            let cmp = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    count,
                    self.ctx.i64_type().const_int(1, false),
                    "",
                )
                .unwrap();
            self.builder
                .build_conditional_branch(cmp, count_one_block, count_not_one_block)
                .unwrap();
        }

        // Special-case optimisation for 1-element arrays.
        {
            self.builder.position_at_end(count_one_block);
            let cmp = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    size,
                    self.ctx.i64_type().const_int(1, false),
                    "",
                )
                .unwrap();
            // If element is 1 byte, return 8 bytes because the allocator will probably round to that anyway.
            // Otherwise branch further.
            self.builder
                .build_conditional_branch(cmp, ret_block, size_not_one_block)
                .unwrap();
        }

        // Special-case optimisation for 1-element arrays, cont.
        let size_not_one_cap = {
            self.builder.position_at_end(size_not_one_block);
            let cmp = self
                .builder
                .build_int_compare(
                    IntPredicate::ULT,
                    size,
                    self.ctx.i64_type().const_int(1025, false),
                    "",
                )
                .unwrap();
            let med_cap = self
                .builder
                .build_left_shift(size, self.ctx.i64_type().const_int(2, false), "")
                .unwrap();
            // 4 is a good balance for medium-size elements.
            // For >1kb elements, just use 1 to avoid wasting too much memory.
            let cap = self.builder.build_select(cmp, med_cap, size, "").unwrap();
            self.builder.build_unconditional_branch(ret_block).unwrap();
            cap
        };

        // Branch to more special-cases.
        {
            self.builder.position_at_end(count_not_one_block);
            let cmp = self
                .builder
                .build_int_compare(
                    IntPredicate::ULT,
                    count,
                    self.ctx.i64_type().const_int(8, false),
                    "",
                )
                .unwrap();
            self.builder
                .build_conditional_branch(cmp, count_small_block, count_other_block)
                .unwrap();
        }

        // If count is less than 8, round it up to 8 elements.
        let count_small_cap = {
            self.builder.position_at_end(count_small_block);
            let cap = self
                .builder
                .build_left_shift(size, self.ctx.i64_type().const_int(3, false), "")
                .unwrap();
            self.builder.build_unconditional_branch(ret_block).unwrap();
            cap
        };

        // Main path.
        {
            self.builder.position_at_end(count_other_block);
            let ctpop = Intrinsic::find("llvm.ctpop")
                .unwrap()
                .get_declaration(&self.module, &[self.ctx.i64_type().into()])
                .unwrap();
            let bit_count = self
                .builder
                .build_call(ctpop, &[count.into()], "")
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic()
                .into_int_value();
            let is_pow2 = self
                .builder
                .build_int_compare(
                    IntPredicate::ULT,
                    bit_count,
                    self.ctx.i64_type().const_int(2, false),
                    "",
                )
                .unwrap();
            self.builder
                .build_conditional_branch(is_pow2, count_pow2_block, count_not_pow2_block)
                .unwrap();
        }

        // If count isn't a power of 2, round up to one.
        // (https://stackoverflow.com/a/466242)
        let rounded_count = {
            self.builder.position_at_end(count_not_pow2_block);
            let dec = self
                .builder
                .build_int_sub(count, self.ctx.i64_type().const_int(1, false), "")
                .unwrap();
            let shr = self
                .builder
                .build_right_shift(dec, self.ctx.i64_type().const_int(1, false), false, "")
                .unwrap();
            let or = self.builder.build_or(shr, dec, "").unwrap();
            let shr = self
                .builder
                .build_right_shift(or, self.ctx.i64_type().const_int(2, false), false, "")
                .unwrap();
            let or = self.builder.build_or(shr, or, "").unwrap();
            let shr = self
                .builder
                .build_right_shift(or, self.ctx.i64_type().const_int(4, false), false, "")
                .unwrap();
            let or = self.builder.build_or(shr, or, "").unwrap();
            let shr = self
                .builder
                .build_right_shift(or, self.ctx.i64_type().const_int(8, false), false, "")
                .unwrap();
            let or = self.builder.build_or(shr, or, "").unwrap();
            let shr = self
                .builder
                .build_right_shift(or, self.ctx.i64_type().const_int(16, false), false, "")
                .unwrap();
            let or = self.builder.build_or(shr, or, "").unwrap();
            let shr = self
                .builder
                .build_right_shift(or, self.ctx.i64_type().const_int(32, false), false, "")
                .unwrap();
            let or = self.builder.build_or(shr, or, "").unwrap();
            let inc = self
                .builder
                .build_int_add(or, self.ctx.i64_type().const_int(1, false), "")
                .unwrap();
            self.builder
                .build_unconditional_branch(count_pow2_block)
                .unwrap();
            inc
        };

        // Once count is a power of 2, multiply it by the element size to get our capacity.
        let count_other_cap = {
            self.builder.position_at_end(count_pow2_block);
            let phi = self.builder.build_phi(self.ctx.i64_type(), "").unwrap();
            phi.add_incoming(&[
                (&rounded_count, count_not_pow2_block),
                (&count, count_other_block),
            ]);
            let cap = self
                .builder
                .build_int_mul(phi.as_basic_value().into_int_value(), size, "")
                .unwrap();
            self.builder.build_unconditional_branch(ret_block).unwrap();
            cap
        };

        // Return the computed capacity.
        {
            self.builder.position_at_end(ret_block);
            let ret_val = self.builder.build_phi(self.ctx.i64_type(), "").unwrap();
            ret_val.add_incoming(&[
                (&self.ctx.i64_type().const_int(8, false), count_one_block),
                (&size_not_one_cap, size_not_one_block),
                (&count_small_cap, count_small_block),
                (&count_other_cap, count_pow2_block),
            ]);
            self.builder
                .build_return(Some(&ret_val.as_basic_value()))
                .unwrap();
        }

        self.builder.position_at_end(old_insert_block);

        func
    }

    pub(crate) fn array_bounds_check(&self) -> FunctionValue<'ctx> {
        let func_name = "bounds_check";

        // Check if we already built this function
        if let Some(func) = self.module.get_function(func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the function
        let ty = self
            .ctx
            .void_type()
            .fn_type(&[self.ptr_ty().into(), self.ctx.i64_type().into()], false);
        let func = self
            .module
            .add_function(func_name, ty, Some(Linkage::Private));
        let entry_block = self.ctx.append_basic_block(func, "entry");
        let bounds_block = self.ctx.append_basic_block(func, "bounds_chk");
        let panic_block = self.ctx.append_basic_block(func, "panic");
        let ret_block = self.ctx.append_basic_block(func, "return");
        let array = func.get_nth_param(0).unwrap().into_pointer_value();
        let index = func.get_nth_param(1).unwrap().into_int_value();

        self.builder.position_at_end(entry_block);
        let header = self.get_array_header(array);
        let is_null = self
            .builder
            .build_int_compare(IntPredicate::EQ, header, self.null_ptr(), "")
            .unwrap();
        self.builder
            .build_conditional_branch(is_null, panic_block, bounds_block)
            .unwrap();

        self.builder.position_at_end(bounds_block);
        let count = self
            .builder
            .build_struct_gep(self.array_header_ty(), header, 1, "")
            .unwrap();
        let count = self
            .builder
            .build_load(self.ctx.i64_type(), count, "")
            .unwrap()
            .into_int_value();
        let is_in_bounds = self
            .builder
            .build_int_compare(IntPredicate::ULT, index, count, "")
            .unwrap();
        self.builder
            .build_conditional_branch(is_in_bounds, ret_block, panic_block)
            .unwrap();

        self.builder.position_at_end(panic_block);
        let panic_msg = self
            .builder
            .build_global_string_ptr("index out of bounds", "oob_panic_msg")
            .unwrap()
            .as_pointer_value();
        self.builder
            .build_call(self.panic(), &[panic_msg.into()], "")
            .unwrap();
        self.builder.build_unconditional_branch(ret_block).unwrap();

        self.builder.position_at_end(ret_block);
        self.builder.build_return(None).unwrap();

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
            .add_function(func_name, self.drop_func_ty(), Some(Linkage::Private));
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
                self.drop_func_ty(),
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

        // Create the function
        let func = self
            .module
            .add_function(func_name, self.copy_func_ty(), Some(Linkage::Private));
        self.builder
            .position_at_end(self.ctx.append_basic_block(func, "entry"));
        let dst = func.get_nth_param(0).unwrap();
        let src = func.get_nth_param(1).unwrap();
        let copy_func = self
            .builder
            .build_struct_gep(self.closure_ty(), src.into_pointer_value(), 3, "")
            .unwrap();
        let copy_func = self
            .builder
            .build_load(self.ptr_ty(), copy_func, "")
            .unwrap();
        self.builder
            .build_indirect_call(
                self.copy_func_ty(),
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
            .add_function(func_name, self.drop_func_ty(), Some(Linkage::Private));
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
                self.equals_func_ty(),
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
        let func =
            self.module
                .add_function(&func_name, self.drop_func_ty(), Some(Linkage::Private));
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
        let func =
            self.module
                .add_function(&func_name, self.copy_func_ty(), Some(Linkage::Private));
        self.builder
            .position_at_end(self.ctx.append_basic_block(func, "entry"));
        // Copy the source into the target.
        let dst = func.get_nth_param(0).unwrap().into_pointer_value();
        let src = func.get_nth_param(1).unwrap().into_pointer_value();
        let ty = self.closure_ty();
        self.emit_memcpy(dst, src, &ty);
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
                self.emit_memcpy(dst_env, src_env, &env_ty);
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
                .add_function(&func_name, self.equals_func_ty(), Some(Linkage::Private));
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
}
